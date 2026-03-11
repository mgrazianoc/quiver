# quiver — Arrow Flight SQL TUI Client

**The definitive interactive client for Arrow Flight SQL servers.**

![status](https://img.shields.io/badge/status-v0.2--dev-blue)
[![CI](https://github.com/mgrazianoc/quiver/actions/workflows/ci.yml/badge.svg)](https://github.com/mgrazianoc/quiver/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

## What is this?

`quiver` is a terminal-based SQL client purpose-built for the
[Arrow Flight SQL][flight-sql] protocol.
Unlike generic SQL tools that treat results as bags of strings,
quiver preserves Arrow's columnar typed semantics all the way
to the rendering layer.

**Current state:** v0.2-dev — the full UI shell is implemented
(editor, result viewer, command palette, theming, mouse support).
The project is a Cargo workspace with `quiver-core`
(connection/data layer) and `quiver-tui` (terminal UI).
The Flight SQL client wrapper covers the complete command surface;
async bridge and TUI integration are next.

[flight-sql]: https://arrow.apache.org/docs/format/FlightSql.html

## Building

```bash
# Requires Rust 1.75+
cargo build --release

# Run
cargo run --release -p quiver-tui

# Or install
cargo install --path crates/quiver-tui
```

## Quick Start

```bash
quiver                           # Launch TUI
quiver --theme gruvbox           # Launch with specific theme (future)
quiver -c 'SELECT 1' --conn dev # Non-interactive mode (future)
```

## Keybindings

### Global

| Key | Action |
| --- | --- |
| `Ctrl+Q` | Quit |
| `Ctrl+P` | Command palette |
| `F1` / `?` | Keybinding help popup |
| `Ctrl+1/2/3/4` | Focus pane (Schema / Editor / Results / Context) |
| `Ctrl+Z` | Toggle zoom on focused pane |
| `Tab` / `Shift+Tab` | Cycle pane focus |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Alt+Left/Right` | Switch tab |
| `Alt+1-9` | Jump to tab N |
| `Ctrl+L` | Cycle layout preset |
| `Ctrl+K` | Cycle theme |
| `Ctrl+J` | Cycle context panel mode |

### Editor

Standard text editing: arrow keys, Home/End, Backspace, Delete, Enter, Tab (4 spaces).

### Results Viewer

| Key | Action |
| --- | --- |
| `j/k` or `↑/↓` | Navigate rows |
| `h/l` or `←/→` | Scroll columns |
| `g` / `G` | Jump to first / last row |
| `PageUp/Down` | Page through results |

### Schema Browser

| Key | Action |
| --- | --- |
| `j/k` or `↑/↓` | Navigate tree |
| `Enter` / `→` | Expand node |
| `←` | Collapse node |

## Themes

7 built-in themes, cycle with `Ctrl+K`:

- Tokyo Night (default)
- Catppuccin Mocha
- Gruvbox
- Nord
- Dracula
- Solarized Dark
- Rosé Pine

Custom themes via TOML in `~/.config/quiver/themes/` (coming soon).

## Layout Presets

Cycle with `Ctrl+L`:

- **Default** — 4-pane grid (schema | editor over context | results)
- **Wide Editor** — full-width editor on top, 3 panes below
- **Results Focus** — large results pane, sidebar with schema + editor + context

## Terminal Compatibility

Tested and designed for:

- **macOS**: iTerm2, Terminal.app, Alacritty, Kitty, WezTerm
- **Linux**: gnome-terminal, Alacritty, Kitty, WezTerm, foot, xterm (256-color fallback)
- **Multiplexers**: tmux, zellij (mouse passthrough recommended)

Requires:

- Truecolor support recommended (256-color fallback planned)
- UTF-8 locale
- Minimum 80×24 terminal size (120×40+ recommended)

## Project Structure

```text
Cargo.toml                         # Workspace root
crates/
├── quiver-core/                   # Library: connection management, config, data layer
│   └── src/
│       ├── lib.rs
│       ├── catalog.rs             # TreeNode / FlatNode schema tree types
│       ├── client.rs              # Flight SQL client wrapper (full command surface)
│       ├── config.rs              # TOML configuration loading
│       └── connection.rs          # Connection profiles, auth, and manager
└── quiver-tui/                    # Binary: terminal UI
    └── src/
        ├── main.rs                # Entry point, terminal setup, event loop
        ├── app.rs                 # Application state, event dispatch
        ├── event.rs               # Crossterm event reader
        ├── keybindings/mod.rs     # Key mode detection and mapping
        ├── theme/mod.rs           # Theme definitions (7 built-in)
        └── ui/
            ├── mod.rs             # Layout computation, render dispatch
            ├── tabs.rs            # Tab bar rendering
            ├── statusbar.rs       # Status bar rendering
            ├── command_palette.rs # Fuzzy-searchable command overlay
            ├── help.rs            # Context-aware keybinding help popup
            └── panes/
                ├── editor.rs      # SQL editor with syntax highlighting
                ├── results.rs     # Tabular result viewer
                ├── schema_browser.rs  # Catalog tree browser
                └── context_panel.rs   # Switchable info panels
```

## Roadmap

- [ ] **v0.2** — Flight SQL connection (arrow-flight, tonic, tokio)
- [ ] **v0.3** — Real RecordBatch rendering, catalog population from server
- [ ] **v0.4** — Query execution, streaming results, export (Parquet/CSV/IPC)
- [ ] **v0.5** — DataFusion local analytics engine
- [ ] **v0.6** — Vim/Emacs keybinding modes, tree-sitter highlighting
- [ ] **v0.7** — Server compliance & conformance testing (§16)
- [ ] **v1.0** — Full feature spec implementation

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards, and PR guidelines.

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
