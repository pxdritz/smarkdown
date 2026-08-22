use crate::config::Config;
use crate::editor::Buffer;

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

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

impl App {
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
            _ => {}
        }

        match self.mode {
            Mode::Edit => self.handle_edit_key(key, ctrl),
            Mode::Preview => self.handle_preview_key(key),
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent, ctrl: bool) {
        match key.code {
            KeyCode::Char('o') if ctrl => self.prompt = Prompt::Open(String::new()),
            KeyCode::Char('n') if ctrl => {
                self.buffer = Buffer::default();
                self.set_status("New note");
            }
            KeyCode::Char(c) => self.handle_char_input(c),
            KeyCode::Enter => self.buffer.insert_newline(),
            KeyCode::Backspace => self.handle_backspace(),
            KeyCode::Delete => self.buffer.delete_forward(),
            KeyCode::Left => self.buffer.move_left(),
            KeyCode::Right => self.buffer.move_right(),
            KeyCode::Up => self.buffer.move_up(),
            KeyCode::Down => self.buffer.move_down(),
            KeyCode::Home => self.buffer.cursor_col = 0,
            KeyCode::End => {
                self.buffer.cursor_col = self.buffer.lines[self.buffer.cursor_row].chars().count()
            }
            KeyCode::Tab => {
                for _ in 0..self.config.editor.tab_spaces {
                    self.buffer.insert_char(' ');
                }
            }
            _ => {}
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
