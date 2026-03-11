# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Workspace restructure** — split into `quiver-core` (library)
  and `quiver-tui` (binary) crates
- **Visible keybinding hints** — status bar now shows
  `F1:Help` and `Ctrl+P:Commands` in accent color

### Added

- **Flight SQL client** — `FlightClient` wrapper around
  `FlightSqlServiceClient<Channel>` with profile-based connection,
  Basic/Bearer authentication, and methods covering the full Flight
  SQL surface: query execution, prepared statements, catalog
  introspection (catalogs, schemas, tables, table types, SQL info,
  primary keys, exported/imported keys, cross-reference, XDBC type
  info), and transaction management; 33 integration tests against
  an in-process mock server
- **Connection profiles** — `ConnectionProfile` with host, port, TLS,
  and `AuthMethod` (None, Basic, Bearer Token); TOML persistence via
  `ConnectionManager` (`~/.config/quiver/connections.toml`)
- **Catalog types** — `TreeNode`, `TreeNodeKind`, `FlatNode` moved to
  `quiver-core::catalog` for reuse by the data layer
- **Help popup** — press `F1` or `?` for context-aware keybinding reference overlay

## [0.1.0] - 2026-03-11

### Added

- **Multi-pane workspace** — four-quadrant layout
  (Schema Browser, Editor, Results, Context Panel)
- **Query editor** — multi-buffer SQL editor with basic syntax highlighting and tab system
- **Results viewer** — tabular result display with mock data and keyboard navigation
- **Schema browser** — catalog/schema/table/column tree with expand/collapse
- **Context panel** — switchable modes (Server Info, Query History, Connection Manager)
- **Command palette** — fuzzy-searchable command overlay via `Ctrl+P`
- **Tab system** — multiple query sessions with tab bar, `Ctrl+T`/`Ctrl+W` tab management
- **7 built-in themes** — Tokyo Night, Catppuccin Mocha, Gruvbox,
  Nord, Dracula, Solarized Dark, Rosé Pine (`Ctrl+K`)
- **3 layout presets** — Default, Wide Editor, Results Focus (cycle with `Ctrl+L`)
- **Mouse support** — click to focus panes, scroll in any pane
- **Pane zoom** — `Ctrl+Z` to maximize/restore any pane
- **Status bar** — connection status, schema context, editor mode, row count
