use std::path::PathBuf;

pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll: usize,
    pub scroll_x: u16,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub selection_anchor: Option<(usize, usize)>,
    pub undo_stack: Vec<(Vec<String>, usize, usize)>,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll: 0,
            scroll_x: 0,
            path: None,
            dirty: false,
            selection_anchor: None,
            undo_stack: Vec::new(),
        }
    }
}

impl Buffer {
    pub fn from_file(path: PathBuf) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(&path)?;
        let mut lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Ok(Self { lines, path: Some(path), ..Default::default() })
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(path) = &self.path {
            std::fs::write(path, self.lines.join("\n"))?;
            self.dirty = false;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: PathBuf) -> std::io::Result<()> {
        std::fs::write(&path, self.lines.join("\n"))?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }
}

impl Buffer {
    fn line_chars(&self, row: usize) -> Vec<char> {
        self.lines[row].chars().collect()
    }

    pub fn insert_char(&mut self, c: char) {
        let mut chars = self.line_chars(self.cursor_row);
        chars.insert(self.cursor_col, c);
        self.lines[self.cursor_row] = chars.into_iter().collect();
        self.cursor_col += 1;
        self.dirty = true;
    }

    pub fn insert_newline(&mut self) {
        let chars = self.line_chars(self.cursor_row);
        let (left, right) = chars.split_at(self.cursor_col.min(chars.len()));
        let left: String = left.iter().collect();
        let right: String = right.iter().collect();
        self.lines[self.cursor_row] = left;
        self.lines.insert(self.cursor_row + 1, right);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.dirty = true;
    }

    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let mut chars = self.line_chars(self.cursor_row);
            chars.remove(self.cursor_col - 1);
            self.lines[self.cursor_row] = chars.into_iter().collect();
            self.cursor_col -= 1;
            self.dirty = true;
        } else if self.cursor_row > 0 {
            let prev_len = self.line_chars(self.cursor_row - 1).len();
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.lines[self.cursor_row].push_str(&current);
            self.cursor_col = prev_len;
            self.dirty = true;
        }
    }

    pub fn delete_forward(&mut self) {
        let len = self.line_chars(self.cursor_row).len();
        if self.cursor_col < len {
            let mut chars = self.line_chars(self.cursor_row);
            chars.remove(self.cursor_col);
            self.lines[self.cursor_row] = chars.into_iter().collect();
            self.dirty = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
            self.dirty = true;
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.line_chars(self.cursor_row).len();
        }
    }

    pub fn move_right(&mut self) {
        let len = self.line_chars(self.cursor_row).len();
        if self.cursor_col < len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.cursor_col.min(self.line_chars(self.cursor_row).len());
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = self.cursor_col.min(self.line_chars(self.cursor_row).len());
        }
    }

    pub fn word_count(&self) -> usize {
        self.lines.iter().map(|l| l.split_whitespace().count()).sum()
    }

    pub fn ensure_visible(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.cursor_row < self.scroll {
            self.scroll = self.cursor_row;
        } else if self.cursor_row >= self.scroll + height {
            self.scroll = self.cursor_row + 1 - height;
        }
    }

    pub fn ensure_visible_x(&mut self, width: u16) {
        if width == 0 {
            return;
        }
        let cursor_col = display_width(&self.lines[self.cursor_row], self.cursor_col);
        if cursor_col < self.scroll_x {
            self.scroll_x = cursor_col;
        } else if cursor_col >= self.scroll_x + width {
            self.scroll_x = cursor_col + 1 - width;
        }
    }

    pub fn snapshot(&mut self) {
        self.undo_stack.push((self.lines.clone(), self.cursor_row, self.cursor_col));
        if self.undo_stack.len() > 200 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some((lines, row, col)) = self.undo_stack.pop() else {
            return false;
        };
        self.lines = lines;
        self.cursor_row = row.min(self.lines.len() - 1);
        self.cursor_col = col.min(self.line_chars(self.cursor_row).len());
        self.selection_anchor = None;
        self.dirty = true;
        true
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some((0, 0));
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.line_chars(self.cursor_row).len();
    }

    pub fn start_selection_if_needed(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some((self.cursor_row, self.cursor_col));
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.selection_anchor?;
        let cursor = (self.cursor_row, self.cursor_col);
        if anchor == cursor {
            return None;
        }
        if anchor <= cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    pub fn selected_text(&self) -> Option<String> {
        let ((sr, sc), (er, ec)) = self.selection_range()?;
        if sr == er {
            let chars = self.line_chars(sr);
            let sc = sc.min(chars.len());
            let ec = ec.min(chars.len());
            return Some(chars[sc..ec].iter().collect());
        }
        let mut out = String::new();
        let first = self.line_chars(sr);
        let sc = sc.min(first.len());
        out.push_str(&first[sc..].iter().collect::<String>());
        out.push('\n');
        for row in sr + 1..er {
            out.push_str(&self.lines[row]);
            out.push('\n');
        }
        let last = self.line_chars(er);
        let ec = ec.min(last.len());
        out.push_str(&last[..ec].iter().collect::<String>());
        Some(out)
    }

    pub fn delete_selection(&mut self) -> bool {
        let Some(((sr, sc), (er, ec))) = self.selection_range() else {
            return false;
        };
        if sr == er {
            let mut chars = self.line_chars(sr);
            let sc = sc.min(chars.len());
            let ec = ec.min(chars.len());
            chars.drain(sc..ec);
            self.lines[sr] = chars.into_iter().collect();
        } else {
            let first = self.line_chars(sr);
            let sc = sc.min(first.len());
            let head: String = first[..sc].iter().collect();
            let last = self.line_chars(er);
            let ec = ec.min(last.len());
            let tail: String = last[ec..].iter().collect();
            self.lines.drain(sr + 1..=er);
            self.lines[sr] = format!("{head}{tail}");
        }
        self.cursor_row = sr;
        self.cursor_col = sc;
        self.selection_anchor = None;
        self.dirty = true;
        true
    }

    pub fn insert_text(&mut self, text: &str) {
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.insert_newline();
            }
            for c in part.chars() {
                self.insert_char(c);
            }
        }
    }
}

pub fn display_width(line: &str, col: usize) -> u16 {
    use unicode_width::UnicodeWidthChar;
    line.chars()
        .take(col)
        .map(|c| c.width().unwrap_or(1) as u16)
        .sum()
}

pub fn char_index_at_display_col(line: &str, target_col: u16) -> usize {
    use unicode_width::UnicodeWidthChar;
    let mut width = 0u16;
    for (idx, c) in line.chars().enumerate() {
        let w = c.width().unwrap_or(1) as u16;
        if width + w > target_col {
            return idx;
        }
        width += w;
    }
    line.chars().count()
}
