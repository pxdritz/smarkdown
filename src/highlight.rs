use crate::config::ThemeConfig;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

pub fn highlight_line<'a>(line: &'a str, theme: &ThemeConfig) -> Vec<Span<'a>> {
    let fg = theme.fg.resolve();
    let base = Style::default().fg(fg);

    if line.trim_start().starts_with('#') {
        return vec![Span::styled(
            line,
            Style::default().fg(theme.heading.resolve()).add_modifier(Modifier::BOLD),
        )];
    }
    if line.trim_start().starts_with('>') {
        return vec![Span::styled(
            line,
            Style::default().fg(theme.quote.resolve()).add_modifier(Modifier::ITALIC),
        )];
    }

    highlight_inline(line, theme, base, true)
}

pub fn highlight_line_preview(line: &str, theme: &ThemeConfig) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    let fg = theme.fg.resolve();
    let base = Style::default().fg(fg);

    if let Some(rest) = trimmed.strip_prefix("###") {
        return vec![owned(rest.trim(), Style::default().fg(theme.heading.resolve()).add_modifier(Modifier::BOLD))];
    }
    if let Some(rest) = trimmed.strip_prefix("##") {
        return vec![owned(rest.trim(), Style::default().fg(theme.heading.resolve()).add_modifier(Modifier::BOLD | Modifier::UNDERLINED))];
    }
    if let Some(rest) = trimmed.strip_prefix('#') {
        return vec![owned(
            &format!("» {}", rest.trim()),
            Style::default().fg(theme.heading.resolve()).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )];
    }
    if let Some(rest) = trimmed.strip_prefix('>') {
        return vec![owned(
            &format!("▏ {}", rest.trim_start()),
            Style::default().fg(theme.quote.resolve()).add_modifier(Modifier::ITALIC),
        )];
    }
    if let Some(rest) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
        let mut spans = vec![owned("  • ", Style::default().fg(theme.accent.resolve()))];
        spans.extend(
            highlight_inline(rest, theme, base, false)
                .into_iter()
                .map(|s| owned(&s.content, s.style)),
        );
        return spans;
    }

    highlight_inline(line, theme, base, false)
        .into_iter()
        .map(|s| owned(&s.content, s.style))
        .collect()
}

fn owned(text: &str, style: Style) -> Span<'static> {
    Span::styled(text.to_string(), style)
}

fn highlight_inline<'a>(
    line: &'a str,
    theme: &ThemeConfig,
    base: Style,
    keep_markers: bool,
) -> Vec<Span<'a>> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0usize;
    let mut start = 0usize;

    macro_rules! flush_plain {
        ($end:expr) => {
            if $end > start {
                spans.push(Span::styled(&line[start..$end], base));
            }
        };
    }

    while i < bytes.len() {
        if line[i..].starts_with("**") {
            if let Some(end) = line[i + 2..].find("**") {
                flush_plain!(i);
                let full = &line[i..i + 2 + end + 2];
                let text = if keep_markers { full } else { &full[2..full.len() - 2] };
                spans.push(Span::styled(
                    text,
                    Style::default().fg(theme.bold.resolve()).add_modifier(Modifier::BOLD),
                ));
                i += 2 + end + 2;
                start = i;
                continue;
            }
        }
        if bytes[i] == b'[' {
            if let Some(bracket_end) = line[i + 1..].find(']') {
                let text_end = i + 1 + bracket_end;
                if bytes.get(text_end + 1) == Some(&b'(') {
                    if let Some(paren_len) = line[text_end + 2..].find(')') {
                        let paren_end = text_end + 2 + paren_len;
                        flush_plain!(i);
                        let link_text = &line[i + 1..text_end];
                        let full = &line[i..=paren_end];
                        let text = if keep_markers { full } else { link_text };
                        spans.push(Span::styled(
                            text,
                            Style::default().fg(theme.link.resolve()).add_modifier(Modifier::UNDERLINED),
                        ));
                        i = paren_end + 1;
                        start = i;
                        continue;
                    }
                }
            }
        }
        if bytes[i] == b'`' {
            if let Some(end) = line[i + 1..].find('`') {
                flush_plain!(i);
                let full = &line[i..i + 1 + end + 1];
                let text = if keep_markers { full } else { &full[1..full.len() - 1] };
                spans.push(Span::styled(
                    text,
                    Style::default().fg(theme.code.resolve()).bg(theme.code_bg.resolve()),
                ));
                i += 1 + end + 1;
                start = i;
                continue;
            }
        }

        if (bytes[i] == b'*' || bytes[i] == b'_') && !line[i..].starts_with("**") {
            let marker = bytes[i] as char;
            if let Some(end) = line[i + 1..].find(marker) {
                flush_plain!(i);
                let full = &line[i..i + 1 + end + 1];
                let text = if keep_markers { full } else { &full[1..full.len() - 1] };
                spans.push(Span::styled(
                    text,
                    Style::default().fg(theme.italic.resolve()).add_modifier(Modifier::ITALIC),
                ));
                i += 1 + end + 1;
                start = i;
                continue;
            }
        }
        let step = line[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += step;
    }
    flush_plain!(line.len());
    if spans.is_empty() {
        spans.push(Span::styled(line, base));
    }
    spans
}
