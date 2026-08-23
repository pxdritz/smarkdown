use crate::app::{App, Mode, Prompt};
use crate::editor::{char_index_at_display_col, display_width, Buffer};
use crate::highlight::{highlight_line, highlight_line_preview};
use crate::table::{self, Table};
use crate::wrap::{ensure_visible_generic, wrap_segments};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
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
    let theme = app.config.theme.clone();
    let theme = &theme;
    let is_preview = app.mode == Mode::Preview;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.quote.resolve()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let height = inner.height as usize;
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

    if is_preview {
        app.buffer.ensure_visible(height.max(1));
        draw_preview(frame, app, inner, theme, height, total, num_width, gutter_w);
    } else if app.config.editor.word_wrap {
        draw_edit_wrapped(frame, app, inner, theme, height, num_width, show_gutter, show_margin, gutter_w);
    } else {
        app.buffer.ensure_visible(height.max(1));
        draw_edit_plain(frame, app, inner, theme, height, total, num_width, show_gutter, show_margin, gutter_w);
    }
}

fn draw_preview(
    frame: &mut Frame,
    app: &App,
    inner: Rect,
    theme: &crate::config::ThemeConfig,
    height: usize,
    total: usize,
    _num_width: u16,
    _gutter_w: u16,
) {
    let tables: Vec<Table> = table::detect_tables(&app.buffer.lines);
    let mut rendered: Vec<Line> = Vec::with_capacity(height);
    let scroll = app.buffer.scroll;
    for row in scroll..(scroll + height).min(total) {
        if let Some(t) = tables.iter().find(|t| row >= t.start && row < t.end) {
            rendered.push(table::render_row(t, row, theme));
            continue;
        }
        rendered.push(Line::from(highlight_line_preview(&app.buffer.lines[row], theme, inner.width)));
    }
    frame.render_widget(Paragraph::new(rendered), inner);
}

fn draw_edit_plain(
    frame: &mut Frame,
    app: &mut App,
    inner: Rect,
    theme: &crate::config::ThemeConfig,
    height: usize,
    total: usize,
    num_width: u16,
    show_gutter: bool,
    show_margin: bool,
    gutter_w: u16,
) {
    let text_width = inner.width.saturating_sub(gutter_w);
    app.buffer.ensure_visible_x(text_width);

    let mut rendered: Vec<Line> = Vec::with_capacity(height);
    let scroll = app.buffer.scroll;
    for row in scroll..(scroll + height).min(total) {
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
        let content_spans = if app.buffer.scroll_x > 0 {
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
    frame.render_widget(Paragraph::new(rendered), inner);

    let cursor_display_col =
        display_width(&app.buffer.lines[app.buffer.cursor_row], app.buffer.cursor_col);
    let cursor_x = inner.x + gutter_w + cursor_display_col.saturating_sub(app.buffer.scroll_x);
    let cursor_y = inner.y + (app.buffer.cursor_row - scroll) as u16;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn draw_edit_wrapped(
    frame: &mut Frame,
    app: &mut App,
    inner: Rect,
    theme: &crate::config::ThemeConfig,
    height: usize,
    num_width: u16,
    show_gutter: bool,
    show_margin: bool,
    gutter_w: u16,
) {
    app.buffer.scroll_x = 0;
    let text_width = inner.width.saturating_sub(gutter_w).max(1);

    let mut screen_rows: Vec<(usize, usize, usize)> = Vec::new();
    let mut cursor_screen_idx = 0usize;
    for (br, line) in app.buffer.lines.iter().enumerate() {
        let segments = wrap_segments(line, text_width);
        for (cs, ce) in segments {
            if br == app.buffer.cursor_row && app.buffer.cursor_col >= cs
                && (app.buffer.cursor_col < ce || ce == line.chars().count())
            {
                cursor_screen_idx = screen_rows.len();
            }
            screen_rows.push((br, cs, ce));
        }
    }

    ensure_visible_generic(&mut app.buffer.scroll, cursor_screen_idx, height.max(1));
    let scroll = app.buffer.scroll.min(screen_rows.len().saturating_sub(1));

    let mut rendered: Vec<Line> = Vec::with_capacity(height);
    for idx in scroll..(scroll + height).min(screen_rows.len()) {
        let (br, cs, ce) = screen_rows[idx];
        let is_first = cs == 0;

        let mut spans = Vec::new();
        if show_gutter {
            let label = if is_first {
                format!("{:>width$} ", br + 1, width = num_width as usize)
            } else {
                " ".repeat(num_width as usize + 1)
            };
            spans.push(Span::styled(label, Style::default().fg(theme.line_number.resolve())));
        }
        if show_margin {
            let marker = if is_first { "│ " } else { "  " };
            spans.push(Span::styled(marker, Style::default().fg(theme.margin_line.resolve())));
        }

        let segment: String = app.buffer.lines[br].chars().skip(cs).take(ce - cs).collect();
        let base = highlight_line(&segment, theme);
        let sel = selection_cols_for_row(&app.buffer, br).and_then(|(s, e)| {
            let s2 = s.max(cs);
            let e2 = e.min(ce);
            if s2 < e2 { Some((s2 - cs, e2 - cs)) } else { None }
        });
        spans.extend(apply_selection(base, sel, theme));
        rendered.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(rendered), inner);

    let (cur_br, cur_cs, _) = screen_rows.get(cursor_screen_idx).copied().unwrap_or((0, 0, 0));
    let segment: String = app.buffer.lines[cur_br].chars().skip(cur_cs).collect();
    let cursor_col_in_seg = app.buffer.cursor_col.saturating_sub(cur_cs);
    let cursor_x = inner.x + gutter_w + display_width(&segment, cursor_col_in_seg);
    let cursor_y = inner.y + (cursor_screen_idx - scroll) as u16;
    frame.set_cursor_position((cursor_x, cursor_y));
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

fn selection_cols_for_row(buffer: &Buffer, row: usize) -> Option<(usize, usize)> {
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
