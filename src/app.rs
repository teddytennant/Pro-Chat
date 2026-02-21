use ratatui::prelude::*;
use tokio::sync::mpsc;

use crate::api::{ApiClient, Message};
use crate::config::Config;
use crate::event::{Event, EventHandler};
use crate::history::Conversation;
use crate::keybinds::{handle_key, KeyAction};
use crate::neovim::NeovimClient;
use crate::ui;

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    Help,
    History,
    Settings,
}

pub struct App {
    pub config: Config,
    pub input: String,
    pub input_mode: InputMode,
    pub cursor_pos: usize,
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: usize,
    pub streaming: bool,
    pub stream_buffer: String,
    pub command_input: String,
    pub overlay: Overlay,
    pub overlay_scroll: usize,
    pub status_message: Option<String>,
    pub conversation: Conversation,
    pub history_list: Vec<Conversation>,
    pub input_history: Vec<String>,
    pub input_history_idx: Option<usize>,
    pub should_quit: bool,
    pub terminal_height: u16,
    pub neovim: Option<NeovimClient>,
    api_client: ApiClient,
    event_tx: Option<mpsc::UnboundedSender<Event>>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        let neovim = if config.neovim.auto_connect {
            config.neovim.socket_path.as_deref()
                .map(|s| NeovimClient::new(s))
                .or_else(|| NeovimClient::discover().map(|s| NeovimClient::new(&s)))
        } else {
            None
        };

