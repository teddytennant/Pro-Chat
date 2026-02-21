use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, InputMode, Overlay};
use crate::markdown;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_frame(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick as usize / 2) % SPINNER_FRAMES.len()]
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Main layout: messages area + input + status bar
    let line_count = app.input.lines().count()
        + if app.input.ends_with('\n') { 1 } else { 0 };
    let input_height = (line_count + 2).min(10) as u16;
    let input_height = input_height.max(3);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),              // Messages
            Constraint::Length(input_height), // Input
            Constraint::Length(1),            // Status bar
        ])
        .split(area);

    draw_messages(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    // Draw overlay if active
    match &app.overlay {
        Overlay::Help => draw_help_overlay(f, area),
        Overlay::History => draw_history_overlay(f, app, area),
        Overlay::Settings => draw_settings_overlay(f, app, area),
        Overlay::ToolConfirm => draw_tool_confirm_overlay(f, app, area),
        Overlay::None => {}
    }
}

fn draw_messages(f: &mut Frame, app: &mut App, area: Rect) {
    let messages_block = Block::default()
        .borders(Borders::NONE)
        .padding(Padding::horizontal(1));

    let inner = messages_block.inner(area);
    f.render_widget(messages_block, area);

    if app.messages.is_empty() {
        // Welcome screen
        let banner_style = Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD);
        let welcome = vec![
            Line::from(""),
            Line::from(""),
            Line::from(Span::styled("██████╗ ██████╗  ██████╗ ",  banner_style)),
            Line::from(Span::styled("██╔══██╗██╔══██╗██╔═══██╗", banner_style)),
            Line::from(Span::styled("██████╔╝██████╔╝██║   ██║", banner_style)),
            Line::from(Span::styled("██╔═══╝ ██╔══██╗██║   ██║", banner_style)),
            Line::from(Span::styled("██║     ██║  ██║╚██████╔╝", banner_style)),
            Line::from(Span::styled("╚═╝     ╚═╝  ╚═╝ ╚═════╝", banner_style)),
            Line::from(""),
            Line::from(Span::styled(
                "Fast AI chat in your terminal",
                Style::default().fg(Color::Rgb(86, 95, 137)),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Type a message to start • ? for help • :q to quit",
                Style::default().fg(Color::Rgb(59, 66, 97)),
            )),
        ];
        let p = Paragraph::new(welcome).alignment(Alignment::Center);
        f.render_widget(p, inner);
        return;
    }

    // Build rendered lines from messages
    let mut all_lines: Vec<Line> = Vec::new();
    let width = inner.width as usize;

    for msg in &app.messages {
        // Role header
        let (label, color) = match msg.role.as_str() {
            "user" => ("You", Color::Rgb(158, 206, 106)),
            "assistant" => ("Assistant", Color::Rgb(187, 154, 247)),
            _ => ("System", Color::Rgb(86, 95, 137)),
        };

        all_lines.push(Line::from(""));
        all_lines.push(Line::from(Span::styled(
            format!("  {label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));

        // Message content
        if msg.role == "assistant" {
            let parsed = markdown::parse_markdown(&msg.content);
            for line in parsed {
                all_lines.push(line);
            }
        } else {
            // User messages - plain text with wrapping
            for line in msg.content.lines() {
                if line.len() > width.saturating_sub(4) {
                    // Simple word wrap
                    let mut current = String::from("  ");
                    for word in line.split_whitespace() {
                        if current.len() + word.len() + 1 > width.saturating_sub(2) {
                            all_lines.push(Line::from(current.clone()));
                            current = format!("  {word}");
                        } else {
                            if current.len() > 2 {
                                current.push(' ');
                            }
                            current.push_str(word);
                        }
                    }
                    if !current.trim().is_empty() {
                        all_lines.push(Line::from(current));
                    }
                } else {
                    all_lines.push(Line::from(format!("  {line}")));
                }
            }
        }

        // Tool invocations
        for inv in &msg.tool_invocations {
            all_lines.push(Line::from(""));
            let status_icon = match &inv.result {
                Some(r) if r.success => "✓",
                Some(_) => "✗",
                None => "…",
            };
            let status_color = match &inv.result {
                Some(r) if r.success => Color::Rgb(158, 206, 106),
                Some(_) => Color::Rgb(247, 118, 142),
                None => Color::Rgb(224, 175, 104),
            };
            all_lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{status_icon} "),
                    Style::default().fg(status_color),
                ),
                Span::styled(
                    inv.tool_name.clone(),
                    Style::default().fg(Color::Rgb(224, 175, 104)).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  {}", inv.tool_args),
                    Style::default().fg(Color::Rgb(86, 95, 137)),
                ),
            ]));

            if let Some(ref result) = inv.result {
                if !inv.collapsed {
                    // Show result (truncated if long)
                    let output_lines: Vec<&str> = result.output.lines().collect();
                    let max_lines = 15;
                    let show_lines = if output_lines.len() > max_lines {
                        &output_lines[..max_lines]
                    } else {
                        &output_lines
                    };
                    for ol in show_lines {
                        all_lines.push(Line::from(Span::styled(
                            format!("    {ol}"),
                            Style::default().fg(Color::Rgb(86, 95, 137)),
                        )));
                    }
                    if output_lines.len() > max_lines {
                        all_lines.push(Line::from(Span::styled(
                            format!("    ... ({} more lines)", output_lines.len() - max_lines),
                            Style::default().fg(Color::Rgb(59, 66, 97)),
                        )));
                    }
                } else {
                    all_lines.push(Line::from(Span::styled(
                        format!("    ({} lines collapsed)", result.output.lines().count()),
                        Style::default().fg(Color::Rgb(59, 66, 97)),
                    )));
                }
            }
        }

        // Streaming indicator with spinner
        if msg.role == "assistant" && app.streaming {
            let frame = spinner_frame(app.tick_count);
            if msg.content.is_empty() && msg.tool_invocations.is_empty() {
                all_lines.push(Line::from(Span::styled(
                    format!("  {frame} "),
                    Style::default().fg(Color::Rgb(187, 154, 247)),
                )));
            } else if !msg.content.is_empty() {
                // Append spinner to the last line of streaming text
                if let Some(last_line) = all_lines.last_mut() {
                    let mut spans: Vec<Span> = last_line.spans.clone();
                    spans.push(Span::styled(
                        format!(" {frame}"),
                        Style::default().fg(Color::Rgb(187, 154, 247)),
                    ));
                    *last_line = Line::from(spans);
                }
            }
        }
    }

    // Handle scrolling
    let total_lines = all_lines.len();
    let visible = inner.height as usize;

    let max_scroll = total_lines.saturating_sub(visible);
    if app.scroll_offset > max_scroll {
        app.scroll_offset = max_scroll;
    }

    let p = Paragraph::new(all_lines)
        .scroll((app.scroll_offset as u16, 0));
    f.render_widget(p, inner);

    // Scroll indicator
    if total_lines > visible {
        let scrollbar_area = Rect::new(
            area.x + area.width - 1,
            area.y,
            1,
            area.height,
        );
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .style(Style::default().fg(Color::Rgb(59, 66, 97)));
        let mut state = ScrollbarState::new(total_lines)
            .position(app.scroll_offset);
        f.render_stateful_widget(scrollbar, scrollbar_area, &mut state);
    }
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled(" NOR ", Style::default().bg(Color::Rgb(122, 162, 247)).fg(Color::Rgb(26, 27, 38)).add_modifier(Modifier::BOLD)),
        InputMode::Insert => Span::styled(" INS ", Style::default().bg(Color::Rgb(158, 206, 106)).fg(Color::Rgb(26, 27, 38)).add_modifier(Modifier::BOLD)),
        InputMode::Command => Span::styled(" CMD ", Style::default().bg(Color::Rgb(224, 175, 104)).fg(Color::Rgb(26, 27, 38)).add_modifier(Modifier::BOLD)),
        InputMode::Search => Span::styled(" SRC ", Style::default().bg(Color::Rgb(247, 118, 142)).fg(Color::Rgb(26, 27, 38)).add_modifier(Modifier::BOLD)),
    };

    // Build right-side title spans
    let line_count = app.input.lines().count();
    let has_trailing_newline = app.input.ends_with('\n');
    let effective_lines = if has_trailing_newline { line_count + 1 } else { line_count };
    let mut right_title_spans: Vec<Span> = Vec::new();
    if effective_lines > 1 {
        right_title_spans.push(Span::styled(
            format!(" [{} lines] ", effective_lines),
            Style::default().fg(Color::Rgb(86, 95, 137)),
        ));
    }
    if app.streaming {
        let frame = spinner_frame(app.tick_count);
        right_title_spans.push(Span::styled(
            format!(" {frame} streaming... "),
            Style::default().fg(Color::Rgb(187, 154, 247)).add_modifier(Modifier::ITALIC),
        ));
    }

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(match app.input_mode {
            InputMode::Normal => Color::Rgb(59, 66, 97),
            InputMode::Insert => Color::Rgb(122, 162, 247),
            InputMode::Command => Color::Rgb(224, 175, 104),
            InputMode::Search => Color::Rgb(247, 118, 142),
        }))
        .border_type(BorderType::Rounded)
        .title(Line::from(mode_indicator).alignment(Alignment::Left))
        .title(Line::from(right_title_spans).alignment(Alignment::Right));

    let display_text = if app.input_mode == InputMode::Command {
        format!(":{}", app.command_input)
    } else if app.input_mode == InputMode::Search {
        format!("/{}", app.search_query)
    } else if app.input.is_empty() {
        match app.input_mode {
            InputMode::Insert => "Type a message... (Enter to send, Shift+Enter for newline)".to_string(),
            _ => String::new(),
        }
    } else {
        app.input.clone()
    };

    let style = if app.input.is_empty() && app.input_mode == InputMode::Insert {
        Style::default().fg(Color::Rgb(86, 95, 137))
    } else {
        Style::default().fg(Color::Rgb(192, 202, 245))
    };

    let input_paragraph = Paragraph::new(display_text)
        .style(style)
        .block(input_block);

    f.render_widget(input_paragraph, area);

    // Cursor position
    if app.input_mode == InputMode::Insert || app.input_mode == InputMode::Command || app.input_mode == InputMode::Search {
        let cursor_x = if app.input_mode == InputMode::Command {
            area.x + 2 + app.command_input.len() as u16
        } else if app.input_mode == InputMode::Search {
            area.x + 2 + app.search_query.len() as u16
        } else {
            let current_line_start = app.input[..app.cursor_pos]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            area.x + 1 + (app.cursor_pos - current_line_start) as u16
        };
        let cursor_line = if app.input_mode == InputMode::Command || app.input_mode == InputMode::Search {
            0
        } else {
            app.input[..app.cursor_pos].matches('\n').count()
        };
        let cursor_y = area.y + 1 + cursor_line as u16;
        if cursor_x < area.x + area.width - 1 && cursor_y < area.y + area.height - 1 {
            f.set_cursor_position(Position::new(cursor_x, cursor_y));
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = vec![
        Span::styled(
            format!(" {} ", app.config.provider),
            Style::default().fg(Color::Rgb(86, 95, 137)),
        ),
        Span::styled("│", Style::default().fg(Color::Rgb(59, 66, 97))),
        Span::styled(
            format!(" {} ", app.config.model),
            Style::default().fg(Color::Rgb(86, 95, 137)),
        ),
    ];

    // Tools status
    if app.tools_enabled {
        spans.push(Span::styled("│", Style::default().fg(Color::Rgb(59, 66, 97))));
        spans.push(Span::styled(
            " tools ",
            Style::default().fg(Color::Rgb(158, 206, 106)),
        ));
    }

    // Neovim status
    if let Some(ref nvim) = app.neovim {
        spans.push(Span::styled("│", Style::default().fg(Color::Rgb(59, 66, 97))));
        spans.push(Span::styled(
            if nvim.is_connected() { "  nvim " } else { "  nvim ✗ " },
            Style::default().fg(if nvim.is_connected() {
                Color::Rgb(158, 206, 106)
            } else {
                Color::Rgb(247, 118, 142)
            }),
        ));
    }

    // Status message or default hints
    if let Some(ref msg) = app.status_message {
        spans.push(Span::styled("│", Style::default().fg(Color::Rgb(59, 66, 97))));
        spans.push(Span::styled(
            format!(" {msg} "),
            Style::default().fg(Color::Rgb(224, 175, 104)),
        ));
    }

    // Right side: message count and conversation size
    let total_chars: usize = app.messages.iter().map(|m| m.content.len()).sum();
    let size_display = if total_chars >= 1000 {
        format!("{:.1}k", total_chars as f64 / 1000.0)
    } else {
        format!("{}", total_chars)
    };
    let msg_count = app.messages.iter().filter(|m| m.role == "user").count();
    let right_text = format!(" {size_display} chars | {msg_count}msgs ");

    let left = Line::from(spans);
    let right = Span::styled(right_text, Style::default().fg(Color::Rgb(86, 95, 137)));

    let bar = Paragraph::new(left)
        .style(Style::default().bg(Color::Rgb(26, 27, 38)));
    f.render_widget(bar, area);

    // Right-aligned text
    let right_width = right.width() as u16;
    if area.width > right_width {
        let right_area = Rect::new(
            area.x + area.width - right_width,
            area.y,
            right_width,
            1,
        );
        let right_p = Paragraph::new(Line::from(right))
            .style(Style::default().bg(Color::Rgb(26, 27, 38)));
        f.render_widget(right_p, right_area);
    }
}

fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let overlay_area = centered_rect(60, 80, area);
    f.render_widget(Clear, overlay_area);

    let help_text = vec![
        Line::from(Span::styled("Pro Chat — Keyboard Reference", Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Normal Mode", Style::default().fg(Color::Rgb(187, 154, 247)).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("  i/a/A/I/o    Enter insert mode")),
        Line::from(Span::raw("  :            Enter command mode")),
        Line::from(Span::raw("  j/k          Scroll messages")),
        Line::from(Span::raw("  Ctrl+d/u     Half-page scroll")),
        Line::from(Span::raw("  G/gg         Bottom/top")),
        Line::from(Span::raw("  h/l          Cursor left/right")),
        Line::from(Span::raw("  w/b          Word forward/back")),
        Line::from(Span::raw("  0/$          Line start/end")),
        Line::from(Span::raw("  x            Delete char")),
        Line::from(Span::raw("  dd           Clear input")),
        Line::from(Span::raw("  y            Copy last response")),
        Line::from(Span::raw("  Ctrl+y       Extract code blocks (1-9 to yank)")),
        Line::from(Span::raw("  Ctrl+e       Send last code block to nvim")),
        Line::from(Span::raw("  p            Paste from clipboard")),
        Line::from(Span::raw("  ?            This help")),
        Line::from(Span::raw("  /            Search messages")),
        Line::from(Span::raw("  n/N          Next/prev match")),
        Line::from(Span::raw("  Ctrl+h       History")),
        Line::from(Span::raw("  Ctrl+n       New conversation")),
        Line::from(""),
        Line::from(Span::styled("Insert Mode", Style::default().fg(Color::Rgb(158, 206, 106)).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("  Enter        Send message")),
        Line::from(Span::raw("  Shift+Enter  New line")),
        Line::from(Span::raw("  Esc          Normal mode")),
        Line::from(Span::raw("  Ctrl+w       Delete word")),
        Line::from(Span::raw("  Ctrl+u       Delete to start")),
        Line::from(Span::raw("  Tab          Autocomplete /cmd")),
        Line::from(Span::raw("  Up/Down      Input history")),
        Line::from(""),
        Line::from(Span::styled("Commands", Style::default().fg(Color::Rgb(224, 175, 104)).add_modifier(Modifier::BOLD))),
        Line::from(Span::raw("  /clear       Clear conversation")),
        Line::from(Span::raw("  /new         New conversation")),
        Line::from(Span::raw("  /model <m>   Set model")),
        Line::from(Span::raw("  /provider    Set provider")),
        Line::from(Span::raw("  /system      Set system prompt")),
        Line::from(Span::raw("  /temp <t>    Set temperature")),
        Line::from(Span::raw("  /history     Browse history")),
        Line::from(Span::raw("  /nvim        Connect neovim")),
        Line::from(Span::raw("  /file <p>    Load file into input")),
        Line::from(Span::raw("  /save        Save config")),
        Line::from(Span::raw("  /quit        Quit")),
        Line::from(""),
        Line::from(Span::styled("  Press Esc or q to close", Style::default().fg(Color::Rgb(86, 95, 137)))),
    ];

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(59, 66, 97)))
                .title(Line::from(Span::styled(
                    " Help ",
                    Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD),
                )))
                .style(Style::default().bg(Color::Rgb(22, 22, 30))),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(help, overlay_area);
}

