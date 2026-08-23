## Smarkdown

A simple, lightweight terminal markdown notepad.

Built with `ratatui` and `crossterm`, running fully inside your terminal
and inheriting its color scheme instead of forcing its own.

### Features

- Terminal-native TUI, resizes with your terminal window
- Follows your terminal's own ANSI color palette by default
- Notebook-style layout: line numbers and a red margin rule
- Inline markdown highlighting while editing (`#`, `**bold**`, `*italic*`, `` `code` ``, `> quote`)
- Preview mode with rendered markdown, toggled with `Ctrl+Tab`
- Markdown tables and hyperlinks, rendered nicely in preview
- Click a hyperlink to open it in your default browser
- Text selection (`Ctrl+A`, `Shift+Arrows`), copy/cut/paste through the system clipboard
- Toggleable word wrap (`Shift+Q`)
- Auto-closing pairs for `()`, `[]`, `{}`, `"`, `'`, `` ` ``, `*`, `_`
- Configuration through a TOML file, no recompiling needed

### Build

```
cargo install smarkdown
```

### Run

```
smarkdown path/to/file.md
```

Running without an argument opens an empty, unsaved note.

### Keybindings

| Key | Action |
| --- | --- |
| `Ctrl+S` | Save |
| `Ctrl+O` | Open file |
| `Ctrl+N` | New note |
| `Ctrl+Q` | Quit |
| `Ctrl+Tab` | Toggle Edit / Preview mode |
| `Shift+Q` | Toggle word wrap |
| `Ctrl+A` | Select all |
| `Shift+Arrows` | Extend selection |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste |
| Click on a link (Edit mode) | Open it in your browser |

`Ctrl+Tab` requires a terminal that supports the Kitty keyboard protocol
(Alacritty, kitty, WezTerm, and others). On terminals without it, plain
`Tab` may be sent instead.

### Configuration

On first run, Smarkdown creates `~/.config/smarkdown/config.toml`.

Colors accept ANSI names (`red`, `cyan`, `darkgray`, `reset`, ...), or
`rgb:R,G,B` / `indexed:N` if you want to bypass the terminal's palette.

```toml
[theme]
fg = "reset"
bg = "reset"
accent = "cyan"
heading = "magenta"
bold = "yellow"
italic = "magenta"
code = "green"
code_bg = "darkgray"
quote = "darkgray"
link = "blue"
line_number = "darkgray"
margin_line = "red"
status_bar_fg = "black"
status_bar_bg = "cyan"

[editor]
show_line_numbers = true
notebook_margin = true
margin_column = 4
tab_spaces = 4
word_wrap = true
highlight_current_line = true
```

### License

MIT.
