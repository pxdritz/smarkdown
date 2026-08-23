use crate::config::ThemeConfig;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

pub struct Table {
    pub start: usize,
    pub end: usize,
    pub rows: Vec<Vec<String>>,
    pub widths: Vec<usize>,
}

pub fn detect_tables(lines: &[String]) -> Vec<Table> {
    let mut tables = Vec::new();
    let mut i = 0;
    while i + 1 < lines.len() {
        if is_table_row(&lines[i]) && is_separator_row(&lines[i + 1]) {
            let start = i;
            let mut j = i + 2;
            while j < lines.len() && is_table_row(&lines[j]) {
                j += 1;
            }

            let mut rows = Vec::new();
            rows.push(split_cells(&lines[start]));
            for line in &lines[start + 2..j] {
                rows.push(split_cells(line));
            }

            let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
            let mut widths = vec![3usize; col_count];
            for row in &rows {
                for (idx, cell) in row.iter().enumerate() {
                    widths[idx] = widths[idx].max(cell.chars().count());
                }
            }

            tables.push(Table { start, end: j, rows, widths });
            i = j;
        } else {
            i += 1;
        }
    }
    tables
}

fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && t.contains('|')
}

fn is_separator_row(line: &str) -> bool {
    let t = line.trim();
    t.contains('-') && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn split_cells(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

pub fn render_row(table: &Table, buffer_line: usize, theme: &ThemeConfig) -> Line<'static> {
    if buffer_line == table.start {
        return render_cells(&table.rows[0], &table.widths, theme, true);
    }
    if buffer_line == table.start + 1 {
        return render_divider(&table.widths, theme);
    }
    let data_idx = buffer_line - table.start - 1;
    render_cells(&table.rows[data_idx], &table.widths, theme, false)
}

fn render_cells(cells: &[String], widths: &[usize], theme: &ThemeConfig, header: bool) -> Line<'static> {
    let border = Style::default().fg(theme.quote.resolve());
    let style = if header {
        Style::default().fg(theme.heading.resolve()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg.resolve())
    };

    let mut spans = vec![Span::styled("│ ", border)];
    for (idx, width) in widths.iter().enumerate() {
        let cell = cells.get(idx).map(|s| s.as_str()).unwrap_or("");
        let pad = width.saturating_sub(cell.chars().count());
        spans.push(Span::styled(format!("{cell}{}", " ".repeat(pad)), style));
        spans.push(Span::styled(" │ ", border));
    }
    Line::from(spans)
}

fn render_divider(widths: &[usize], theme: &ThemeConfig) -> Line<'static> {
    let border = Style::default().fg(theme.quote.resolve());
    let mut spans = vec![Span::styled("├", border)];
    for (idx, width) in widths.iter().enumerate() {
        spans.push(Span::styled("─".repeat(width + 2), border));
        let joint = if idx + 1 == widths.len() { "┤" } else { "┼" };
        spans.push(Span::styled(joint, border));
    }
    Line::from(spans)
}
