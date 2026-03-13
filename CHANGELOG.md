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
- **Default context panel** — starts on Connection Manager
  instead of Server Info for quicker access to saved profiles

### Added

- **Async bridge** — `CoreHandle` spawns a background tokio runtime
  connected to the TUI event loop via mpsc channels; supports
  Connect, Disconnect, ExecuteQuery, CancelQuery, RefreshSchema,
  and TestConnection requests
- **Live query execution** — press `Ctrl+E` to run the
  editor contents against the connected Flight SQL server; results
  stored as native Arrow `RecordBatch` data and rendered with
  type-aware formatting in the results pane
- **Query cancellation** — `CancellationToken` integration via
  `tokio::select!`; press `Ctrl+Shift+C` to cancel a running query
- **Connection dialog** — `Ctrl+O` opens an inline connect popup
  with Name, Host, Port, TLS toggle, and Auth method selector
  (None / Basic / Bearer Token); `Ctrl+D` disconnects
- **Test Connection** — `Ctrl+T` or `[ Test Connection ]` button
  in the dialog tests connectivity with inline ✓/✗ feedback
  without closing the dialog
- **Connection timeouts** — configurable `connect_timeout_secs`
  (default 10s) applied to the tonic Endpoint; collapsible
  Advanced section in dialog with Timeout and Max Retries fields
- **Connection retry** — `max_retries` setting with 500ms delay
  between attempts on connect failure
- **Interactive Connection Manager** — context panel lists saved
  profiles with selection highlight (j/k/↑/↓), green dot for
  active connection; Enter to connect, `e` to edit, `x` to delete
- **Edit connection profiles** — press `e` on a saved profile
  to re-open the connection dialog pre-filled with all settings
- **Command palette commands** — Connect, Disconnect, Execute
  Query, Cancel Query, and Refresh Schema added to `Ctrl+P`
- **Status bar connection hint** — shows "No Connection (Ctrl+O)"
  when disconnected for discoverability
- **Schema refresh** — `Ctrl+R` fetches catalogs, schemas, tables,
  and columns from the server and rebuilds the schema browser tree
- **Schema introspection helpers** — `extract_tables()`,
  `extract_columns()`, `build_schema_tree()` in `quiver-core::catalog`
  with 6 unit tests
- **Flight SQL client** — `FlightClient` wrapper around
  `FlightSqlServiceClient<Channel>` with profile-based connection,
  Basic/Bearer authentication, and methods covering the full Flight
  SQL surface: query execution, prepared statements, catalog
  introspection (catalogs, schemas, tables, table types, SQL info,
  primary keys, exported/imported keys, cross-reference, XDBC type
  info), and transaction management; 33 integration tests against
  an in-process mock server
- **Connection profiles** — `ConnectionProfile` with host, port, TLS,
  `AuthMethod` (None, Basic, Bearer Token), connect timeout, and
  max retries; TOML persistence via `ConnectionManager`
  (`~/.config/quiver/connections.toml`)
- **Catalog types** — `TreeNode`, `TreeNodeKind`, `FlatNode` moved to
  `quiver-core::catalog` for reuse by the data layer
- **Columnar-native results** — query results stored internally as
  `Vec<RecordBatch>` with full `SchemaRef`; no string conversion at
  the data layer boundary. Only visible rows (~30-60) are formatted
  per frame via virtual scrolling with batch-aware row resolution
- **Type-aware cell formatting** — cells formatted directly from
  Arrow arrays: booleans render as ✓/✗, NULLs styled distinctly,
  all other types use Arrow's native display. Formatting is
  on-demand during draw, not pre-computed
- **Real type badges** — column headers show compact Arrow type
  badges derived from the actual schema (`i64`, `f64`, `utf8`,
  `ts[μs,UTC]`, `dec(38,18)`, `list<i32>`, `§utf8` for
  dictionary-encoded, etc.) with nullable columns marked `?`.
  Badge colors follow type families: integers=cyan, floats=green,
  strings=yellow, temporal=magenta, booleans=blue, binary=red,
  nested=orange
- **Error modal** — query and connection errors now open a modal
  overlay showing operation name, error message (word-wrapped),
  and elapsed time; press Esc to dismiss. Replaces the transient
  3-second notification for errors
- **Query elapsed time** — execution duration is tracked for both
  successful and failed queries; displayed in the status bar
  (e.g. `123.4ms` or `1.23s`) and inside the error modal on failure.
  The bridge sends `elapsed: Option<Duration>` on `CoreResponse::Error`
- **Export results** — query results can be exported to CSV, JSON,
  or Apache Parquet files from the command palette or via `Ctrl+S`
  (CSV). Export uses Arrow's native writers — CSV via `arrow::csv`,
  JSON via `arrow::json`, Parquet via the `parquet` crate with
  Snappy compression. Export module in `quiver-core::export` with
  6 unit tests
- **Copy to clipboard** — `Ctrl+Shift+S` copies results as CSV to
  the system clipboard using the OSC 52 terminal escape sequence
  (supported by most modern terminals including iTerm2, Kitty,
  Alacritty, WezTerm, Windows Terminal)
- **Help popup** — press `F1` or `?` for context-aware keybinding reference overlay
- **Example** — `test_connect` example in quiver-core for quick
  connection validation (`cargo run --example test_connect -p quiver-core`)

### Fixed

- **Tick polling bug** — main event loop now dispatches `Tick` events
  when no input is received, so async responses (connection status,
  query results, schema loads) are actually processed; previously
  the `None` branch was a no-op, meaning the UI would never update
  after connecting or running a query until a key was pressed
- **Keybinding compatibility** — replaced terminal-dependent shortcuts
  (F5, Ctrl+Enter, Shift+Enter) with universal `Ctrl+E` for query
  execution; enabled Kitty keyboard protocol for better modifier
  key detection; Tab no longer conflicts with SQL indentation
  in the editor pane

### Removed

- **Placeholder data** — removed `placeholder_schema_tree()`,
  `load_placeholder_results()`, and hardcoded connection profiles;
  all panes now start empty and populate from the live server

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
