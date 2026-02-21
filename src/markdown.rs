use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Parse markdown text into styled ratatui Lines.
/// Supports: bold, italic, code blocks, inline code, headers, lists, links.
pub fn parse_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End code block - render accumulated code
                lines.push(Line::from(Span::styled(
                    format!("  ┌─ {code_lang} "),
                    Style::default().fg(Color::DarkGray),
                )));
                for code_line in &code_lines {
                    lines.push(Line::from(Span::styled(
                        format!("  │ {code_line}"),
                        Style::default().fg(Color::Rgb(169, 177, 214)),
                    )));
                }
                lines.push(Line::from(Span::styled(
                    "  └─",
                    Style::default().fg(Color::DarkGray),
                )));
                code_lines.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_lang = line.trim_start_matches('`').to_string();
            }
            continue;
        }

        if in_code_block {
            code_lines.push(line.to_string());
            continue;
        }

        lines.push(parse_inline(line));
    }

    // Handle unclosed code block
    if in_code_block {
        lines.push(Line::from(Span::styled(
            format!("  ┌─ {code_lang} "),
            Style::default().fg(Color::DarkGray),
        )));
        for code_line in &code_lines {
            lines.push(Line::from(Span::styled(
                format!("  │ {code_line}"),
                Style::default().fg(Color::Rgb(169, 177, 214)),
            )));
        }
    }

    lines
}

fn parse_inline(line: &str) -> Line<'static> {
    let line = line.to_string();

    // Headers
    if line.starts_with("### ") {
        return Line::from(Span::styled(
            line[4..].to_string(),
            Style::default()
                .fg(Color::Rgb(122, 162, 247))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if line.starts_with("## ") {
        return Line::from(Span::styled(
            line[3..].to_string(),
            Style::default()
                .fg(Color::Rgb(122, 162, 247))
                .add_modifier(Modifier::BOLD),
        ));
    }
    if line.starts_with("# ") {
        return Line::from(Span::styled(
            line[2..].to_string(),
            Style::default()
                .fg(Color::Rgb(122, 162, 247))
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ));
    }

    // List items
    if line.starts_with("- ") || line.starts_with("* ") {
        let mut spans = vec![Span::styled(
            "  • ",
            Style::default().fg(Color::Rgb(122, 162, 247)),
        )];
        spans.extend(parse_inline_spans(&line[2..]));
        return Line::from(spans);
    }

    // Numbered lists
    if let Some(rest) = line.strip_prefix(|c: char| c.is_ascii_digit()) {
        if let Some(rest) = rest.strip_prefix(". ") {
            let num: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
            let mut spans = vec![Span::styled(
                format!("  {num}. "),
                Style::default().fg(Color::Rgb(122, 162, 247)),
            )];
            spans.extend(parse_inline_spans(rest));
            return Line::from(spans);
        }
    }

    // Regular text with inline formatting
    Line::from(parse_inline_spans(&line))
}

fn parse_inline_spans(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text.to_string();

    while !remaining.is_empty() {
        // Inline code
        if let Some(start) = remaining.find('`') {
            if let Some(end) = remaining[start + 1..].find('`') {
                if start > 0 {
                    spans.push(Span::raw(remaining[..start].to_string()));
                }
                let code = &remaining[start + 1..start + 1 + end];
                spans.push(Span::styled(
                    code.to_string(),
                    Style::default()
                        .fg(Color::Rgb(224, 175, 104))
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = remaining[start + 1 + end + 1..].to_string();
                continue;
            }
        }

        // Bold **text**
        if let Some(start) = remaining.find("**") {
            if let Some(end) = remaining[start + 2..].find("**") {
                if start > 0 {
                    spans.push(Span::raw(remaining[..start].to_string()));
                }
                let bold_text = &remaining[start + 2..start + 2 + end];
                spans.push(Span::styled(
                    bold_text.to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                remaining = remaining[start + 2 + end + 2..].to_string();
                continue;
            }
        }

        // Italic *text*
        if let Some(start) = remaining.find('*') {
            if let Some(end) = remaining[start + 1..].find('*') {
                if start > 0 {
                    spans.push(Span::raw(remaining[..start].to_string()));
                }
                let italic_text = &remaining[start + 1..start + 1 + end];
                spans.push(Span::styled(
                    italic_text.to_string(),
                    Style::default().add_modifier(Modifier::ITALIC),
                ));
                remaining = remaining[start + 1 + end + 1..].to_string();
                continue;
            }
        }

        // No more formatting
        spans.push(Span::raw(remaining.clone()));
        break;
    }

    if spans.is_empty() {
        spans.push(Span::raw(text.to_string()));
    }

    spans
}
