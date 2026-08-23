use crate::app::{App, Mode, Prompt};
use crate::editor::{char_index_at_display_col, display_width};
use crate::highlight::{highlight_line, highlight_line_preview};
use crate::table::{self, Table};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use ratatui::Frame;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(3), Constraint::Length(1)])
        .split(frame.area());

    draw_titlebar(frame, app, root[0]);
    draw_editor(frame, app, root[1]);
    draw_statusbar(frame, app, root[2]);
}

fn draw_titlebar(frame: &mut Frame, app: &App, area: Rect) {
    let name = app
        .buffer
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("untitled");
    let dirty = if app.buffer.dirty { " ●" } else { "" };
    let mode_tag = match app.mode {
        Mode::Edit => "EDIT",
        Mode::Preview => "PREVIEW",
    };
    let title = format!(" 📓 Smarkdown — {name}{dirty}   [{mode_tag}] ");
    let line = Line::from(Span::styled(
        title,
        Style::default().fg(app.config.theme.accent.resolve()).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(Paragraph::new(line), area);
}

fn draw_editor(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = &app.config.theme;
    let is_preview = app.mode == Mode::Preview;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.quote.resolve()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let height = inner.height as usize;
    app.buffer.ensure_visible(height.max(1));

    let total = app.buffer.lines.len();
    let num_width = total.to_string().len().max(3) as u16;
    let show_gutter = app.config.editor.show_line_numbers && !is_preview;
    let show_margin = app.config.editor.notebook_margin && !is_preview;
    let mut gutter_w = 0u16;
    if show_gutter {
        gutter_w += num_width + 1;
    }
    if show_margin {
        gutter_w += 2;
    }

    app.editor_area = inner;
    app.gutter_width = gutter_w;

    let text_width = inner.width.saturating_sub(gutter_w);
    if app.config.editor.word_wrap || is_preview {
        app.buffer.scroll_x = 0;
    } else {
        app.buffer.ensure_visible_x(text_width);
    }

    let tables: Vec<Table> = if is_preview {
        table::detect_tables(&app.buffer.lines)
    } else {
        Vec::new()
    };

    let mut rendered: Vec<Line> = Vec::with_capacity(height);
    let scroll = app.buffer.scroll;
    for row in scroll..(scroll + height).min(total) {
        if let Some(t) = tables.iter().find(|t| row >= t.start && row < t.end) {
            rendered.push(table::render_row(t, row, theme));
            continue;
        }

        let mut spans = Vec::new();
        if show_gutter {
            spans.push(Span::styled(
                format!("{:>width$} ", row + 1, width = num_width as usize),
                Style::default().fg(theme.line_number.resolve()),
            ));
        }
        if show_margin {
            spans.push(Span::styled("│ ", Style::default().fg(theme.margin_line.resolve())));
        }
        let content_spans = if is_preview {
            highlight_line_preview(&app.buffer.lines[row], theme, inner.width)
        } else if app.buffer.scroll_x > 0 {
            let skip = char_index_at_display_col(&app.buffer.lines[row], app.buffer.scroll_x);
            let visible: String = app.buffer.lines[row].chars().skip(skip).collect();
            let base = highlight_line(&visible, theme);
            let sel = selection_cols_for_row(&app.buffer, row)
                .map(|(s, e)| (s.saturating_sub(skip), e.saturating_sub(skip)));
            apply_selection(base, sel, theme)
        } else {
            let base = highlight_line(&app.buffer.lines[row], theme);
            apply_selection(base, selection_cols_for_row(&app.buffer, row), theme)
        };
        spans.extend(content_spans);
        rendered.push(Line::from(spans));
    }

    let paragraph = Paragraph::new(rendered);
    let paragraph = if app.config.editor.word_wrap {
        paragraph.wrap(Wrap { trim: false })
    } else {
        paragraph
    };
    frame.render_widget(paragraph, inner);

    if !is_preview {
        let cursor_display_col =
            display_width(&app.buffer.lines[app.buffer.cursor_row], app.buffer.cursor_col);
        let cursor_x = inner.x + gutter_w + cursor_display_col.saturating_sub(app.buffer.scroll_x);
        let cursor_y = inner.y + (app.buffer.cursor_row - scroll) as u16;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = &app.config.theme;
    let style = Style::default().fg(theme.status_bar_fg.resolve()).bg(theme.status_bar_bg.resolve());

    let text = match &app.prompt {
        Prompt::Open(input) => format!(" Open file: {input}_"),
        Prompt::SaveAs(input) => format!(" Save as: {input}_"),
        Prompt::ConfirmQuit => " Discard unsaved changes? [y/N] ".to_string(),
        Prompt::None => {
            let pos = format!(
                " {}:{} ",
                app.buffer.cursor_row + 1,
                app.buffer.cursor_col + 1
            );
            let words = format!("{} words ", app.buffer.word_count());
            let hint = " ^Tab preview  ^S save  ^O open  ^N new  ^Z undo  ^Q quit ";
            let msg = app.status_msg.clone().unwrap_or_default();
            format!("{pos}│ {words}│ {msg}{hint}")
        }
    };

    frame.render_widget(Paragraph::new(Line::from(Span::styled(text, style))), area);
}

fn selection_cols_for_row(buffer: &crate::editor::Buffer, row: usize) -> Option<(usize, usize)> {
    let ((sr, sc), (er, ec)) = buffer.selection_range()?;
    if row < sr || row > er {
        return None;
    }
    let start = if row == sr { sc } else { 0 };
    let end = if row == er {
        ec
    } else {
        buffer.lines[row].chars().count()
    };
    Some((start, end))
}

fn apply_selection<'a>(
    spans: Vec<Span<'a>>,
    sel: Option<(usize, usize)>,
    theme: &crate::config::ThemeConfig,
) -> Vec<Span<'static>> {
    let Some((start, end)) = sel else {
        return spans.into_iter().map(|s| Span::styled(s.content.to_string(), s.style)).collect();
    };
    let sel_bg = theme.accent.resolve();
    let mut result = Vec::new();
    let mut col = 0usize;
    for span in spans {
        for ch in span.content.chars() {
            let style = if col >= start && col < end {
                span.style.bg(sel_bg)
            } else {
                span.style
            };
            result.push(Span::styled(ch.to_string(), style));
            col += 1;
        }
    }
    result
}