fn draw_history_overlay(f: &mut Frame, app: &App, area: Rect) {
    let overlay_area = centered_rect(60, 70, area);
    f.render_widget(Clear, overlay_area);

    let items: Vec<ListItem> = app.history_list.iter().enumerate().map(|(i, conv)| {
        let style = if i == app.overlay_scroll {
            Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Rgb(192, 202, 245))
        };
        let prefix = if i == app.overlay_scroll { "▸ " } else { "  " };
        let date = conv.updated_at.format("%Y-%m-%d %H:%M");
        ListItem::new(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(conv.title.chars().take(40).collect::<String>(), style),
            Span::styled(format!("  {date}"), Style::default().fg(Color::Rgb(86, 95, 137))),
        ]))
    }).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(59, 66, 97)))
                .title(Line::from(Span::styled(
                    " History ",
                    Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD),
                )))
                .style(Style::default().bg(Color::Rgb(22, 22, 30))),
        );

    f.render_widget(list, overlay_area);
}

fn draw_settings_overlay(f: &mut Frame, app: &App, area: Rect) {
    let overlay_area = centered_rect(50, 50, area);
    f.render_widget(Clear, overlay_area);

    let settings = vec![
        Line::from(Span::styled("Settings", Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(format!("  Provider:    {}", app.config.provider)),
        Line::from(format!("  Model:       {}", app.config.model)),
        Line::from(format!("  Temperature: {}", app.config.temperature)),
        Line::from(format!("  Max tokens:  {}", app.config.max_tokens)),
        Line::from(format!("  Vim mode:    {}", app.config.vim_mode)),
        Line::from(""),
        Line::from(format!("  Config: {}", crate::config::Config::path().display())),
    ];

    let p = Paragraph::new(settings)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(59, 66, 97)))
                .style(Style::default().bg(Color::Rgb(22, 22, 30))),
        );

    f.render_widget(p, overlay_area);
}

