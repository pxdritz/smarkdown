use crate::config::Config;
use crate::editor::Buffer;
use ratatui::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Edit,
    Preview,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Prompt {
    None,
    Open(String),
    SaveAs(String),
    ConfirmQuit,
}

pub struct App {
    pub config: Config,
    pub buffer: Buffer,
    pub mode: Mode,
    pub prompt: Prompt,
    pub status_msg: Option<String>,
    pub should_quit: bool,
    pub editor_area: Rect,
    pub gutter_width: u16,
}

impl App {
    pub fn new(config: Config, buffer: Buffer) -> Self {
        Self {
            config,
            buffer,
            mode: Mode::Edit,
            prompt: Prompt::None,
            status_msg: None,
            should_quit: false,
            editor_area: Rect::default(),
            gutter_width: 0,
        }
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Edit => Mode::Preview,
            Mode::Preview => Mode::Edit,
        };
    }
}

use crate::editor::char_index_at_display_col;
use crate::link;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use std::path::PathBuf;

impl App {
    pub fn handle_mouse(&mut self, mouse: MouseEvent) {
        if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
            return;
        }
        if self.mode != Mode::Edit || self.prompt != Prompt::None {
            return;
        }
        let area = self.editor_area;
        if mouse.column < area.x
            || mouse.column >= area.x + area.width
            || mouse.row < area.y
            || mouse.row >= area.y + area.height
        {
            return;
        }
        let rel_col = mouse.column - area.x;
        let rel_row = (mouse.row - area.y) as usize;
        let row = self.buffer.scroll + rel_row;
        if row >= self.buffer.lines.len() || rel_col < self.gutter_width {
            return;
        }
        let text_col = rel_col - self.gutter_width;
        let line = &self.buffer.lines[row];
        let char_idx = char_index_at_display_col(line, text_col);
        if let Some(url) = link::find_link_at(line, char_idx) {
            link::open_url(&url);
            self.set_status(format!("Opened: {url}"));
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.prompt != Prompt::None {
            self.handle_prompt_key(key);
            return;
        }

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Tab if ctrl => {
                self.toggle_mode();
                return;
            }
            KeyCode::Char('s') if ctrl => {
                self.save_or_prompt();
                return;
            }
            KeyCode::Char('q') if ctrl => {
                if self.buffer.dirty {
                    self.prompt = Prompt::ConfirmQuit;
                } else {
                    self.should_quit = true;
                }
                return;
            }
            KeyCode::Char('Q') if key.modifiers.contains(KeyModifiers::SHIFT) && !ctrl => {
                self.config.editor.word_wrap = !self.config.editor.word_wrap;
                self.config.save();
                let state = if self.config.editor.word_wrap { "on" } else { "off" };
                self.set_status(format!("Word wrap: {state}"));
                return;
            }
            _ => {}
        }

        match self.mode {
            Mode::Edit => self.handle_edit_key(key, ctrl),
            Mode::Preview => self.handle_preview_key(key),
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent, ctrl: bool) {
        let shift = key.modifiers.contains(KeyModifiers::SHIFT);
        match key.code {
            KeyCode::Char('o') if ctrl => self.prompt = Prompt::Open(String::new()),
            KeyCode::Char('n') if ctrl => {
                self.buffer = Buffer::default();
                self.set_status("New note");
            }
            KeyCode::Char('a') if ctrl => self.buffer.select_all(),
            KeyCode::Char('c') if ctrl => self.copy_selection(),
            KeyCode::Char('x') if ctrl => self.cut_selection(),
            KeyCode::Char('v') if ctrl => self.paste_clipboard(),
            KeyCode::Char(c) if ctrl => {
                let _ = c;
            }
            KeyCode::Char(c) => {
                self.buffer.delete_selection();
                self.handle_char_input(c);
            }
            KeyCode::Enter => {
                self.buffer.delete_selection();
                self.buffer.insert_newline();
            }
            KeyCode::Backspace => {
                if !self.buffer.delete_selection() {
                    self.handle_backspace();
                }
            }
            KeyCode::Delete => {
                if !self.buffer.delete_selection() {
                    self.buffer.delete_forward();
                }
            }
            KeyCode::Left => {
                self.selecting(shift);
                self.buffer.move_left();
            }
            KeyCode::Right => {
                self.selecting(shift);
                self.buffer.move_right();
            }
            KeyCode::Up => {
                self.selecting(shift);
                self.buffer.move_up();
            }
            KeyCode::Down => {
                self.selecting(shift);
                self.buffer.move_down();
            }
            KeyCode::Home => {
                self.buffer.clear_selection();
                self.buffer.cursor_col = 0;
            }
            KeyCode::End => {
                self.buffer.clear_selection();
                self.buffer.cursor_col = self.buffer.lines[self.buffer.cursor_row].chars().count()
            }
            KeyCode::Tab => {
                self.buffer.delete_selection();
                for _ in 0..self.config.editor.tab_spaces {
                    self.buffer.insert_char(' ');
                }
            }
            _ => {}
        }
    }

