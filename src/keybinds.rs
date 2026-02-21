use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, InputMode, Overlay};

/// Result of handling a key event
pub enum KeyAction {
    /// Nothing happened
    None,
    /// App should quit
    Quit,
    /// Input was consumed
    Consumed,
    /// Send the current message
    SendMessage,
    /// Cancel current streaming
    CancelStream,
    /// Retry/regenerate last assistant response
    RetryMessage,
    /// Edit last user message
    EditLastMessage,
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> KeyAction {
    // Global keybinds that work in any mode
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
            if app.is_streaming() {
                return KeyAction::CancelStream;
            }
            return KeyAction::Quit;
        }
        (KeyModifiers::CONTROL, KeyCode::Char('q')) => return KeyAction::Quit,
        _ => {}
    }

    // Handle overlays first
    if app.overlay != Overlay::None {
        return handle_overlay_key(app, key);
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Insert => handle_insert_mode(app, key),
        InputMode::Command => handle_command_mode(app, key),
        InputMode::Search => handle_search_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        // Mode switching
        (KeyModifiers::NONE, KeyCode::Char('i')) => {
            app.input_mode = InputMode::Insert;
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('a')) => {
            app.input_mode = InputMode::Insert;
            app.cursor_right();
            KeyAction::Consumed
        }
        (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
            app.input_mode = InputMode::Insert;
            app.cursor_end();
            KeyAction::Consumed
        }
        (KeyModifiers::SHIFT, KeyCode::Char('I')) => {
            app.input_mode = InputMode::Insert;
            app.cursor_home();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('o')) => {
            app.input_mode = InputMode::Insert;
            app.cursor_end();
            app.insert_newline();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char(':')) => {
            app.input_mode = InputMode::Command;
            app.command_input.clear();
            KeyAction::Consumed
        }

        // Navigation
        (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.scroll_down(1);
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.scroll_up(1);
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
            app.scroll_down(app.visible_height() / 2);
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.scroll_up(app.visible_height() / 2);
            KeyAction::Consumed
        }
        (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
            app.scroll_to_bottom();
            KeyAction::Consumed
        }
        (_, KeyCode::Char('g')) => {
            // gg to top - simplified: single g goes to top
            app.scroll_to_top();
            KeyAction::Consumed
        }

        // Text movement in input
        (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
            app.cursor_left();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
            app.cursor_right();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('w')) => {
            app.cursor_word_forward();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('b')) => {
            app.cursor_word_back();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('0')) => {
            app.cursor_home();
            KeyAction::Consumed
        }
        (KeyModifiers::SHIFT, KeyCode::Char('$')) => {
            app.cursor_end();
            KeyAction::Consumed
        }

        // Editing
        (KeyModifiers::NONE, KeyCode::Char('x')) => {
            app.delete_char_at_cursor();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('d')) => {
            // dd clears line - simplified: single d clears
            app.clear_input();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('p')) => {
            app.paste_clipboard();
            KeyAction::Consumed
        }

        // Overlays
        (KeyModifiers::NONE, KeyCode::Char('?')) => {
            app.overlay = Overlay::Help;
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
            app.overlay = Overlay::History;
            app.load_history_list();
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
            app.new_conversation();
            KeyAction::Consumed
        }

        // Search
        (KeyModifiers::NONE, KeyCode::Char('/')) => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Char('n')) => {
            app.next_search_match();
            KeyAction::Consumed
        }
        (KeyModifiers::SHIFT, KeyCode::Char('N')) => {
            app.prev_search_match();
            KeyAction::Consumed
        }

        // Retry/regenerate last response
        (KeyModifiers::CONTROL, KeyCode::Char('r')) => {
            return KeyAction::RetryMessage;
        }

        // Edit last user message (only when input is empty to avoid conflicts)
        (KeyModifiers::NONE, KeyCode::Char('e')) if app.input.is_empty() => {
            return KeyAction::EditLastMessage;
        }

        // Yank (copy) last response
        (KeyModifiers::NONE, KeyCode::Char('y')) => {
            app.yank_last_response();
            KeyAction::Consumed
        }

        // Extract code blocks and enter visual selection mode
        (KeyModifiers::CONTROL, KeyCode::Char('y')) => {
            app.extract_code_blocks();
            if app.code_blocks.is_empty() {
                app.status_message = Some("No code blocks found".into());
            } else {
                app.visual_mode = true;
                let summary: Vec<String> = app.code_blocks.iter().enumerate().map(|(i, (_, lang, content))| {
                    let lang_label = if lang.is_empty() { "text" } else { lang.as_str() };
                    let preview: String = content.lines().next().unwrap_or("").chars().take(30).collect();
                    format!("[{}] {} {}", i + 1, lang_label, preview)
                }).collect();
                app.status_message = Some(format!("Code blocks: {}", summary.join(" | ")));
            }
            KeyAction::Consumed
        }

        // Send last code block to neovim
        (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            app.extract_code_blocks();
            if app.code_blocks.is_empty() {
                app.status_message = Some("No code blocks to send".into());
            } else {
                let last_idx = app.code_blocks.len() - 1;
                app.send_code_to_nvim(last_idx);
            }
            KeyAction::Consumed
        }

        // Number keys 1-9 yank code block when in visual mode
        (KeyModifiers::NONE, KeyCode::Char(c @ '1'..='9')) if app.visual_mode => {
            let idx = (c as usize) - ('1' as usize);
            app.yank_code_block(idx);
            KeyAction::Consumed
        }

        _ => KeyAction::None,
    }
}

