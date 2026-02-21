use crossterm::event::MouseEventKind;
use ratatui::prelude::*;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::api::{ApiClient, Message, MessageContent};
use crate::config::{Config, ThemeColors, get_theme};
use crate::event::{Event, EventHandler};
use crate::history::Conversation;
use crate::keybinds::{handle_key, KeyAction};
use crate::neovim::NeovimClient;
use crate::tools::{self, ToolCall, ToolExecutor, ToolPermission, ToolResult};
use crate::ui;

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Insert,
    Command,
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    Help,
    History,
    Settings,
    ToolConfirm,
}

/// Represents a tool invocation displayed in the chat.
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    pub tool_name: String,
    pub tool_args: String,
    pub result: Option<ToolResult>,
    pub collapsed: bool,
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
    pub tool_executor: ToolExecutor,
    pub pending_tool_calls: Vec<ToolCall>,
    pub pending_tool_confirm_idx: usize,
    pub tool_invocations: Vec<ToolInvocation>,
    /// Full API message history (includes tool_use and tool_result blocks)
    pub api_messages: Vec<Message>,
    /// Whether tools are enabled for this session
    pub tools_enabled: bool,
    /// Whether we're in visual selection mode (for code block picking)
    pub visual_mode: bool,
    /// Start message index for visual selection
    pub visual_start: usize,
    /// End message index for visual selection (multi-message select)
    pub visual_end: usize,
    /// Extracted code blocks: (message_index, language, content)
    pub code_blocks: Vec<(usize, String, String)>,
    /// Search query string (for / search mode)
    pub search_query: String,
    /// Indices of messages matching the search
    pub search_matches: Vec<usize>,
    /// Current search match index
    pub search_match_idx: usize,
    /// Tick counter for animations
    pub tick_count: u64,
    /// When the current stream started
    pub stream_start_time: Option<std::time::Instant>,
    /// Duration of the last completed response
    pub last_response_time: Option<std::time::Duration>,
    /// Whether to auto-scroll to bottom on new content
    pub auto_scroll: bool,
    /// Undo stack for input field: (input_text, cursor_pos)
    pub undo_stack: Vec<(String, usize)>,
    /// Redo stack for input field: (input_text, cursor_pos)
    pub redo_stack: Vec<(String, usize)>,
    api_client: ApiClient,
    event_tx: Option<mpsc::UnboundedSender<Event>>,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// When this message was created
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Optional tool invocations associated with this message
    pub tool_invocations: Vec<ToolInvocation>,
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

        let mut tool_executor = ToolExecutor::new();
        // Auto-allow read-only tools
        tool_executor.set_permission("read_file", ToolPermission::AutoAllow);
        tool_executor.set_permission("list_files", ToolPermission::AutoAllow);
        tool_executor.set_permission("search_files", ToolPermission::AutoAllow);

        let last_conversation_id = config.last_conversation_id.clone();

        let mut app = Self {
            config,
            input: String::new(),
            input_mode: InputMode::Insert,
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
            tool_executor,
            pending_tool_calls: Vec::new(),
            pending_tool_confirm_idx: 0,
            tool_invocations: Vec::new(),
            api_messages: Vec::new(),
            tools_enabled: true,
            visual_mode: false,
            visual_start: 0,
            visual_end: 0,
            code_blocks: Vec::new(),
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: 0,
            tick_count: 0,
            stream_start_time: None,
            last_response_time: None,
            auto_scroll: true,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            api_client: ApiClient::new(),
            event_tx: None,
        };

        // Auto-restore last conversation if configured
        if let Some(ref id) = last_conversation_id {
            if app.load_conversation(id).is_ok() {
                app.status_message = Some("Restored previous session".into());
            }
        }

        app
    }

    /// Estimate the number of tokens in the conversation.
    /// Uses a simple heuristic: total characters / 4 (rough average for English text with code).
    pub fn estimate_tokens(&self) -> usize {
        let total_chars: usize = self.messages.iter().map(|m| m.content.len()).sum();
        total_chars / 4
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

    /// Return the resolved theme colors based on the current config theme_name.
    pub fn colors(&self) -> ThemeColors {
        get_theme(&self.config.theme_name)
    }

    pub fn load_conversation(&mut self, id: &str) -> anyhow::Result<()> {
        let conv = Conversation::load(id)?;
        self.messages = conv.messages.iter().map(|m| ChatMessage {
            role: m.role.clone(),
            content: m.content.clone(),
            timestamp: m.timestamp,
            tool_invocations: Vec::new(),
        }).collect();
        self.conversation = conv;
        self.scroll_to_bottom();
        Ok(())
    }

    /// Save the current conversation and update the config to track it as the last session.
    fn save_and_track_conversation(&mut self) {
        let _ = self.conversation.save();
        self.config.last_conversation_id = Some(self.conversation.id.clone());
        let _ = self.config.save();
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
            terminal.draw(|f| {
                self.terminal_height = f.area().height;
                ui::draw(f, self);
            })?;

            if let Some(event) = events.next().await {
                match event {
                    Event::Key(key) => {
                        self.status_message = None;

                        // Handle tool confirmation overlay keys
                        if self.overlay == Overlay::ToolConfirm {
                            self.handle_tool_confirm_key(key).await;
                            continue;
                        }

                        match handle_key(self, key) {
                            KeyAction::Quit => {
                                if !self.messages.is_empty() {
                                    self.save_and_track_conversation();
                                }
                                return Ok(());
                            }
                            KeyAction::SendMessage => {
                                self.send_message().await?;
                            }
                            KeyAction::CancelStream => {
                                self.cancel_stream();
                            }
                            KeyAction::RetryMessage => {
                                self.retry_last().await?;
                            }
                            KeyAction::EditLastMessage => {
                                self.edit_last_message();
                            }
                            _ => {}
                        }
                    }
                    Event::ApiChunk(text) => {
                        self.stream_buffer.push_str(&text);
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = self.stream_buffer.clone();
                            }
                        }
                        if self.auto_scroll {
                            self.scroll_to_bottom();
                        }
                    }
                    Event::ApiDone => {
                        self.streaming = false;
                        if let Some(start) = self.stream_start_time.take() {
                            self.last_response_time = Some(start.elapsed());
                        }
                        if !self.stream_buffer.is_empty() {
                            self.conversation.add_message("assistant", &self.stream_buffer);
                            self.save_and_track_conversation();
                        }
                        self.stream_buffer.clear();
                        // Ring terminal bell to notify user the response is complete
                        if self.config.notify_on_complete {
                            eprint!("\x07");
                        }
                    }
                    Event::ApiError(err) => {
                        self.streaming = false;
                        self.stream_start_time = None;
                        self.stream_buffer.clear();
                        if let Some(last) = self.messages.last() {
                            if last.role == "assistant" && last.content.is_empty() {
                                self.messages.pop();
                            }
                        }
                        self.status_message = Some(format!("Error: {err}"));
                    }
                    Event::ToolUseRequest(response_body) => {
                        self.streaming = false;
                        self.handle_tool_use_response(&response_body).await;
                    }
                    Event::Resize(_, h) => {
                        self.terminal_height = h;
                    }
                    Event::Tick => {
                        self.tick_count = self.tick_count.wrapping_add(1);
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => self.scroll_up(3),
                            MouseEventKind::ScrollDown => self.scroll_down(3),
                            _ => {}
                        }
                    }
                }
            }

            if self.should_quit {
                return Ok(());
            }
        }
    }

    /// Handle a tool_use response from the API.
    async fn handle_tool_use_response(&mut self, response_body: &str) {
        let response: Value = match serde_json::from_str(response_body) {
            Ok(v) => v,
            Err(e) => {
                self.status_message = Some(format!("Failed to parse tool response: {e}"));
                return;
            }
        };

        // Store the assistant's response in api_messages (with tool_use blocks)
        self.api_messages.push(Message {
            role: "assistant".into(),
            content: MessageContent::Blocks(
                response["content"].as_array().cloned().unwrap_or_default()
            ),
        });

        // Parse tool calls
        let tool_calls = tools::parse_tool_calls(&response);
        if tool_calls.is_empty() {
            return;
        }

        // Save current stream text to the last assistant message
        if !self.stream_buffer.is_empty() {
            if let Some(last) = self.messages.last_mut() {
                if last.role == "assistant" {
                    last.content = self.stream_buffer.clone();
                }
            }
        }
        self.stream_buffer.clear();

        self.pending_tool_calls = tool_calls;
        self.pending_tool_confirm_idx = 0;

        // Process tool calls - auto-allow or prompt
        self.process_next_tool_call().await;
    }

    /// Process tool calls one by one. Auto-allowed ones run immediately,
    /// otherwise show confirmation overlay.
    async fn process_next_tool_call(&mut self) {
        while self.pending_tool_confirm_idx < self.pending_tool_calls.len() {
            let call = &self.pending_tool_calls[self.pending_tool_confirm_idx];
            let perm = self.tool_executor.permission(call.tool.name());

            match perm {
                ToolPermission::AutoAllow => {
                    self.execute_tool_at_index(self.pending_tool_confirm_idx);
                    self.pending_tool_confirm_idx += 1;
                }
                ToolPermission::AskFirst => {
                    // Show confirmation overlay
                    self.overlay = Overlay::ToolConfirm;
                    return;
                }
                ToolPermission::Deny => {
                    // Add a denied result
                    let call = &self.pending_tool_calls[self.pending_tool_confirm_idx];
                    let invocation = ToolInvocation {
                        tool_name: call.tool.name().to_string(),
                        tool_args: format_tool_args(&call.tool),
                        result: Some(ToolResult::err("Tool execution denied by user")),
                        collapsed: false,
                    };
                    self.tool_invocations.push(invocation);
                    if let Some(last) = self.messages.last_mut() {
                        if last.role == "assistant" {
                            last.tool_invocations.push(ToolInvocation {
                                tool_name: call.tool.name().to_string(),
                                tool_args: format_tool_args(&call.tool),
                                result: Some(ToolResult::err("Denied")),
                                collapsed: false,
                            });
                        }
                    }
                    self.pending_tool_confirm_idx += 1;
                }
            }
        }

        // All tool calls processed - send results back to the API
        self.send_tool_results().await;
    }

    fn execute_tool_at_index(&mut self, idx: usize) {
        let call = &self.pending_tool_calls[idx];
        let result = self.tool_executor.execute(&call.tool);

        let invocation = ToolInvocation {
            tool_name: call.tool.name().to_string(),
            tool_args: format_tool_args(&call.tool),
            result: Some(result.clone()),
            collapsed: result.output.lines().count() > 10,
        };

        // Add to the current assistant message's tool invocations
        if let Some(last) = self.messages.last_mut() {
            if last.role == "assistant" {
                last.tool_invocations.push(invocation.clone());
            }
        }
        self.tool_invocations.push(invocation);
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    async fn handle_tool_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Allow this tool
                self.overlay = Overlay::None;
                self.execute_tool_at_index(self.pending_tool_confirm_idx);
                self.pending_tool_confirm_idx += 1;
                self.process_next_tool_call().await;
            }
            KeyCode::Char('a') => {
                // Always allow this tool type
                let tool_name = self.pending_tool_calls[self.pending_tool_confirm_idx]
                    .tool.name().to_string();
                self.tool_executor.set_permission(&tool_name, ToolPermission::AutoAllow);
                self.overlay = Overlay::None;
                self.execute_tool_at_index(self.pending_tool_confirm_idx);
                self.pending_tool_confirm_idx += 1;
                self.process_next_tool_call().await;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                // Deny this tool
                let call = &self.pending_tool_calls[self.pending_tool_confirm_idx];
                let invocation = ToolInvocation {
                    tool_name: call.tool.name().to_string(),
                    tool_args: format_tool_args(&call.tool),
                    result: Some(ToolResult::err("Denied by user")),
                    collapsed: false,
                };
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.tool_invocations.push(invocation.clone());
                    }
                }
                self.tool_invocations.push(invocation);
                self.overlay = Overlay::None;
                self.pending_tool_confirm_idx += 1;
                self.process_next_tool_call().await;
            }
            KeyCode::Char('d') => {
                // Deny all of this type
                let tool_name = self.pending_tool_calls[self.pending_tool_confirm_idx]
                    .tool.name().to_string();
                self.tool_executor.set_permission(&tool_name, ToolPermission::Deny);
                let call = &self.pending_tool_calls[self.pending_tool_confirm_idx];
                let invocation = ToolInvocation {
                    tool_name: call.tool.name().to_string(),
                    tool_args: format_tool_args(&call.tool),
                    result: Some(ToolResult::err("Denied by user")),
                    collapsed: false,
                };
                if let Some(last) = self.messages.last_mut() {
                    if last.role == "assistant" {
                        last.tool_invocations.push(invocation.clone());
                    }
                }
                self.tool_invocations.push(invocation);
                self.overlay = Overlay::None;
                self.pending_tool_confirm_idx += 1;
                self.process_next_tool_call().await;
            }
            _ => {}
        }
    }

    /// Send all tool results back to the API and continue the conversation.
    async fn send_tool_results(&mut self) {
        let mut tool_results: Vec<Value> = Vec::new();

        for (i, call) in self.pending_tool_calls.iter().enumerate() {
            let result = if let Some(inv) = self.tool_invocations.iter().rev()
                .find(|inv| inv.tool_name == call.tool.name())
            {
                inv.result.clone().unwrap_or_else(|| ToolResult::err("No result"))
            } else {
                ToolResult::err("Tool not executed")
            };

            tool_results.push(serde_json::json!({
                "type": "tool_result",
                "tool_use_id": call.id,
                "content": result.output,
                "is_error": !result.success,
            }));
            let _ = i; // used in iteration
        }

        if tool_results.is_empty() {
            return;
        }

        // Add tool results as a user message (Anthropic API format)
        self.api_messages.push(Message {
            role: "user".into(),
            content: MessageContent::Blocks(tool_results),
        });

        self.pending_tool_calls.clear();
        self.pending_tool_confirm_idx = 0;

        // Continue the conversation - make another API call
        self.streaming = true;
        self.stream_start_time = Some(std::time::Instant::now());
        self.stream_buffer.clear();

        // Add a new assistant placeholder for the continuation
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            timestamp: chrono::Utc::now(),
            tool_invocations: Vec::new(),
        });

        let api_key = match self.config.api_key_from_env() {
            Some(key) => key,
            None => return,
        };

        let tx = self.event_tx.clone().unwrap();
        let model = self.config.model.clone();
        let system = self.config.system_prompt.clone();
        let max_tokens = self.config.max_tokens;
        let temp = self.config.temperature;
        let messages = self.api_messages.clone();
        let tools_enabled = self.tools_enabled;
        let client = ApiClient::new();

        tokio::spawn(async move {
            let result = if tools_enabled {
                client.call_anthropic_with_tools(
                    &api_key, &model, &messages,
                    system.as_deref(), max_tokens, temp, tx.clone(),
                ).await
            } else {
                client.stream_anthropic(
                    &api_key, &model, &messages,
                    system.as_deref(), max_tokens, temp, tx.clone(),
                ).await
            };

            if let Err(e) = result {
                let _ = tx.send(Event::ApiError(e.to_string()));
            }
        });
    }

    pub async fn send_message(&mut self) -> anyhow::Result<()> {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        if input.starts_with('/') {
            return self.handle_slash_command(&input);
        }

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
            timestamp: chrono::Utc::now(),
            tool_invocations: Vec::new(),
        });
        self.conversation.add_message("user", &input);

        // Add to API message history
        self.api_messages.push(Message {
            role: "user".into(),
            content: MessageContent::Text(input.clone()),
        });

        self.input_history.push(input);
        self.input_history_idx = None;
        self.input.clear();
        self.cursor_pos = 0;

        // Add placeholder for assistant
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            timestamp: chrono::Utc::now(),
            tool_invocations: Vec::new(),
        });

        self.streaming = true;
        self.stream_start_time = Some(std::time::Instant::now());
        self.stream_buffer.clear();
        self.scroll_to_bottom();

        let tx = self.event_tx.clone().unwrap();
        let provider = self.config.provider.clone();
        let model = self.config.model.clone();
        let system = self.config.system_prompt.clone();
        let max_tokens = self.config.max_tokens;
        let temp = self.config.temperature;
        let messages = self.api_messages.clone();
        let tools_enabled = self.tools_enabled && provider == "anthropic";
        let client = ApiClient::new();

        tokio::spawn(async move {
            let result = match provider.as_str() {
                "openai" => {
                    client.stream_openai(
                        &api_key, &model, &messages,
                        system.as_deref(), max_tokens, temp, tx.clone(),
                    ).await
                }
                _ => {
                    if tools_enabled {
                        client.call_anthropic_with_tools(
                            &api_key, &model, &messages,
                            system.as_deref(), max_tokens, temp, tx.clone(),
                        ).await
                    } else {
                        client.stream_anthropic(
                            &api_key, &model, &messages,
                            system.as_deref(), max_tokens, temp, tx.clone(),
                        ).await
                    }
                }
            };

            if let Err(e) = result {
                let _ = tx.send(Event::ApiError(e.to_string()));
            }
        });

        Ok(())
    }

    /// Retry/regenerate the last assistant response.
    /// Removes the last assistant message and re-sends to the API.
    pub async fn retry_last(&mut self) -> anyhow::Result<()> {
        if self.streaming {
            self.status_message = Some("Cannot retry while streaming".into());
            return Ok(());
        }

        // Remove the last assistant message from display messages
        if let Some(last) = self.messages.last() {
            if last.role != "assistant" {
                self.status_message = Some("No assistant message to retry".into());
                return Ok(());
            }
        } else {
            self.status_message = Some("No messages to retry".into());
            return Ok(());
        }
        self.messages.pop();

        // Remove the last assistant message from api_messages
        if let Some(pos) = self.api_messages.iter().rposition(|m| m.role == "assistant") {
            self.api_messages.remove(pos);
        }

        // Remove from conversation history
        if let Some(pos) = self.conversation.messages.iter().rposition(|m| m.role == "assistant") {
            self.conversation.messages.remove(pos);
        }

        // Check we still have a user message to respond to
        if self.api_messages.is_empty() {
            self.status_message = Some("No user message to retry".into());
            return Ok(());
        }

        let api_key = match self.config.api_key_from_env() {
            Some(key) => key,
            None => {
                self.status_message = Some("No API key set".into());
                return Ok(());
            }
        };

        self.status_message = Some("Regenerating...".into());

        // Add placeholder for new assistant response
        self.messages.push(ChatMessage {
            role: "assistant".into(),
            content: String::new(),
            timestamp: chrono::Utc::now(),
            tool_invocations: Vec::new(),
        });

        self.streaming = true;
        self.stream_start_time = Some(std::time::Instant::now());
        self.stream_buffer.clear();
        self.scroll_to_bottom();

        let tx = self.event_tx.clone().unwrap();
        let provider = self.config.provider.clone();
        let model = self.config.model.clone();
        let system = self.config.system_prompt.clone();
        let max_tokens = self.config.max_tokens;
        let temp = self.config.temperature;
        let messages = self.api_messages.clone();
        let tools_enabled = self.tools_enabled && provider == "anthropic";
        let client = ApiClient::new();

        tokio::spawn(async move {
            let result = match provider.as_str() {
                "openai" => {
                    client.stream_openai(
                        &api_key, &model, &messages,
                        system.as_deref(), max_tokens, temp, tx.clone(),
                    ).await
                }
                _ => {
                    if tools_enabled {
                        client.call_anthropic_with_tools(
                            &api_key, &model, &messages,
                            system.as_deref(), max_tokens, temp, tx.clone(),
                        ).await
                    } else {
                        client.stream_anthropic(
                            &api_key, &model, &messages,
                            system.as_deref(), max_tokens, temp, tx.clone(),
                        ).await
                    }
                }
            };

            if let Err(e) = result {
                let _ = tx.send(Event::ApiError(e.to_string()));
            }
        });

        Ok(())
    }

    /// Edit the last user message: put it back in input, remove it and the following assistant response.
    pub fn edit_last_message(&mut self) {
        if self.streaming {
            self.status_message = Some("Cannot edit while streaming".into());
            return;
        }

        // Find the last user message
        let last_user_idx = match self.messages.iter().rposition(|m| m.role == "user") {
            Some(idx) => idx,
            None => {
                self.status_message = Some("No user message to edit".into());
                return;
            }
        };

        // Get the content and put it in the input field
        let content = self.messages[last_user_idx].content.clone();
        self.input = content;
        self.cursor_pos = self.input.len();

        // Remove the user message and any following assistant message from display messages
        let remove_count = if last_user_idx + 1 < self.messages.len()
            && self.messages[last_user_idx + 1].role == "assistant"
        {
            2
        } else {
            1
        };
        self.messages.drain(last_user_idx..last_user_idx + remove_count);

        // Remove corresponding messages from api_messages
        if let Some(api_user_idx) = self.api_messages.iter().rposition(|m| m.role == "user") {
            let api_remove_end = if api_user_idx + 1 < self.api_messages.len()
                && self.api_messages[api_user_idx + 1].role == "assistant"
            {
                api_user_idx + 2
            } else {
                api_user_idx + 1
            };
            self.api_messages.drain(api_user_idx..api_remove_end);
        }

        // Remove from conversation history
        if let Some(conv_user_idx) = self.conversation.messages.iter().rposition(|m| m.role == "user") {
            let conv_remove_end = if conv_user_idx + 1 < self.conversation.messages.len()
                && self.conversation.messages[conv_user_idx + 1].role == "assistant"
            {
                conv_user_idx + 2
            } else {
                conv_user_idx + 1
            };
            self.conversation.messages.drain(conv_user_idx..conv_remove_end);
        }

        // Switch to insert mode so the user can edit
        self.input_mode = InputMode::Insert;
        self.status_message = Some("Editing last message".into());
    }

    fn handle_slash_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            "/clear" | "/c" => {
                self.messages.clear();
                self.api_messages.clear();
                self.tool_invocations.clear();
                self.conversation = Conversation::new();
                self.status_message = Some("Conversation cleared".into());
            }
            "/new" | "/n" => {
                self.new_conversation();
            }
            "/model" | "/m" => {
                if let Some(model) = parts.get(1) {
                    let resolved = Self::resolve_model_alias(model);
                    self.config.model = resolved.clone();
                    self.status_message = Some(format!("Model set to {resolved}"));
                } else {
                    self.status_message = Some(format!("Current model: {}", self.config.model));
                }
            }
            "/models" => {
                self.status_message = Some(
                    "Model aliases: sonnet/s -> claude-sonnet-4-20250514, opus/o -> claude-opus-4-20250514, \
                     haiku/h -> claude-haiku-4-5-20251001, gpt4 -> gpt-4o, gpt4m -> gpt-4o-mini"
                        .into(),
                );
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
            "/tools" => {
                if let Some(arg) = parts.get(1) {
                    match *arg {
                        "on" => {
                            self.tools_enabled = true;
                            self.status_message = Some("Tools enabled".into());
                        }
                        "off" => {
                            self.tools_enabled = false;
                            self.status_message = Some("Tools disabled".into());
                        }
                        _ => {
                            self.status_message = Some("Usage: /tools [on|off]".into());
                        }
                    }
                } else {
                    let status = if self.tools_enabled { "on" } else { "off" };
                    let perms: Vec<String> = ["read_file", "write_file", "edit_file", "list_files", "search_files", "execute"]
                        .iter()
                        .map(|t| {
                            let p = self.tool_executor.permission(t);
                            format!("  {t}: {p:?}")
                        })
                        .collect();
                    self.status_message = Some(format!("Tools: {status}\n{}", perms.join("\n")));
                }
            }
            "/file" | "/f" => {
                if let Some(path_str) = parts.get(1) {
                    let path = std::path::Path::new(path_str.trim());
                    if path.exists() {
                        // Check for binary file: look for null bytes in first 512 bytes
                        match std::fs::read(path) {
                            Ok(raw_bytes) => {
                                let check_len = raw_bytes.len().min(512);
                                if raw_bytes[..check_len].contains(&0u8) {
                                    self.status_message = Some(format!(
                                        "Cannot load binary file: {}", path_str.trim()
                                    ));
                                } else {
                                    let file_size = raw_bytes.len();
                                    let filename = path.file_name()
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_else(|| path_str.to_string());
                                    let ext = path.extension()
                                        .map(|e| e.to_string_lossy().to_string())
                                        .unwrap_or_default();

                                    let max_size: usize = 100 * 1024; // 100KB
                                    let mut content = String::from_utf8_lossy(&raw_bytes).to_string();
                                    let truncated = if file_size > max_size {
                                        content.truncate(max_size);
                                        true
                                    } else {
                                        false
                                    };

                                    let size_display = if file_size >= 1024 * 1024 {
                                        format!("{:.1} MB", file_size as f64 / (1024.0 * 1024.0))
                                    } else if file_size >= 1024 {
                                        format!("{:.1} KB", file_size as f64 / 1024.0)
                                    } else {
                                        format!("{} B", file_size)
                                    };

                                    if truncated {
                                        self.input = format!(
                                            "Here is the contents of `{filename}`:\n```{ext}\n{content}\n```\n\n**Note: File was truncated at 100KB. Original size: {size_display}**\n"
                                        );
                                    } else {
                                        self.input = format!(
                                            "Here is the contents of `{filename}`:\n```{ext}\n{content}\n```\n"
                                        );
                                    }
                                    self.cursor_pos = 0;
                                    self.status_message = Some(format!(
                                        "Loaded {filename} ({size_display}) into input"
                                    ));
                                    return Ok(());
                                }
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Error reading file: {e}"));
                            }
                        }
                    } else {
                        self.status_message = Some(format!("File not found: {}", path_str.trim()));
                    }
                } else {
                    self.status_message = Some("Usage: /file <path>".into());
                }
            }
            "/context" | "/ctx" => {
                self.load_project_context();
            }
            "/paste" => {
                self.paste_clipboard_as_codeblock();
            }
            "/resume" | "/r" => {
                if let Some(ref id) = self.config.last_conversation_id.clone() {
                    match self.load_conversation(id) {
                        Ok(_) => self.status_message = Some("Resumed last session".into()),
                        Err(e) => self.status_message = Some(format!("Failed to resume: {e}")),
                    }
                } else {
                    // Fall back to the most recently updated conversation
                    match Conversation::latest() {
                        Ok(Some(conv)) => {
                            let id = conv.id.clone();
                            match self.load_conversation(&id) {
                                Ok(_) => self.status_message = Some("Resumed latest conversation".into()),
                                Err(e) => self.status_message = Some(format!("Failed to resume: {e}")),
                            }
                        }
                        _ => self.status_message = Some("No previous conversation found".into()),
                    }
                }
            }
            "/diff" | "/d" => {
                match std::process::Command::new("git")
                    .arg("diff")
                    .output()
                {
                    Ok(output) => {
                        let diff_output = String::from_utf8_lossy(&output.stdout).to_string();
                        if diff_output.trim().is_empty() {
                            self.status_message = Some("No changes detected (git diff is empty)".into());
                        } else {
                            self.input = format!(
                                "Here are my current git changes:\n```diff\n{diff_output}\n```\nPlease review these changes.\n"
                            );
                            self.cursor_pos = 0;
                            self.status_message = Some("Loaded git diff into input".into());
                            return Ok(());
                        }
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to run git diff: {e}"));
                    }
                }
            }
            "/export" => {
                self.export_conversation(parts.get(1).map(|s| s.trim()));
            }
            "/theme" => {
                if let Some(name) = parts.get(1) {
                    let name = name.trim();
                    let valid = ["tokyo-night", "catppuccin", "gruvbox", "dracula"];
                    if valid.contains(&name) {
                        self.config.theme_name = name.to_string();
                        self.status_message = Some(format!("Theme set to {name}"));
                    } else {
                        self.status_message = Some(format!(
                            "Unknown theme: {name}. Available: {}",
                            valid.join(", ")
                        ));
                    }
                } else {
                    self.status_message = Some(format!("Current theme: {}", self.config.theme_name));
                }
            }
            "/retry" => {
                // Handled specially: set status and return so the caller
                // can invoke the async retry_last method.
                self.input.clear();
                self.cursor_pos = 0;
                // We cannot call async from here, so remove the last assistant
                // message and set a flag via status_message that retry is needed.
                // Instead, inline the sync part and leave it to the user to re-send.
                if self.streaming {
                    self.status_message = Some("Cannot retry while streaming".into());
                } else if self.messages.last().map_or(true, |m| m.role != "assistant") {
                    self.status_message = Some("No assistant message to retry".into());
                } else {
                    // Remove the last assistant message
                    self.messages.pop();
                    if let Some(pos) = self.api_messages.iter().rposition(|m| m.role == "assistant") {
                        self.api_messages.remove(pos);
                    }
                    if let Some(pos) = self.conversation.messages.iter().rposition(|m| m.role == "assistant") {
                        self.conversation.messages.remove(pos);
                    }
                    self.status_message = Some("Removed last response. Use Ctrl+r to regenerate, or re-send your message.".into());
                }
                return Ok(());
            }
            "/edit" => {
                self.input.clear();
                self.cursor_pos = 0;
                self.edit_last_message();
                return Ok(());
            }
            "/run" | "/!" => {
                if let Some(cmd_str) = parts.get(1) {
                    let cmd_str = cmd_str.trim();
                    if cmd_str.is_empty() {
                        self.status_message = Some("Usage: /run <command>".into());
                    } else {
                        match std::process::Command::new("sh")
                            .arg("-c")
                            .arg(cmd_str)
                            .output()
                        {
                            Ok(output) => {
                                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                                let full_output = if stderr.is_empty() {
                                    stdout.clone()
                                } else if stdout.is_empty() {
                                    format!("stderr: {stderr}")
                                } else {
                                    format!("{stdout}\nstderr: {stderr}")
                                };
                                let full_output = full_output.trim().to_string();

                                if full_output.len() > 200 {
                                    let truncated: String = full_output.chars().take(200).collect();
                                    self.status_message = Some(format!("$ {cmd_str}: {truncated}..."));
                                    self.input = format!(
                                        "Output of `{cmd_str}`:\n```\n{full_output}\n```\n"
                                    );
                                    self.cursor_pos = 0;
                                } else if full_output.is_empty() {
                                    let code = output.status.code().unwrap_or(-1);
                                    self.status_message = Some(format!("$ {cmd_str}: (exit {code}, no output)"));
                                } else {
                                    self.status_message = Some(format!("$ {cmd_str}: {full_output}"));
                                }
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Failed to run: {e}"));
                            }
                        }
                    }
                } else {
                    self.status_message = Some("Usage: /run <command>".into());
                }
            }
            "/undo" => {
                self.input.clear();
                self.cursor_pos = 0;
                self.undo();
                return Ok(());
            }
            "/redo" => {
                self.input.clear();
                self.cursor_pos = 0;
                self.redo();
                return Ok(());
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

    /// Export the current conversation to a markdown file.
    fn export_conversation(&mut self, path_arg: Option<&str>) {
        if self.messages.is_empty() {
            self.status_message = Some("No messages to export".into());
            return;
        }

        let path = if let Some(p) = path_arg {
            if p.is_empty() {
                self.default_export_path()
            } else {
                std::path::PathBuf::from(p)
            }
        } else {
            self.default_export_path()
        };

        let mut content = String::new();
        for msg in &self.messages {
            let label = match msg.role.as_str() {
                "user" => "You",
                "assistant" => "Assistant",
                _ => "System",
            };
            content.push_str(&format!("## {label}\n\n"));
            content.push_str(&msg.content);
            content.push_str("\n\n");

            // Include tool invocations if any
            for inv in &msg.tool_invocations {
                content.push_str(&format!("**Tool: {}**\n", inv.tool_name));
                content.push_str(&format!("Args: {}\n", inv.tool_args));
                if let Some(ref result) = inv.result {
                    let status = if result.success { "Success" } else { "Error" };
                    content.push_str(&format!("Result ({status}):\n```\n{}\n```\n\n", result.output));
                }
            }
        }

        match std::fs::write(&path, &content) {
            Ok(()) => {
                self.status_message = Some(format!("Exported to {}", path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Export failed: {e}"));
            }
        }
    }

    fn default_export_path(&self) -> std::path::PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        std::path::PathBuf::from(format!("./chat-export-{timestamp}.md"))
    }

    pub fn cancel_stream(&mut self) {
        self.streaming = false;
        self.stream_start_time = None;
        if !self.stream_buffer.is_empty() {
            self.conversation.add_message("assistant", &self.stream_buffer);
        }
        self.stream_buffer.clear();
        self.status_message = Some("Stream cancelled".into());
    }

    // Undo/redo support
    /// Save the current input state to the undo stack and clear the redo stack.
    /// Called before any editing operation.
    fn save_undo_state(&mut self) {
        self.undo_stack.push((self.input.clone(), self.cursor_pos));
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    /// Undo the last input editing operation.
    pub fn undo(&mut self) {
        if let Some((text, pos)) = self.undo_stack.pop() {
            self.redo_stack.push((self.input.clone(), self.cursor_pos));
            self.input = text;
            self.cursor_pos = pos;
            self.status_message = Some("Undo".into());
        } else {
            self.status_message = Some("Nothing to undo".into());
        }
    }

    /// Redo the last undone input editing operation.
    pub fn redo(&mut self) {
        if let Some((text, pos)) = self.redo_stack.pop() {
            self.undo_stack.push((self.input.clone(), self.cursor_pos));
            self.input = text;
            self.cursor_pos = pos;
            self.status_message = Some("Redo".into());
        } else {
            self.status_message = Some("Nothing to redo".into());
        }
    }

    // Text editing operations
    pub fn insert_char(&mut self, c: char) {
        self.save_undo_state();
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn insert_newline(&mut self) {
        self.save_undo_state();
        self.input.insert(self.cursor_pos, '\n');
        self.cursor_pos += 1;
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            self.save_undo_state();
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
            self.save_undo_state();
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn delete_word_before_cursor(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.save_undo_state();
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let word_start = trimmed.rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input = format!("{}{}", &self.input[..word_start], &self.input[self.cursor_pos..]);
        self.cursor_pos = word_start;
    }

    pub fn delete_to_start(&mut self) {
        self.save_undo_state();
        self.input = self.input[self.cursor_pos..].to_string();
        self.cursor_pos = 0;
    }

    pub fn clear_input(&mut self) {
        self.save_undo_state();
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
        let before = &self.input[..self.cursor_pos];
        self.cursor_pos = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    }

    pub fn cursor_end(&mut self) {
        let after = &self.input[self.cursor_pos..];
        self.cursor_pos += after.find('\n').unwrap_or(after.len());
    }

    pub fn cursor_word_forward(&mut self) {
        let after = &self.input[self.cursor_pos..];
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

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
        self.auto_scroll = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = usize::MAX;
        self.auto_scroll = true;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn execute_search(&mut self) {
        self.search_matches.clear();
        self.search_match_idx = 0;
        if self.search_query.is_empty() {
            return;
        }
        let query = self.search_query.to_lowercase();
        for (i, msg) in self.messages.iter().enumerate() {
            if msg.content.to_lowercase().contains(&query) {
                self.search_matches.push(i);
            }
        }
        if !self.search_matches.is_empty() {
            self.scroll_to_match(0);
            self.status_message = Some(format!(
                "/{}: match {}/{}",
                self.search_query, 1, self.search_matches.len()
            ));
        } else {
            self.status_message = Some(format!("Pattern not found: {}", self.search_query));
        }
    }

    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        self.scroll_to_match(self.search_match_idx);
        self.status_message = Some(format!(
            "/{}: match {}/{}",
            self.search_query, self.search_match_idx + 1, self.search_matches.len()
        ));
    }

    pub fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_match_idx == 0 {
            self.search_match_idx = self.search_matches.len() - 1;
        } else {
            self.search_match_idx -= 1;
        }
        self.scroll_to_match(self.search_match_idx);
        self.status_message = Some(format!(
            "/{}: match {}/{}",
            self.search_query, self.search_match_idx + 1, self.search_matches.len()
        ));
    }

    fn scroll_to_match(&mut self, match_idx: usize) {
        if let Some(&msg_idx) = self.search_matches.get(match_idx) {
            let estimated_line = msg_idx * 4;
            self.scroll_offset = estimated_line;
        }
    }

    pub fn paste_clipboard(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                // save_undo_state is called by insert_char, but we save once
                // here so the entire paste can be undone in a single step.
                self.save_undo_state();
                for c in text.chars() {
                    self.input.insert(self.cursor_pos, c);
                    self.cursor_pos += c.len_utf8();
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

    /// Scan all assistant messages for fenced code blocks (```...```)
    /// and store them in self.code_blocks as (msg_idx, language, content).
    pub fn extract_code_blocks(&mut self) {
        self.code_blocks.clear();
        for (msg_idx, msg) in self.messages.iter().enumerate() {
            if msg.role != "assistant" {
                continue;
            }
            let content = &msg.content;
            let mut search_from = 0;
            while let Some(fence_start) = content[search_from..].find("```") {
                let abs_fence_start = search_from + fence_start;
                let after_backticks = abs_fence_start + 3;
                // Extract language from the opening fence line
                let line_end = content[after_backticks..]
                    .find('\n')
                    .map(|i| after_backticks + i)
                    .unwrap_or(content.len());
                let lang = content[after_backticks..line_end].trim().to_string();
                let code_start = if line_end < content.len() { line_end + 1 } else { line_end };
                // Find closing fence
                if let Some(close_pos) = content[code_start..].find("```") {
                    let abs_close = code_start + close_pos;
                    let code_content = content[code_start..abs_close].to_string();
                    // Strip trailing newline from code content
                    let code_content = code_content.trim_end_matches('\n').to_string();
                    self.code_blocks.push((msg_idx, lang, code_content));
                    // Skip past the closing fence
                    search_from = abs_close + 3;
                } else {
                    break;
                }
            }
        }
    }

    /// Copy the code block at the given index to the system clipboard.
    pub fn yank_code_block(&mut self, idx: usize) {
        if let Some((_msg_idx, lang, content)) = self.code_blocks.get(idx) {
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(content);
                let preview: String = content.chars().take(40).collect();
                let lang_label = if lang.is_empty() { "text" } else { lang.as_str() };
                self.status_message = Some(format!(
                    "Yanked block #{} [{}]: {}{}",
                    idx + 1,
                    lang_label,
                    preview,
                    if content.len() > 40 { "..." } else { "" }
                ));
            } else {
                self.status_message = Some("Failed to access clipboard".into());
            }
        } else {
            self.status_message = Some(format!("No code block #{}", idx + 1));
        }
        self.visual_mode = false;
    }

    /// Send the code block at the given index to neovim if connected.
    pub fn send_code_to_nvim(&mut self, idx: usize) {
        if let Some((_msg_idx, lang, content)) = self.code_blocks.get(idx).cloned() {
            if let Some(ref nvim) = self.neovim {
                let ft = if lang.is_empty() { "text" } else { &lang };
                match nvim.send_to_buffer(&content, ft) {
                    Ok(()) => {
                        self.status_message = Some(format!(
                            "Sent block #{} [{}] to neovim",
                            idx + 1,
                            ft
                        ));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Neovim error: {e}"));
                    }
                }
            } else {
                self.status_message = Some("No neovim connection".into());
            }
        } else {
            self.status_message = Some(format!("No code block #{}", idx + 1));
        }
    }

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

        // Check if we should do file path completion instead of command completion
        let file_cmd_prefixes = ["/file ", "/f ", "/export "];
        for prefix in &file_cmd_prefixes {
            if self.input.starts_with(prefix) {
                self.tab_complete_path(prefix);
                return;
            }
        }

        let commands = [
            "/clear", "/new", "/model", "/models", "/provider", "/system",
            "/history", "/help", "/temp", "/save", "/nvim", "/tools", "/file",
            "/context", "/paste", "/resume", "/diff", "/export", "/theme",
            "/retry", "/edit", "/quit", "/run", "/undo", "/redo",
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

    /// Tab-complete a file path after a slash command prefix (e.g. "/file ", "/export ").
    fn tab_complete_path(&mut self, prefix: &str) {
        let partial = &self.input[prefix.len()..];
        let partial_path = std::path::Path::new(partial);

        // Determine the directory to list and the prefix to match against
        let (dir, name_prefix) = if partial.is_empty() {
            // No path typed yet - list current directory
            (std::path::PathBuf::from("."), String::new())
        } else if partial.ends_with('/') || partial.ends_with(std::path::MAIN_SEPARATOR) {
            // Trailing slash - list that directory
            (std::path::PathBuf::from(partial), String::new())
        } else {
            // Partial filename - list parent and filter by prefix
            let parent = partial_path.parent()
                .map(|p| if p.as_os_str().is_empty() { std::path::Path::new(".") } else { p })
                .unwrap_or(std::path::Path::new("."));
            let file_prefix = partial_path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent.to_path_buf(), file_prefix)
        };

        let entries = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                self.status_message = Some(format!("Cannot read directory: {}", dir.display()));
                return;
            }
        };

        let mut matches: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name_prefix.is_empty() || name.starts_with(&name_prefix) {
                // Build the full path string relative to what was typed
                let full = if partial.is_empty() {
                    name.clone()
                } else if partial.ends_with('/') || partial.ends_with(std::path::MAIN_SEPARATOR) {
                    format!("{}{}", partial, name)
                } else {
                    let parent_str = partial_path.parent()
                        .map(|p| {
                            let s = p.to_string_lossy().to_string();
                            if s.is_empty() { String::new() } else { format!("{}/", s) }
                        })
                        .unwrap_or_default();
                    format!("{}{}", parent_str, name)
                };

                // Append '/' for directories
                let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                if is_dir {
                    matches.push(format!("{}/", full));
                } else {
                    matches.push(full);
                }
            }
        }

        matches.sort();

        if matches.len() == 1 {
            self.input = format!("{}{}", prefix, matches[0]);
            self.cursor_pos = self.input.len();
        } else if matches.is_empty() {
            self.status_message = Some("No matches".into());
        } else {
            // Show options in status, limit to avoid overflow
            let display: Vec<&str> = matches.iter().map(|s| s.as_str()).take(15).collect();
            let suffix = if matches.len() > 15 {
                format!(" ... ({} total)", matches.len())
            } else {
                String::new()
            };
            self.status_message = Some(format!("{}{}", display.join("  "), suffix));

            // Auto-complete the common prefix among matches
            if let Some(common) = common_prefix(&matches) {
                if common.len() > partial.len() {
                    self.input = format!("{}{}", prefix, common);
                    self.cursor_pos = self.input.len();
                }
            }
        }
    }

    /// Clear the conversation (same as /clear command).
    pub fn clear_conversation(&mut self) {
        self.messages.clear();
        self.api_messages.clear();
        self.tool_invocations.clear();
        self.conversation = Conversation::new();
        self.status_message = Some("Conversation cleared".into());
    }

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
            self.save_and_track_conversation();
        }
        self.messages.clear();
        self.api_messages.clear();
        self.tool_invocations.clear();
        self.conversation = Conversation::new();
        self.scroll_offset = 0;
        self.status_message = Some("New conversation".into());
    }

    pub fn load_project_context(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_default();
        let dir_name = cwd.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cwd.display().to_string());

        // Get project file listing
        let file_listing = std::process::Command::new("find")
            .arg(".")
            .args([
                "-name", "*.rs",
                "-o", "-name", "*.py",
                "-o", "-name", "*.js",
                "-o", "-name", "*.ts",
                "-o", "-name", "*.go",
                "-o", "-name", "*.toml",
                "-o", "-name", "*.json",
                "-o", "-name", "*.yaml",
                "-o", "-name", "*.yml",
                "-o", "-name", "Makefile",
                "-o", "-name", "Dockerfile",
            ])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        // Take first 50 lines
        let files: String = file_listing
            .lines()
            .take(50)
            .collect::<Vec<_>>()
            .join("\n");

        let context = format!(
            "Project directory: {dir_name}\nWorking directory: {}\n\nProject files:\n{files}",
            cwd.display()
        );

        // Prepend context to the system prompt
        let existing_prompt = self.config.system_prompt.clone().unwrap_or_default();
        self.config.system_prompt = Some(format!(
            "{existing_prompt}\n\n--- Project Context ---\n{context}"
        ));

        self.status_message = Some(format!("Loaded project context for '{dir_name}'"));
    }

    pub fn paste_clipboard_as_codeblock(&mut self) {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            match clipboard.get_text() {
                Ok(text) if !text.is_empty() => {
                    let codeblock = format!("```\n{text}\n```");
                    self.input.push_str(&codeblock);
                    self.cursor_pos = self.input.len();
                    self.status_message = Some("Clipboard pasted as code block".into());
                }
                Ok(_) => {
                    self.status_message = Some("Clipboard is empty".into());
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to read clipboard: {e}"));
                }
            }
        } else {
            self.status_message = Some("Failed to access clipboard".into());
        }
    }

    /// Resolve a short model alias to its full model identifier.
    /// If the alias is not recognized, the input is returned unchanged.
    fn resolve_model_alias(alias: &str) -> String {
        match alias.trim() {
            "sonnet" | "s" => "claude-sonnet-4-20250514".into(),
            "opus" | "o" => "claude-opus-4-20250514".into(),
            "haiku" | "h" => "claude-haiku-4-5-20251001".into(),
            "gpt4" => "gpt-4o".into(),
            "gpt4m" => "gpt-4o-mini".into(),
            other => other.to_string(),
        }
    }

    pub fn load_history_list(&mut self) {
        self.history_list = Conversation::list_all().unwrap_or_default();
        self.overlay_scroll = 0;
    }

    /// Delete the currently selected conversation from the history overlay.
    pub fn delete_history_entry(&mut self) {
        if let Some(conv) = self.history_list.get(self.overlay_scroll) {
            let title = conv.title.clone();
            let id = conv.id.clone();
            if Conversation::delete(&id).is_ok() {
                self.status_message = Some(format!("Deleted conversation: {title}"));
                self.load_history_list();
                // Adjust scroll if we deleted the last item
                if self.overlay_scroll >= self.history_list.len() && self.overlay_scroll > 0 {
                    self.overlay_scroll -= 1;
                }
            } else {
                self.status_message = Some("Failed to delete conversation".into());
            }
        }
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
                self.api_messages.clear();
                self.tool_invocations.clear();
                self.conversation = Conversation::new();
            }
            "new" | "n" => self.new_conversation(),
            "help" | "h" => self.overlay = Overlay::Help,
            "history" => {
                self.overlay = Overlay::History;
                self.load_history_list();
            }
            "tools" => {
                self.tools_enabled = !self.tools_enabled;
                self.status_message = Some(format!(
                    "Tools: {}", if self.tools_enabled { "on" } else { "off" }
                ));
            }
            _ => {
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
            "tools" => {
                self.tools_enabled = !self.tools_enabled;
                self.status_message = Some(format!(
                    "Tools: {}", if self.tools_enabled { "on" } else { "off" }
                ));
            }
            _ => {
                self.status_message = Some(format!("Unknown setting: {}", parts[0]));
            }
        }
    }
}