    fn selecting(&mut self, shift: bool) {
        if shift {
            self.buffer.start_selection_if_needed();
        } else {
            self.buffer.clear_selection();
        }
    }

    fn copy_selection(&mut self) {
        let Some(text) = self.buffer.selected_text() else {
            self.set_status("Nothing selected");
            return;
        };
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
            Ok(_) => self.set_status("Copied."),
            Err(e) => self.set_status(format!("Clipboard error: {e}")),
        }
    }

    fn cut_selection(&mut self) {
        let Some(text) = self.buffer.selected_text() else {
            self.set_status("Nothing selected");
            return;
        };
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text)) {
            Ok(_) => {
                self.buffer.delete_selection();
                self.set_status("Cut.");
            }
            Err(e) => self.set_status(format!("Clipboard error: {e}")),
        }
    }

    fn paste_clipboard(&mut self) {
        match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
            Ok(text) => {
                self.buffer.delete_selection();
                self.buffer.insert_text(&text);
                self.set_status("Pasted.");
            }
            Err(e) => self.set_status(format!("Clipboard error: {e}")),
        }
    }

    fn handle_char_input(&mut self, c: char) {
        let line = &self.buffer.lines[self.buffer.cursor_row];
        let next_char = line.chars().nth(self.buffer.cursor_col);

        if (is_closer(c) || is_symmetric(c)) && next_char == Some(c) {
            self.buffer.move_right();
            return;
        }
        if let Some(closer) = matching_closer(c) {
            self.buffer.insert_char(c);
            self.buffer.insert_char(closer);
            self.buffer.move_left();
            return;
        }
        self.buffer.insert_char(c);
    }

    fn handle_backspace(&mut self) {
        let row = self.buffer.cursor_row;
        let col = self.buffer.cursor_col;
        if col > 0 {
            let chars: Vec<char> = self.buffer.lines[row].chars().collect();
            let prev = chars[col - 1];
            let next = chars.get(col).copied();
            let is_empty_pair = match prev {
                '(' | '[' | '{' | '"' | '\'' | '`' | '*' | '_' => {
                    next == matching_closer(prev).or(Some(prev))
                }
                _ => false,
            };
            if is_empty_pair {
                self.buffer.delete_forward();
            }
        }
        self.buffer.backspace();
    }


    fn handle_preview_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => {
                if self.buffer.scroll > 0 {
                    self.buffer.scroll -= 1;
                }
            }
            KeyCode::Down => self.buffer.scroll += 1,
            KeyCode::PageUp => {
                self.buffer.scroll = self.buffer.scroll.saturating_sub(10)
            }
            KeyCode::PageDown => self.buffer.scroll += 10,
            _ => {}
        }
    }

    fn save_or_prompt(&mut self) {
        if self.buffer.path.is_some() {
            match self.buffer.save() {
                Ok(_) => self.set_status("Saved."),
                Err(e) => self.set_status(format!("Error saving: {e}")),
            }
        } else {
            self.prompt = Prompt::SaveAs(String::new());
        }
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) {
        match &mut self.prompt {
            Prompt::Open(input) | Prompt::SaveAs(input) => match key.code {
                KeyCode::Char(c) => input.push(c),
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Esc => self.prompt = Prompt::None,
                KeyCode::Enter => {
                    let path = PathBuf::from(input.trim());
                    let is_open = matches!(self.prompt, Prompt::Open(_));
                    self.prompt = Prompt::None;
                    if is_open {
                        match Buffer::from_file(path) {
                            Ok(buf) => {
                                self.buffer = buf;
                                self.set_status("File opened.");
                            }
                            Err(e) => self.set_status(format!("Error opening: {e}")),
                        }
                    } else {
                        match self.buffer.save_as(path) {
                            Ok(_) => self.set_status("Saved."),
                            Err(e) => self.set_status(format!("Error saving: {e}")),
                        }
                    }
                }
                _ => {}
            },
            Prompt::ConfirmQuit => match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => self.should_quit = true,
                _ => self.prompt = Prompt::None,
            },
            Prompt::None => {}
        }
    }
}

fn matching_closer(c: char) -> Option<char> {
    match c {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '"' => Some('"'),
        '\'' => Some('\''),
        '`' => Some('`'),
        '*' => Some('*'),
        '_' => Some('_'),
        _ => None,
    }
}

fn is_closer(c: char) -> bool {
    matches!(c, ')' | ']' | '}')
}

fn is_symmetric(c: char) -> bool {
    matches!(c, '"' | '\'' | '`' | '*' | '_')
}