fn handle_insert_mode(app: &mut App, key: KeyEvent) -> KeyAction {
    match (key.modifiers, key.code) {
        // Escape to normal mode
        (KeyModifiers::NONE, KeyCode::Esc) => {
            app.input_mode = InputMode::Normal;
            KeyAction::Consumed
        }

        // Send message
        (KeyModifiers::NONE, KeyCode::Enter) => {
            if app.input.trim().is_empty() {
                KeyAction::None
            } else {
                KeyAction::SendMessage
            }
        }

        // Newline
        (KeyModifiers::SHIFT, KeyCode::Enter) | (KeyModifiers::ALT, KeyCode::Enter) => {
            app.insert_newline();
            KeyAction::Consumed
        }

        // Basic editing
        (KeyModifiers::NONE, KeyCode::Backspace) | (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
            app.delete_char_before_cursor();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Delete) => {
            app.delete_char_at_cursor();
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
            app.delete_word_before_cursor();
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
            app.delete_to_start();
            KeyAction::Consumed
        }

        // Cursor movement
        (KeyModifiers::NONE, KeyCode::Left) => {
            app.cursor_left();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Right) => {
            app.cursor_right();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
            app.cursor_home();
            KeyAction::Consumed
        }
        (KeyModifiers::NONE, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
            app.cursor_end();
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('p')) | (KeyModifiers::NONE, KeyCode::Up) => {
            app.history_prev();
            KeyAction::Consumed
        }
        (KeyModifiers::CONTROL, KeyCode::Char('n')) | (KeyModifiers::NONE, KeyCode::Down) => {
            app.history_next();
            KeyAction::Consumed
        }

        // Tab completion (for slash commands)
        (KeyModifiers::NONE, KeyCode::Tab) => {
            app.tab_complete();
            KeyAction::Consumed
        }

        // Type characters
        (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            app.insert_char(c);
            KeyAction::Consumed
        }

        _ => KeyAction::None,
    }
}

fn handle_command_mode(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.command_input.clear();
            KeyAction::Consumed
        }
        KeyCode::Enter => {
            let cmd = app.command_input.clone();
            app.input_mode = InputMode::Normal;
            app.command_input.clear();
            app.execute_command(&cmd);
            KeyAction::Consumed
        }
        KeyCode::Backspace => {
            app.command_input.pop();
            if app.command_input.is_empty() {
                app.input_mode = InputMode::Normal;
            }
            KeyAction::Consumed
        }
        KeyCode::Char(c) => {
            app.command_input.push(c);
            KeyAction::Consumed
        }
        _ => KeyAction::None,
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.search_query.clear();
            KeyAction::Consumed
        }
        KeyCode::Enter => {
            app.input_mode = InputMode::Normal;
            app.execute_search();
            KeyAction::Consumed
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            if app.search_query.is_empty() {
                app.input_mode = InputMode::Normal;
            }
            KeyAction::Consumed
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            KeyAction::Consumed
        }
        _ => KeyAction::None,
    }
}

fn handle_overlay_key(app: &mut App, key: KeyEvent) -> KeyAction {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.overlay = Overlay::None;
            KeyAction::Consumed
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.overlay_scroll_down();
            KeyAction::Consumed
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.overlay_scroll_up();
            KeyAction::Consumed
        }
        KeyCode::Enter => {
            app.overlay_select();
            KeyAction::Consumed
        }
        _ => KeyAction::None,
    }
}
