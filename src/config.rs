use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ColorSpec(pub String);

impl ColorSpec {
    pub fn resolve(&self) -> Color {
        let s = self.0.trim().to_lowercase();
        if let Some(rest) = s.strip_prefix("indexed:") {
            if let Ok(n) = rest.trim().parse::<u8>() {
                return Color::Indexed(n);
            }
        }
        if let Some(rest) = s.strip_prefix("rgb:") {
            let parts: Vec<_> = rest.split(',').map(|p| p.trim().parse::<u8>()).collect();
            if let [Ok(r), Ok(g), Ok(b)] = parts[..] {
                return Color::Rgb(r, g, b);
            }
        }
        match s.as_str() {
            "reset" | "" => Color::Reset,
            "black" => Color::Black,
            "red" => Color::Red,
            "green" => Color::Green,
            "yellow" => Color::Yellow,
            "blue" => Color::Blue,
            "magenta" => Color::Magenta,
            "cyan" => Color::Cyan,
            "gray" | "grey" => Color::Gray,
            "darkgray" | "darkgrey" => Color::DarkGray,
            "lightred" => Color::LightRed,
            "lightgreen" => Color::LightGreen,
            "lightyellow" => Color::LightYellow,
            "lightblue" => Color::LightBlue,
            "lightmagenta" => Color::LightMagenta,
            "lightcyan" => Color::LightCyan,
            "white" => Color::White,
            _ => Color::Reset,
        }
    }
}

fn s(v: &str) -> ColorSpec {
    ColorSpec(v.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub fg: ColorSpec,
    pub bg: ColorSpec,
    pub accent: ColorSpec,
    pub heading: ColorSpec,
    pub bold: ColorSpec,
    pub italic: ColorSpec,
    pub code: ColorSpec,
    pub code_bg: ColorSpec,
    pub quote: ColorSpec,
    pub link: ColorSpec,
    pub line_number: ColorSpec,
    pub margin_line: ColorSpec,
    pub status_bar_fg: ColorSpec,
    pub status_bar_bg: ColorSpec,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            fg: s("reset"),
            bg: s("reset"),
            accent: s("cyan"),
            heading: s("magenta"),
            bold: s("yellow"),
            italic: s("magenta"),
            code: s("green"),
            code_bg: s("darkgray"),
            quote: s("darkgray"),
            link: s("blue"),
            line_number: s("darkgray"),
            margin_line: s("red"),
            status_bar_fg: s("black"),
            status_bar_bg: s("cyan"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub show_line_numbers: bool,
    pub notebook_margin: bool,
    pub margin_column: u16,
    pub tab_spaces: usize,
    pub word_wrap: bool,
    pub highlight_current_line: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            show_line_numbers: true,
            notebook_margin: true,
            margin_column: 4,
            tab_spaces: 4,
            word_wrap: true,
            highlight_current_line: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub theme: ThemeConfig,
    pub editor: EditorConfig,
}

impl Config {
    pub fn config_path() -> PathBuf {
        let mut dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        dir.push("smarkdown");
        std::fs::create_dir_all(&dir).ok();
        dir.push("config.toml");
        dir
    }

    pub fn load_or_create() -> Self {
        let path = Self::config_path();
        if let Ok(text) = std::fs::read_to_string(&path) {
            match toml::from_str::<Config>(&text) {
                Ok(cfg) => return cfg,
                Err(e) => eprintln!("smarkdown: error reading config.toml ({e}), using default"),
            }
        }
        let cfg = Config::default();
        cfg.save();
        cfg
    }

    pub fn save(&self) {
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(Self::config_path(), text);
        }
    }
}