        Self {
            config,
            input: String::new(),
            input_mode: InputMode::Insert, // Start in insert mode for immediate typing
            cursor_pos: 0,
            messages: Vec::new(),
            scroll_offset: 0,
            streaming: false,
            stream_buffer: String::new(),
            command_input: String::new(),
            overlay: Overlay::None,
            overlay_scroll: 0,
            status_message: None,
            conversation: Conversation::new(),
            history_list: Vec::new(),
            input_history: Vec::new(),
            input_history_idx: None,
            should_quit: false,
            terminal_height: 24,
            neovim,
            api_client: ApiClient::new(),
            event_tx: None,
        }
    }

    pub fn set_model(&mut self, model: &str) {
        self.config.model = model.to_string();
    }

    pub fn set_provider(&mut self, provider: &str) {
        self.config.provider = provider.to_string();
    }

    pub fn set_nvim_socket(&mut self, socket: &str) {
        self.neovim = Some(NeovimClient::new(socket));
    }

    pub fn set_input(&mut self, text: &str) {
        self.input = text.to_string();
        self.cursor_pos = self.input.len();
    }

    pub fn load_conversation(&mut self, id: &str) -> anyhow::Result<()> {
        let conv = Conversation::load(id)?;
        self.messages = conv.messages.iter().map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
        }).collect();
        self.conversation = conv;
        self.scroll_to_bottom();
        Ok(())
    }

    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    pub fn visible_height(&self) -> usize {
        self.terminal_height.saturating_sub(6) as usize
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
        mut events: EventHandler,
    ) -> anyhow::Result<()> {
        self.event_tx = Some(events.sender());

        loop {
            // Draw UI
            terminal.draw(|f| {
                self.terminal_height = f.area().height;
                ui::draw(f, self);
            })?;

            // Handle events
            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        // Clear status message on any keypress
                        self.status_message = None;

                        match handle_key(self, key) {
                            KeyAction::Quit => {
                                // Save conversation before quitting
                                if !self.messages.is_empty() {
                                    let _ = self.conversation.save();
                                }
                                return Ok(());
                            }
                            KeyAction::SendMessage => {
                                self.send_message().await?;
                            }
                            KeyAction::CancelStream => {
                                self.cancel_stream();
                            }
                            _ => {}
                        }
                    }
                    Event::ApiChunk(text) => {
                        self.stream_buffer.push_str(&text);
                        // Update the last assistant message
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = self.stream_buffer.clone();
                            }
                        }
                        self.scroll_to_bottom();
                    }
                    Event::ApiDone => {
                        self.streaming = false;
                        // Save to conversation history
                        if !self.stream_buffer.is_empty() {
                            self.conversation.add_message("assistant", &self.stream_buffer);
                            let _ = self.conversation.save();
                        }
                        self.stream_buffer.clear();
                    }
                    Event::ApiError(err) => {
                        self.streaming = false;
                        self.stream_buffer.clear();
                        // Remove the empty assistant message
                        if let Some(last) = self.messages.last() {
                            if last.role == "assistant" && last.content.is_empty() {
                                self.messages.pop();
                            }
                        }
                        self.status_message = Some(format!("Error: {err}"));
                    }
                    Event::Resize(_, h) => {
                        self.terminal_height = h;
                    }
                    Event::Tick => {}
                    Event::Mouse(_) => {}
                }
            }

            if self.should_quit {
                return Ok(());
            }
        }
    }

    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        // Handle slash commands
        if input.starts_with('/') {
            return self.handle_slash_command(&input);
        }

        // Check for API key
        let api_key = match self.config.api_key_from_env() {
            Some(key) => key,
            None => {
                self.status_message = Some(format!(
                    "No API key set. Set {} or add to config: {}",
                    match self.config.provider.as_str() {
                        "openai" => "OPENAI_API_KEY",
                        _ => "ANTHROPIC_API_KEY",
                    },
                    Config::path().display()
                ));
                return Ok(());
            }
        };

        // Add user message
        self.messages.push(ChatMessage {
            role: "user".into(),
            content: input.clone(),
        });
        self.conversation.add_message("user", &input);

        // Save input to history
        self.input_history.push(input);
        self.input_history_idx = None;

        // Clear input
        self.input.clear();
        self.cursor_pos = 0;

        // Add placeholder for assistant
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
        });

        // Start streaming
        self.streaming = true;
        self.stream_buffer.clear();
        self.scroll_to_bottom();

        // Build messages for API
        let api_messages: Vec<Message> = self.messages.iter()
            .filter(|m| !m.content.is_empty())
            .map(|m| Message {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let tx = self.event_tx.clone().unwrap();
        let provider = self.config.provider.clone();
        let model = self.config.model.clone();
        let system = self.config.system_prompt.clone();
        let max_tokens = self.config.max_tokens;
        let temp = self.config.temperature;
        let client = ApiClient::new();

        tokio::spawn(async move {
            let result = match provider.as_str() {
                "openai" => {
                    client.stream_openai(
                        &api_key, &model, &api_messages,
                        system.as_deref(), max_tokens, temp, tx.clone(),
                    ).await
                }
                _ => {
                    client.stream_anthropic(
                        &api_key, &model, &api_messages,
                        system.as_deref(), max_tokens, temp, tx.clone(),
                    ).await
                }
            };

            if let Err(e) = result {
                let _ = tx.send(Event::ApiError(e.to_string()));
            }
        });

        Ok(())
    }

    fn handle_slash_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            "/clear" | "/c" => {
                self.messages.clear();
                self.conversation = Conversation::new();
                self.status_message = Some("Conversation cleared".into());
            }
            "/new" | "/n" => {
                self.new_conversation();
            }
            "/model" | "/m" => {
                if let Some(model) = parts.get(1) {
                    self.config.model = model.to_string();
                    self.status_message = Some(format!("Model set to {model}"));
                } else {
                    self.status_message = Some(format!("Current model: {}", self.config.model));
                }
            }
            "/provider" | "/p" => {
                if let Some(provider) = parts.get(1) {
                    self.config.provider = provider.to_string();
                    self.status_message = Some(format!("Provider set to {provider}"));
                } else {
                    self.status_message = Some(format!("Current provider: {}", self.config.provider));
                }
            }
            "/system" | "/s" => {
                if let Some(prompt) = parts.get(1) {
                    self.config.system_prompt = Some(prompt.to_string());
                    self.status_message = Some("System prompt updated".into());
                } else {
                    self.status_message = Some(
                        self.config.system_prompt.as_deref()
                            .unwrap_or("No system prompt set")
                            .to_string()
                    );
                }
            }
            "/history" | "/h" => {
                self.overlay = Overlay::History;
                self.load_history_list();
            }
            "/help" | "/?" => {
                self.overlay = Overlay::Help;
            }
            "/temp" | "/t" => {
                if let Some(temp) = parts.get(1) {
                    if let Ok(t) = temp.parse::<f32>() {
                        self.config.temperature = t;
                        self.status_message = Some(format!("Temperature set to {t}"));
                    }
                } else {
                    self.status_message = Some(format!("Temperature: {}", self.config.temperature));
                }
            }
            "/save" => {
                self.config.save()?;
                self.status_message = Some("Config saved".into());
            }
            "/nvim" => {
                if let Some(path) = parts.get(1) {
                    self.neovim = Some(NeovimClient::new(path));
                    self.status_message = Some("Neovim connected".into());
                } else if let Some(socket) = NeovimClient::discover() {
                    self.neovim = Some(NeovimClient::new(&socket));
                    self.status_message = Some(format!("Neovim connected: {socket}"));
                } else {
                    self.status_message = Some("No Neovim instance found".into());
                }
            }
            "/quit" | "/q" => {
                self.should_quit = true;
            }
            _ => {
                self.status_message = Some(format!("Unknown command: {}", parts[0]));
            }
        }
        self.input.clear();
        self.cursor_pos = 0;
        Ok(())
    }

    pub fn cancel_stream(&mut self) {
        self.streaming = false;
        if !self.stream_buffer.is_empty() {
            self.conversation.add_message("assistant", &self.stream_buffer);
        }
        self.stream_buffer.clear();
        self.status_message = Some("Stream cancelled".into());
    }

    // Text editing operations
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn insert_newline(&mut self) {
        self.input.insert(self.cursor_pos, '\n');
        self.cursor_pos += 1;
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let word_start = trimmed.rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input = format!("{}{}", &self.input[..word_start], &self.input[self.cursor_pos..]);
        self.cursor_pos = word_start;
    }

    pub fn delete_to_start(&mut self) {
        self.input = self.input[self.cursor_pos..].to_string();
        self.cursor_pos = 0;
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
    }

    pub fn cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn cursor_home(&mut self) {
        // Go to start of current line
        let before = &self.input[..self.cursor_pos];
        self.cursor_pos = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    }

    pub fn cursor_end(&mut self) {
        // Go to end of current line
        let after = &self.input[self.cursor_pos..];
        self.cursor_pos += after.find('\n').unwrap_or(after.len());
    }

    pub fn cursor_word_forward(&mut self) {
        let after = &self.input[self.cursor_pos..];
        // Skip current word chars, then skip whitespace
        let skip_word = after.find(|c: char| c.is_whitespace()).unwrap_or(after.len());
        let rest = &after[skip_word..];
        let skip_space = rest.find(|c: char| !c.is_whitespace()).unwrap_or(rest.len());
        self.cursor_pos += skip_word + skip_space;
    }

    pub fn cursor_word_back(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        self.cursor_pos = trimmed.rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    // Scroll operations
    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX; // Will be clamped by renderer
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    // Clipboard
    pub fn paste_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                for c in text.chars() {
                    self.insert_char(c);
                }
            }
        }
    }

    pub fn yank_last_response(&mut self) {
        if let Some(last) = self.messages.iter().rev().find(|m| m.role == "assistant") {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&last.content);
                self.status_message = Some("Response copied to clipboard".into());
            }
        }
    }

    // History navigation
    pub fn history_prev(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let idx = match self.input_history_idx {
            Some(i) => i.saturating_sub(1),
            None => self.input_history.len() - 1,
        };
        self.input_history_idx = Some(idx);
        self.input = self.input_history[idx].clone();
        self.cursor_pos = self.input.len();
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.input_history_idx {
            if idx + 1 < self.input_history.len() {
                self.input_history_idx = Some(idx + 1);
                self.input = self.input_history[idx + 1].clone();
                self.cursor_pos = self.input.len();
            } else {
                self.input_history_idx = None;
                self.input.clear();
                self.cursor_pos = 0;
            }
        }
    }

    pub fn tab_complete(&mut self) {
        if !self.input.starts_with('/') {
            return;
        }
        let commands = [
            "/clear", "/new", "/model", "/provider", "/system",
            "/history", "/help", "/temp", "/save", "/nvim", "/quit",
        ];
        let matches: Vec<&&str> = commands.iter()
            .filter(|c| c.starts_with(&self.input))
            .collect();
        if matches.len() == 1 {
            self.input = format!("{} ", matches[0]);
            self.cursor_pos = self.input.len();
        } else if !matches.is_empty() {
            self.status_message = Some(
                matches.iter().map(|m| m.to_string()).collect::<Vec<_>>().join("  ")
            );
        }
    }

    // Overlay operations
    pub fn overlay_scroll_down(&mut self) {
        self.overlay_scroll = self.overlay_scroll.saturating_add(1);
    }

    pub fn overlay_scroll_up(&mut self) {
        self.overlay_scroll = self.overlay_scroll.saturating_sub(1);
    }

    pub fn overlay_select(&mut self) {
        match self.overlay {
            Overlay::History => {
                if let Some(conv) = self.history_list.get(self.overlay_scroll) {
                    let id = conv.id.clone();
                    let _ = self.load_conversation(&id);
                    self.overlay = Overlay::None;
                    self.overlay_scroll = 0;
                }
            }
            _ => {
                self.overlay = Overlay::None;
            }
        }
    }

    pub fn new_conversation(&mut self) {
        if !self.messages.is_empty() {
            let _ = self.conversation.save();
        }
        self.messages.clear();
        self.conversation = Conversation::new();
        self.scroll_offset = 0;
        self.status_message = Some("New conversation".into());
    }

    pub fn load_history_list(&mut self) {
        self.history_list = Conversation::list_all().unwrap_or_default();
        self.overlay_scroll = 0;
    }

    pub fn execute_command(&mut self, cmd: &str) {
        match cmd.trim() {
            "q" | "quit" => self.should_quit = true,
            "w" | "save" => {
                let _ = self.config.save();
                self.status_message = Some("Config saved".into());
            }
            "wq" => {
                let _ = self.config.save();
                self.should_quit = true;
            }
            "clear" | "c" => {
                self.messages.clear();
                self.conversation = Conversation::new();
            }
            "new" | "n" => self.new_conversation(),
            "help" | "h" => self.overlay = Overlay::Help,
            "history" => {
                self.overlay = Overlay::History;
                self.load_history_list();
            }
            _ => {
                // Check for :set commands
                if let Some(rest) = cmd.strip_prefix("set ") {
                    self.handle_set_command(rest);
                } else if let Some(rest) = cmd.strip_prefix("model ") {
                    self.config.model = rest.trim().to_string();
                    self.status_message = Some(format!("Model: {}", self.config.model));
                } else {
                    self.status_message = Some(format!("Unknown command: :{cmd}"));
                }
            }
        }
    }

    fn handle_set_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.splitn(2, '=').collect();
        match parts[0].trim() {
            "model" => {
                if let Some(val) = parts.get(1) {
                    self.config.model = val.trim().to_string();
                    self.status_message = Some(format!("Model: {}", self.config.model));
                }
            }
            "temp" | "temperature" => {
                if let Some(val) = parts.get(1) {
                    if let Ok(t) = val.trim().parse::<f32>() {
                        self.config.temperature = t;
                        self.status_message = Some(format!("Temperature: {t}"));
                    }
                }
            }
            "provider" => {
                if let Some(val) = parts.get(1) {
                    self.config.provider = val.trim().to_string();
                    self.status_message = Some(format!("Provider: {}", self.config.provider));
                }
            }
            "vim" => {
                self.config.vim_mode = !self.config.vim_mode;
                self.status_message = Some(format!("Vim mode: {}", self.config.vim_mode));
            }
            _ => {
                self.status_message = Some(format!("Unknown setting: {}", parts[0]));
            }
        }
    }
}
