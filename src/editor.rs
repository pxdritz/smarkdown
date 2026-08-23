use std::path::PathBuf;

pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll: usize,
    pub path: Option<PathBuf>,
    pub dirty: bool,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll: 0,
            path: None,
            dirty: false,
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