fn draw_tool_confirm_overlay(f: &mut Frame, app: &App, area: Rect) {
    let overlay_area = centered_rect(60, 40, area);
    f.render_widget(Clear, overlay_area);

    let call = match app.pending_tool_calls.get(app.pending_tool_confirm_idx) {
        Some(c) => c,
        None => return,
    };

    let tool_name = call.tool.name();
    let tool_args = crate::app::format_tool_args_public(&call.tool);

    let lines = vec![
        Line::from(Span::styled(
            "Tool Execution Request",
            Style::default().fg(Color::Rgb(224, 175, 104)).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tool: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(
                tool_name.to_string(),
                Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Args: ", Style::default().fg(Color::Rgb(86, 95, 137))),
            Span::styled(tool_args, Style::default().fg(Color::Rgb(192, 202, 245))),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "  ({}/{})",
                app.pending_tool_confirm_idx + 1,
                app.pending_tool_calls.len()
            ),
            Style::default().fg(Color::Rgb(86, 95, 137)),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y] ", Style::default().fg(Color::Rgb(158, 206, 106)).add_modifier(Modifier::BOLD)),
            Span::styled("Allow  ", Style::default().fg(Color::Rgb(192, 202, 245))),
            Span::styled("[a] ", Style::default().fg(Color::Rgb(122, 162, 247)).add_modifier(Modifier::BOLD)),
            Span::styled("Always  ", Style::default().fg(Color::Rgb(192, 202, 245))),
            Span::styled("[n] ", Style::default().fg(Color::Rgb(247, 118, 142)).add_modifier(Modifier::BOLD)),
            Span::styled("Deny  ", Style::default().fg(Color::Rgb(192, 202, 245))),
            Span::styled("[d] ", Style::default().fg(Color::Rgb(247, 118, 142)).add_modifier(Modifier::BOLD)),
            Span::styled("Deny all", Style::default().fg(Color::Rgb(192, 202, 245))),
        ]),
    ];

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Rgb(224, 175, 104)))
                .title(Line::from(Span::styled(
                    " Confirm ",
                    Style::default().fg(Color::Rgb(224, 175, 104)).add_modifier(Modifier::BOLD),
                )))
                .style(Style::default().bg(Color::Rgb(22, 22, 30))),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(p, overlay_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