/// Format tool arguments for display (public for use in UI).
pub fn format_tool_args_public(tool: &tools::Tool) -> String {
    format_tool_args(tool)
}

/// Format tool arguments for display.
fn format_tool_args(tool: &tools::Tool) -> String {
    match tool {
        tools::Tool::ReadFile { path } => format!("path: {path}"),
        tools::Tool::WriteFile { path, content } => {
            format!("path: {path} ({} bytes)", content.len())
        }
        tools::Tool::ListFiles { path, pattern } => {
            format!("path: {path}{}", pattern.as_deref().map(|p| format!(", pattern: {p}")).unwrap_or_default())
        }
        tools::Tool::SearchFiles { pattern, path } => {
            format!("pattern: {pattern}{}", path.as_deref().map(|p| format!(", path: {p}")).unwrap_or_default())
        }
        tools::Tool::Execute { command } => format!("$ {command}"),
        tools::Tool::EditFile { path, old_text, new_text: _ } => {
            format!("path: {path}, replacing {} chars", old_text.len())
        }
    }
}

/// Find the longest common prefix among a list of strings.
fn common_prefix(strings: &[String]) -> Option<String> {
    if strings.is_empty() {
        return None;
    }
    let first = &strings[0];
    let mut prefix_len = first.len();
    for s in &strings[1..] {
        prefix_len = prefix_len.min(s.len());
        for (i, (a, b)) in first.chars().zip(s.chars()).enumerate() {
            if i >= prefix_len || a != b {
                prefix_len = i;
                break;
            }
        }
    }
    Some(first[..prefix_len].to_string())
}
