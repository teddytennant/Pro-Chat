use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

// Lazy-initialize the SyntaxSet and ThemeSet so they are loaded exactly once.
static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Name of the syntect theme to use. base16-ocean.dark pairs well with Tokyo Night.
const THEME_NAME: &str = "base16-ocean.dark";

/// Default foreground color for code when no syntax is recognized (Tokyo Night foreground).
const CODE_FG: Color = Color::Rgb(169, 177, 214);
/// Border / chrome color for code block outlines.
const BORDER_COLOR: Color = Color::DarkGray;
/// Language label color inside the top border.
const LANG_LABEL_COLOR: Color = Color::Rgb(122, 162, 247);

/// Minimum visible width for code block content (excluding the "  | " prefix).
const MIN_CODE_WIDTH: usize = 40;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse markdown text into styled ratatui Lines.
/// Supports: bold, italic, code blocks (with syntax highlighting), inline code,
/// headers, lists, links.
pub fn parse_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_lang = String::new();
    let mut code_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End code block -- render the accumulated code with highlighting.
                render_code_block(&code_lang, &code_lines, &mut lines);
                code_lines.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_lang = line.trim_start_matches('`').trim().to_string();
            }
            continue;
        }

        if in_code_block {
            code_lines.push(line.to_string());
            continue;
        }

        lines.push(parse_inline(line));
    }

    // Handle unclosed code block (e.g. streaming partial response).
    if in_code_block {
        render_code_block(&code_lang, &code_lines, &mut lines);
    }

    lines
}

// ---------------------------------------------------------------------------
// Code block rendering
// ---------------------------------------------------------------------------

/// Render a fenced code block with box-drawing borders and syntax highlighting.
///
/// Output looks like:
/// ```text
///   +-  rust  -------------------------+
///   |  fn main() {                     |
///   |      println!("hello");          |
///   |  }                               |
///   +----------------------------------+
/// ```
fn render_code_block(lang: &str, code_lines: &[String], out: &mut Vec<Line<'static>>) {
    let ss = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    // Determine the content width: max of all code lines, the language label, or MIN_CODE_WIDTH.
    let label_width = if lang.is_empty() { 0 } else { lang.len() + 2 }; // " lang "
    let max_line_len = code_lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(0);
    let content_width = max_line_len.max(label_width).max(MIN_CODE_WIDTH);

    // --- Top border ---
    let top_border = if lang.is_empty() {
        let bar = "\u{2500}".repeat(content_width + 2); // +2 for padding inside box
        Line::from(Span::styled(
            format!("  \u{250c}{bar}\u{2510}"),
            Style::default().fg(BORDER_COLOR),
        ))
    } else {
        // "  +-  lang  ---...---+"
        let remaining = content_width + 2 - lang.len() - 2; // subtract " lang "
        let bar_tail = "\u{2500}".repeat(remaining);
        Line::from(vec![
            Span::styled("  \u{250c}\u{2500} ", Style::default().fg(BORDER_COLOR)),
            Span::styled(
                lang.to_string(),
                Style::default().fg(LANG_LABEL_COLOR).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {bar_tail}\u{2510}"),
                Style::default().fg(BORDER_COLOR),
            ),
        ])
    };
    out.push(top_border);

    // --- Code lines (highlighted) ---
    // Try to find a syntax definition for the declared language.
    let syntax = if lang.is_empty() {
        None
    } else {
        ss.find_syntax_by_token(lang)
    };

    let theme = ts.themes.get(THEME_NAME).unwrap_or_else(|| {
        // Fallback to the first available theme if the preferred one is missing.
        ts.themes.values().next().expect("ThemeSet has no themes")
    });

    match syntax {
        Some(syn) => {
            let mut h = HighlightLines::new(syn, theme);
            // Join lines so syntect sees the full source (needed for multi-line tokens).
            let source = code_lines.join("\n") + "\n";
            for src_line in LinesWithEndings::from(&source) {
                let ranges = h.highlight_line(src_line, ss).unwrap_or_default();
                let mut spans: Vec<Span<'static>> = Vec::new();
                spans.push(Span::styled(
                    "  \u{2502} ".to_string(),
                    Style::default().fg(BORDER_COLOR),
                ));

                let mut visible_len: usize = 0;
                for (style, fragment) in &ranges {
                    let text = fragment.trim_end_matches('\n').to_string();
                    visible_len += text.len();
                    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    let mut ratatui_style = Style::default().fg(fg);
                    if style.font_style.contains(FontStyle::BOLD) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
                    }
                    if style.font_style.contains(FontStyle::UNDERLINE) {
                        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
                    }
                    spans.push(Span::styled(text, ratatui_style));
                }

                // Pad to content_width and close the right border.
                let pad = if visible_len < content_width {
                    " ".repeat(content_width - visible_len)
                } else {
                    String::new()
                };
                spans.push(Span::styled(
                    format!("{pad} \u{2502}"),
                    Style::default().fg(BORDER_COLOR),
                ));

                out.push(Line::from(spans));
            }
        }
        None => {
            // No syntax found -- render monochrome.
            for code_line in code_lines {
                let visible_len = code_line.len();
                let pad = if visible_len < content_width {
                    " ".repeat(content_width - visible_len)
                } else {
                    String::new()
                };
                out.push(Line::from(vec![
                    Span::styled(
                        "  \u{2502} ".to_string(),
                        Style::default().fg(BORDER_COLOR),
                    ),
                    Span::styled(code_line.to_string(), Style::default().fg(CODE_FG)),
                    Span::styled(
                        format!("{pad} \u{2502}"),
                        Style::default().fg(BORDER_COLOR),
                    ),
                ]));
            }
        }
    }

    // --- Bottom border ---
    let bar = "\u{2500}".repeat(content_width + 2);
    out.push(Line::from(Span::styled(
        format!("  \u{2514}{bar}\u{2518}"),
        Style::default().fg(BORDER_COLOR),
    )));
}

// ---------------------------------------------------------------------------
// Inline markdown parsing (unchanged from original)
// ---------------------------------------------------------------------------

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
            "  \u{2022} ",
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
