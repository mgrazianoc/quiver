use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::event::AppEvent;
use crate::keybindings::KeyMode;
use crate::theme::{Theme, ThemeKind};
use crate::ui::command_palette::CommandEntry;

// ── Pane identifiers ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pane {
    SchemaBrowser,
    Editor,
    Results,
    ContextPanel,
}

impl Pane {
    pub const ALL: [Pane; 4] = [
        Pane::SchemaBrowser,
        Pane::Editor,
        Pane::Results,
        Pane::ContextPanel,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Pane::SchemaBrowser => "Schema Browser",
            Pane::Editor => "Editor",
            Pane::Results => "Results",
            Pane::ContextPanel => "Context",
        }
    }
}

// ── Layout presets ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    Default,
    WideEditor,
    ResultsFocus,
}

impl LayoutPreset {
    pub fn label(&self) -> &'static str {
        match self {
            LayoutPreset::Default => "Default (4-pane)",
            LayoutPreset::WideEditor => "Wide Editor",
            LayoutPreset::ResultsFocus => "Results Focus",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            LayoutPreset::Default => LayoutPreset::WideEditor,
            LayoutPreset::WideEditor => LayoutPreset::ResultsFocus,
            LayoutPreset::ResultsFocus => LayoutPreset::Default,
        }
    }
}

// ── Context panel modes ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMode {
    ServerInfo,
    QueryHistory,
    ConnectionManager,
    StreamMonitor,
}

impl ContextMode {
    pub fn label(&self) -> &'static str {
        match self {
            ContextMode::ServerInfo => "Server Info",
            ContextMode::QueryHistory => "History",
            ContextMode::ConnectionManager => "Connections",
            ContextMode::StreamMonitor => "Stream Monitor",
        }
    }

    pub fn cycle_next(&self) -> Self {
        match self {
            ContextMode::ServerInfo => ContextMode::QueryHistory,
            ContextMode::QueryHistory => ContextMode::ConnectionManager,
            ContextMode::ConnectionManager => ContextMode::StreamMonitor,
            ContextMode::StreamMonitor => ContextMode::ServerInfo,
        }
    }
}

// ── Query tab ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QueryTab {
    pub id: usize,
    pub title: String,
    pub content: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub pinned: bool,
    pub state: TabState,
    // Future: connection binding, result set, etc.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Idle,
    Running,
    HasResults,
    Error,
}

impl TabState {
    pub fn icon(&self) -> &'static str {
        match self {
            TabState::Idle => "○",
            TabState::Running => "◉",
            TabState::HasResults => "●",
            TabState::Error => "✗",
        }
    }
}

impl QueryTab {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            title: format!("Query {}", id),
            content: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            pinned: false,
            state: TabState::Idle,
        }
    }

    /// Returns the display title: first non-empty line or the default title.
    pub fn display_title(&self) -> &str {
        self.content
            .iter()
            .find(|l| !l.trim().is_empty())
            .map(|s| if s.len() > 24 { &s[..24] } else { s.as_str() })
            .unwrap_or(&self.title)
    }

    pub fn current_line(&self) -> &str {
        self.content
            .get(self.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    fn ensure_cursor_bounds(&mut self) {
        if self.cursor_row >= self.content.len() {
            self.cursor_row = self.content.len().saturating_sub(1);
        }
        let line_len = self.content[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }
}

// ── Schema tree node ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub kind: TreeNodeKind,
    pub depth: usize,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeNodeKind {
    Catalog,
    Schema,
    Table,
    View,
    Column,
}

impl TreeNodeKind {
    pub fn icon(&self) -> &'static str {
        match self {
            TreeNodeKind::Catalog => "◆",
            TreeNodeKind::Schema => "◇",
            TreeNodeKind::Table => "▦",
            TreeNodeKind::View => "▤",
            TreeNodeKind::Column => "│",
        }
    }
}

impl TreeNode {
    /// Flatten tree into a display list of (depth, label, kind, has_children, expanded).
    pub fn flatten(&self) -> Vec<FlatNode> {
        let mut out = Vec::new();
        self.flatten_into(&mut out);
        out
    }

    fn flatten_into(&self, out: &mut Vec<FlatNode>) {
        out.push(FlatNode {
            depth: self.depth,
            label: self.label.clone(),
            kind: self.kind,
            has_children: !self.children.is_empty(),
            expanded: self.expanded,
        });
        if self.expanded {
            for child in &self.children {
                child.flatten_into(out);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct FlatNode {
    pub depth: usize,
    pub label: String,
    pub kind: TreeNodeKind,
    pub has_children: bool,
    pub expanded: bool,
}

// ── Application state ─────────────────────────────────────────

pub struct App {
    pub should_quit: bool,
    pub focused_pane: Pane,
    pub zoomed_pane: Option<Pane>,
    pub layout_preset: LayoutPreset,

    // Split ratios (0.0 to 1.0)
    pub hsplit_ratio: f64, // horizontal split: left columns vs right columns
    pub vsplit_top_ratio: f64, // vertical split in top half
    pub vsplit_bot_ratio: f64, // vertical split in bottom half

    // Tabs
    pub tabs: Vec<QueryTab>,
    pub active_tab: usize,
    next_tab_id: usize,

    // Editor mode
    pub key_mode: KeyMode,

    // Theme
    pub theme: Theme,
    pub theme_kind: ThemeKind,

    // Command palette
    pub command_palette_open: bool,
    pub command_palette_input: String,
    pub command_palette_cursor: usize,
    pub command_palette_selected: usize,
    pub commands: Vec<CommandEntry>,

    // Context panel
    pub context_mode: ContextMode,

    // Schema browser
    pub schema_tree: Vec<TreeNode>,
    pub schema_selected: usize,
    pub schema_filter: String,

    // Results (placeholder for RecordBatch integration)
    pub result_headers: Vec<String>,
    pub result_rows: Vec<Vec<String>>,
    pub result_selected_row: usize,
    pub result_scroll_offset: usize,
    pub result_col_offset: usize,

    // Help popup
    pub help_open: bool,

    // Notification
    pub notification: Option<(String, std::time::Instant)>,

    // Terminal size tracking for mouse hit-testing
    pub terminal_width: u16,
    pub terminal_height: u16,

    // Pane areas (updated each render for mouse hit-testing)
    pub pane_areas: std::collections::HashMap<Pane, ratatui::layout::Rect>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            should_quit: false,
            focused_pane: Pane::Editor,
            zoomed_pane: None,
            layout_preset: LayoutPreset::Default,
            hsplit_ratio: 0.25,
            vsplit_top_ratio: 0.55,
            vsplit_bot_ratio: 0.55,
            tabs: Vec::new(),
            active_tab: 0,
            next_tab_id: 1,
            key_mode: KeyMode::Normal,
            theme: Theme::builtin(ThemeKind::TokyoNight),
            theme_kind: ThemeKind::TokyoNight,
            command_palette_open: false,
            command_palette_input: String::new(),
            command_palette_cursor: 0,
            command_palette_selected: 0,
            commands: CommandEntry::default_commands(),
            context_mode: ContextMode::ServerInfo,
            schema_tree: Self::placeholder_schema_tree(),
            schema_selected: 0,
            schema_filter: String::new(),
            result_headers: Vec::new(),
            result_rows: Vec::new(),
            result_selected_row: 0,
            result_scroll_offset: 0,
            result_col_offset: 0,
            help_open: false,
            notification: None,
            terminal_width: 0,
            terminal_height: 0,
            pane_areas: std::collections::HashMap::new(),
        };
        app.create_tab();
        app.load_placeholder_results();
        app
    }

    fn create_tab(&mut self) {
        let tab = QueryTab::new(self.next_tab_id);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    pub fn active_tab_mut(&mut self) -> &mut QueryTab {
        &mut self.tabs[self.active_tab]
    }

    pub fn active_tab_ref(&self) -> &QueryTab {
        &self.tabs[self.active_tab]
    }

    pub fn notify(&mut self, msg: impl Into<String>) {
        self.notification = Some((msg.into(), std::time::Instant::now()));
    }

    fn cycle_theme(&mut self) {
        self.theme_kind = self.theme_kind.cycle_next();
        self.theme = Theme::builtin(self.theme_kind);
        self.notify(format!("Theme: {}", self.theme_kind.label()));
    }

    // ── Event dispatch ────────────────────────────────────────

    /// Returns `true` if the app should quit.
    pub fn handle_event(&mut self, event: AppEvent) -> bool {
        // Clear stale notifications (>3s)
        if let Some((_, ts)) = &self.notification {
            if ts.elapsed() > std::time::Duration::from_secs(3) {
                self.notification = None;
            }
        }

        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Mouse(mouse) => {
                self.handle_mouse(mouse);
                false
            }
            AppEvent::Resize(w, h) => {
                self.terminal_width = w;
                self.terminal_height = h;
                false
            }
            AppEvent::Tick => false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // ── Help popup (captures Esc / ? / F1 when open) ──────
        if self.help_open {
            match key.code {
                KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('?') => {
                    self.help_open = false;
                }
                _ => {}
            }
            return false;
        }

        // ── Command palette (captures all input when open) ────
        if self.command_palette_open {
            return self.handle_palette_key(key);
        }

        // ── Global keybindings (always active) ────────────────
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        match key.code {
            // Quit
            KeyCode::Char('q') if ctrl => return true,

            // Help popup
            KeyCode::F(1) => {
                self.help_open = !self.help_open;
                return false;
            }
            KeyCode::Char('?') if !ctrl && self.focused_pane != Pane::Editor => {
                self.help_open = !self.help_open;
                return false;
            }

            // Command palette
            KeyCode::Char('p') if ctrl => {
                self.command_palette_open = true;
                self.command_palette_input.clear();
                self.command_palette_cursor = 0;
                self.command_palette_selected = 0;
                return false;
            }

            // Pane focus: Ctrl+1/2/3/4
            KeyCode::Char('1') if ctrl => self.focused_pane = Pane::SchemaBrowser,
            KeyCode::Char('2') if ctrl => self.focused_pane = Pane::Editor,
            KeyCode::Char('3') if ctrl => self.focused_pane = Pane::Results,
            KeyCode::Char('4') if ctrl => self.focused_pane = Pane::ContextPanel,

            // Zoom toggle
            KeyCode::Char('z') if ctrl => {
                self.zoomed_pane = if self.zoomed_pane.is_some() {
                    None
                } else {
                    Some(self.focused_pane)
                };
            }

            // Tab management
            KeyCode::Char('t') if ctrl => self.create_tab(),
            KeyCode::Char('w') if ctrl => {
                if self.tabs.len() > 1 {
                    let removed = self.tabs.remove(self.active_tab);
                    if !removed.pinned {
                        if self.active_tab >= self.tabs.len() {
                            self.active_tab = self.tabs.len() - 1;
                        }
                    } else {
                        // Can't close pinned — reinsert
                        self.tabs.insert(self.active_tab, removed);
                        self.notify("Cannot close pinned tab");
                    }
                }
            }

            // Tab switching: Alt+Left/Right or Alt+1-9
            KeyCode::Left if alt => {
                if self.active_tab > 0 {
                    self.active_tab -= 1;
                }
            }
            KeyCode::Right if alt => {
                if self.active_tab + 1 < self.tabs.len() {
                    self.active_tab += 1;
                }
            }
            KeyCode::Char(c @ '1'..='9') if alt => {
                let idx = (c as usize) - ('1' as usize);
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                }
            }

            // Pane cycle: Tab / Shift+Tab
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.cycle_pane_focus(false);
            }
            KeyCode::Tab if !ctrl && self.focused_pane != Pane::Editor => {
                self.cycle_pane_focus(true);
            }

            // Layout cycle
            KeyCode::Char('l') if ctrl => {
                self.layout_preset = self.layout_preset.cycle_next();
                self.notify(format!("Layout: {}", self.layout_preset.label()));
            }

            // Theme cycle
            KeyCode::Char('k') if ctrl => {
                self.cycle_theme();
            }

            // Context panel mode cycle
            KeyCode::Char('j') if ctrl => {
                self.context_mode = self.context_mode.cycle_next();
            }

            // ── Pane-specific input ───────────────────────────
            _ => self.handle_pane_key(key),
        }

        false
    }

    fn cycle_pane_focus(&mut self, forward: bool) {
        let panes = Pane::ALL;
        let current = panes
            .iter()
            .position(|p| *p == self.focused_pane)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % panes.len()
        } else {
            (current + panes.len() - 1) % panes.len()
        };
        self.focused_pane = panes[next];
    }

    fn handle_pane_key(&mut self, key: KeyEvent) {
        match self.focused_pane {
            Pane::Editor => self.handle_editor_key(key),
            Pane::Results => self.handle_results_key(key),
            Pane::SchemaBrowser => self.handle_schema_key(key),
            Pane::ContextPanel => {} // placeholder
        }
    }

    // ── Editor input ──────────────────────────────────────────

    fn handle_editor_key(&mut self, key: KeyEvent) {
        let tab = &mut self.tabs[self.active_tab];

        match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return; // already handled in global
                }
                let row = tab.cursor_row;
                let col = tab.cursor_col;
                tab.content[row].insert(col, c);
                tab.cursor_col += 1;
            }
            KeyCode::Backspace => {
                let row = tab.cursor_row;
                let col = tab.cursor_col;
                if col > 0 {
                    tab.content[row].remove(col - 1);
                    tab.cursor_col -= 1;
                } else if row > 0 {
                    // Merge with previous line
                    let current_line = tab.content.remove(row);
                    tab.cursor_row -= 1;
                    tab.cursor_col = tab.content[tab.cursor_row].len();
                    tab.content[tab.cursor_row].push_str(&current_line);
                }
            }
            KeyCode::Enter => {
                let row = tab.cursor_row;
                let col = tab.cursor_col;
                let rest = tab.content[row][col..].to_string();
                tab.content[row].truncate(col);
                tab.cursor_row += 1;
                tab.content.insert(tab.cursor_row, rest);
                tab.cursor_col = 0;
            }
            KeyCode::Delete => {
                let row = tab.cursor_row;
                let col = tab.cursor_col;
                let line_len = tab.content[row].len();
                if col < line_len {
                    tab.content[row].remove(col);
                } else if row + 1 < tab.content.len() {
                    let next_line = tab.content.remove(row + 1);
                    tab.content[row].push_str(&next_line);
                }
            }
            KeyCode::Left => {
                if tab.cursor_col > 0 {
                    tab.cursor_col -= 1;
                } else if tab.cursor_row > 0 {
                    tab.cursor_row -= 1;
                    tab.cursor_col = tab.content[tab.cursor_row].len();
                }
            }
            KeyCode::Right => {
                let line_len = tab.content[tab.cursor_row].len();
                if tab.cursor_col < line_len {
                    tab.cursor_col += 1;
                } else if tab.cursor_row + 1 < tab.content.len() {
                    tab.cursor_row += 1;
                    tab.cursor_col = 0;
                }
            }
            KeyCode::Up => {
                if tab.cursor_row > 0 {
                    tab.cursor_row -= 1;
                    tab.ensure_cursor_bounds();
                }
            }
            KeyCode::Down => {
                if tab.cursor_row + 1 < tab.content.len() {
                    tab.cursor_row += 1;
                    tab.ensure_cursor_bounds();
                }
            }
            KeyCode::Home => tab.cursor_col = 0,
            KeyCode::End => {
                tab.cursor_col = tab.content[tab.cursor_row].len();
            }
            KeyCode::Tab => {
                // Insert spaces (expandtab)
                let row = tab.cursor_row;
                let col = tab.cursor_col;
                let spaces = "    ";
                tab.content[row].insert_str(col, spaces);
                tab.cursor_col += spaces.len();
            }
            _ => {}
        }
    }

    // ── Results navigation ────────────────────────────────────

    fn handle_results_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.result_selected_row > 0 {
                    self.result_selected_row -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.result_selected_row + 1 < self.result_rows.len() {
                    self.result_selected_row += 1;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.result_col_offset > 0 {
                    self.result_col_offset -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let max_cols = self.result_headers.len().saturating_sub(3);
                if self.result_col_offset < max_cols {
                    self.result_col_offset += 1;
                }
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.result_selected_row = 0;
                self.result_scroll_offset = 0;
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.result_selected_row = self.result_rows.len().saturating_sub(1);
            }
            KeyCode::PageUp => {
                self.result_selected_row = self.result_selected_row.saturating_sub(20);
            }
            KeyCode::PageDown => {
                self.result_selected_row =
                    (self.result_selected_row + 20).min(self.result_rows.len().saturating_sub(1));
            }
            _ => {}
        }
    }

    // ── Schema browser ────────────────────────────────────────

    fn handle_schema_key(&mut self, key: KeyEvent) {
        let flat_len = self.flat_schema_nodes().len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.schema_selected > 0 {
                    self.schema_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.schema_selected + 1 < flat_len {
                    self.schema_selected += 1;
                }
            }
            KeyCode::Enter | KeyCode::Right => {
                self.toggle_schema_node(self.schema_selected, true);
            }
            KeyCode::Left => {
                self.toggle_schema_node(self.schema_selected, false);
            }
            _ => {}
        }
    }

    pub fn flat_schema_nodes(&self) -> Vec<FlatNode> {
        let mut nodes = Vec::new();
        for tree in &self.schema_tree {
            nodes.extend(tree.flatten());
        }
        nodes
    }

    fn toggle_schema_node(&mut self, _flat_idx: usize, expand: bool) {
        // Walk the tree to find the node at the flattened index and toggle.
        // This is a simplified version — a production implementation would use
        // a persistent index mapping.
        fn toggle_recursive(
            nodes: &mut [TreeNode],
            counter: &mut usize,
            target: usize,
            expand: bool,
        ) -> bool {
            for node in nodes.iter_mut() {
                if *counter == target {
                    if expand && !node.expanded && !node.children.is_empty() {
                        node.expanded = true;
                    } else if !expand {
                        node.expanded = false;
                    }
                    return true;
                }
                *counter += 1;
                if node.expanded && toggle_recursive(&mut node.children, counter, target, expand) {
                    return true;
                }
            }
            false
        }

        let mut counter = 0;
        let target = _flat_idx;
        toggle_recursive(&mut self.schema_tree, &mut counter, target, expand);
    }

    // ── Command palette ───────────────────────────────────────

    fn handle_palette_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.command_palette_open = false;
            }
            KeyCode::Enter => {
                let filtered = self.filtered_commands();
                if let Some(cmd) = filtered.get(self.command_palette_selected) {
                    let action = cmd.action;
                    self.command_palette_open = false;
                    return self.execute_command(action);
                }
                self.command_palette_open = false;
            }
            KeyCode::Char(c) => {
                self.command_palette_input
                    .insert(self.command_palette_cursor, c);
                self.command_palette_cursor += 1;
                self.command_palette_selected = 0;
            }
            KeyCode::Backspace => {
                if self.command_palette_cursor > 0 {
                    self.command_palette_cursor -= 1;
                    self.command_palette_input
                        .remove(self.command_palette_cursor);
                    self.command_palette_selected = 0;
                }
            }
            KeyCode::Up => {
                if self.command_palette_selected > 0 {
                    self.command_palette_selected -= 1;
                }
            }
            KeyCode::Down => {
                let count = self.filtered_commands().len();
                if self.command_palette_selected + 1 < count {
                    self.command_palette_selected += 1;
                }
            }
            _ => {}
        }
        false
    }

    pub fn filtered_commands(&self) -> Vec<CommandEntry> {
        use fuzzy_matcher::skim::SkimMatcherV2;
        use fuzzy_matcher::FuzzyMatcher;

        if self.command_palette_input.is_empty() {
            return self.commands.clone();
        }

        let matcher = SkimMatcherV2::default();
        let mut scored: Vec<(i64, CommandEntry)> = self
            .commands
            .iter()
            .filter_map(|cmd| {
                matcher
                    .fuzzy_match(&cmd.label, &self.command_palette_input)
                    .map(|score| (score, cmd.clone()))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, cmd)| cmd).collect()
    }

    fn execute_command(&mut self, action: CommandAction) -> bool {
        match action {
            CommandAction::Quit => return true,
            CommandAction::NewTab => self.create_tab(),
            CommandAction::CloseTab => {
                if self.tabs.len() > 1 && !self.tabs[self.active_tab].pinned {
                    self.tabs.remove(self.active_tab);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                }
            }
            CommandAction::FocusPane(pane) => self.focused_pane = pane,
            CommandAction::ToggleZoom => {
                self.zoomed_pane = if self.zoomed_pane.is_some() {
                    None
                } else {
                    Some(self.focused_pane)
                };
            }
            CommandAction::CycleLayout => {
                self.layout_preset = self.layout_preset.cycle_next();
                self.notify(format!("Layout: {}", self.layout_preset.label()));
            }
            CommandAction::CycleTheme => {
                self.cycle_theme();
            }
            CommandAction::CycleContext => {
                self.context_mode = self.context_mode.cycle_next();
            }
            CommandAction::PinTab => {
                self.tabs[self.active_tab].pinned = !self.tabs[self.active_tab].pinned;
                let state = if self.tabs[self.active_tab].pinned {
                    "pinned"
                } else {
                    "unpinned"
                };
                self.notify(format!("Tab {}", state));
            }
            CommandAction::DuplicateTab => {
                let current = self.tabs[self.active_tab].clone();
                let mut new_tab = QueryTab {
                    id: self.next_tab_id,
                    title: format!("{} (copy)", current.title),
                    ..current
                };
                new_tab.pinned = false;
                self.next_tab_id += 1;
                self.tabs.push(new_tab);
                self.active_tab = self.tabs.len() - 1;
            }
            CommandAction::ShowHelp => {
                self.help_open = !self.help_open;
            }
        }
        false
    }

    // ── Mouse handling ────────────────────────────────────────

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // Hit-test pane areas
                let col = mouse.column;
                let row = mouse.row;
                for (pane, area) in &self.pane_areas {
                    if col >= area.x
                        && col < area.x + area.width
                        && row >= area.y
                        && row < area.y + area.height
                    {
                        self.focused_pane = *pane;
                        break;
                    }
                }
            }
            MouseEventKind::ScrollUp => match self.focused_pane {
                Pane::Results => {
                    if self.result_selected_row > 0 {
                        self.result_selected_row -= 1;
                    }
                }
                Pane::Editor => {
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.scroll_offset > 0 {
                        tab.scroll_offset -= 1;
                    }
                }
                Pane::SchemaBrowser => {
                    if self.schema_selected > 0 {
                        self.schema_selected -= 1;
                    }
                }
                _ => {}
            },
            MouseEventKind::ScrollDown => match self.focused_pane {
                Pane::Results => {
                    if self.result_selected_row + 1 < self.result_rows.len() {
                        self.result_selected_row += 1;
                    }
                }
                Pane::Editor => {
                    let tab = &mut self.tabs[self.active_tab];
                    if tab.scroll_offset + 1 < tab.content.len() {
                        tab.scroll_offset += 1;
                    }
                }
                Pane::SchemaBrowser => {
                    let flat_len = self.flat_schema_nodes().len();
                    if self.schema_selected + 1 < flat_len {
                        self.schema_selected += 1;
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    // ── Placeholder data ──────────────────────────────────────

    fn placeholder_schema_tree() -> Vec<TreeNode> {
        vec![TreeNode {
            label: "default".into(),
            kind: TreeNodeKind::Catalog,
            depth: 0,
            expanded: true,
            children: vec![TreeNode {
                label: "public".into(),
                kind: TreeNodeKind::Schema,
                depth: 1,
                expanded: true,
                children: vec![
                    TreeNode {
                        label: "devices".into(),
                        kind: TreeNodeKind::Table,
                        depth: 2,
                        expanded: false,
                        children: vec![
                            TreeNode {
                                label: "id: Int64".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "name: Utf8".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "created_at: Timestamp[μs,UTC]".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                        ],
                    },
                    TreeNode {
                        label: "metrics".into(),
                        kind: TreeNodeKind::Table,
                        depth: 2,
                        expanded: false,
                        children: vec![
                            TreeNode {
                                label: "ts: Timestamp[ns,UTC]".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "device_id: Int64".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "temperature: Float64".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "pressure: Float64".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "status: Utf8".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                        ],
                    },
                    TreeNode {
                        label: "events_view".into(),
                        kind: TreeNodeKind::View,
                        depth: 2,
                        expanded: false,
                        children: vec![
                            TreeNode {
                                label: "event_id: Int64".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "event_type: Utf8".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                            TreeNode {
                                label: "payload: Utf8".into(),
                                kind: TreeNodeKind::Column,
                                depth: 3,
                                expanded: false,
                                children: vec![],
                            },
                        ],
                    },
                ],
            }],
        }]
    }

    fn load_placeholder_results(&mut self) {
        self.result_headers = vec![
            "ts".into(),
            "device_id".into(),
            "temperature".into(),
            "pressure".into(),
            "status".into(),
        ];
        self.result_rows = (0..100)
            .map(|i| {
                vec![
                    format!("2025-01-15T10:{:02}:00Z", i % 60),
                    format!("{}", (i % 5) + 1),
                    format!("{:.2}", 20.0 + (i as f64 * 0.3).sin() * 5.0),
                    format!("{:.1}", 1013.0 + (i as f64 * 0.1).cos() * 2.0),
                    if i % 7 == 0 { "WARN" } else { "OK" }.into(),
                ]
            })
            .collect();
    }
}

// ── Command actions ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    Quit,
    NewTab,
    CloseTab,
    FocusPane(Pane),
    ToggleZoom,
    CycleLayout,
    CycleTheme,
    CycleContext,
    PinTab,
    DuplicateTab,
    ShowHelp,
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_alt(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::ALT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    // ── Pane focus ────────────────────────────────────────────

    #[test]
    fn initial_focus_is_editor() {
        let app = App::new();
        assert_eq!(app.focused_pane, Pane::Editor);
    }

    #[test]
    fn cycle_pane_focus_forward() {
        let mut app = App::new();
        assert_eq!(app.focused_pane, Pane::Editor);
        app.cycle_pane_focus(true);
        assert_eq!(app.focused_pane, Pane::Results);
        app.cycle_pane_focus(true);
        assert_eq!(app.focused_pane, Pane::ContextPanel);
        app.cycle_pane_focus(true);
        assert_eq!(app.focused_pane, Pane::SchemaBrowser);
        app.cycle_pane_focus(true);
        assert_eq!(app.focused_pane, Pane::Editor);
    }

    #[test]
    fn cycle_pane_focus_backward() {
        let mut app = App::new();
        app.cycle_pane_focus(false);
        assert_eq!(app.focused_pane, Pane::SchemaBrowser);
        app.cycle_pane_focus(false);
        assert_eq!(app.focused_pane, Pane::ContextPanel);
    }

    #[test]
    fn ctrl_1234_sets_focus() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('1'))));
        assert_eq!(app.focused_pane, Pane::SchemaBrowser);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('3'))));
        assert_eq!(app.focused_pane, Pane::Results);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('4'))));
        assert_eq!(app.focused_pane, Pane::ContextPanel);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('2'))));
        assert_eq!(app.focused_pane, Pane::Editor);
    }

    // ── Zoom ──────────────────────────────────────────────────

    #[test]
    fn zoom_toggles() {
        let mut app = App::new();
        assert!(app.zoomed_pane.is_none());
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(app.zoomed_pane, Some(Pane::Editor));
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('z'))));
        assert!(app.zoomed_pane.is_none());
    }

    #[test]
    fn zoom_captures_focused_pane() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('3'))));
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('z'))));
        assert_eq!(app.zoomed_pane, Some(Pane::Results));
    }

    // ── Tabs ──────────────────────────────────────────────────

    #[test]
    fn starts_with_one_tab() {
        let app = App::new();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active_tab, 0);
    }

    #[test]
    fn create_tab_adds_and_focuses() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active_tab, 2);
    }

    #[test]
    fn close_tab_wont_remove_last() {
        let mut app = App::new();
        assert_eq!(app.tabs.len(), 1);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('w'))));
        assert_eq!(app.tabs.len(), 1);
    }

    #[test]
    fn close_tab_removes_and_clamps() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.active_tab, 2);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('w'))));
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active_tab, 1);
    }

    #[test]
    fn cannot_close_pinned_tab() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        app.tabs[1].pinned = true;
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('w'))));
        assert_eq!(app.tabs.len(), 2);
    }

    #[test]
    fn alt_left_right_switches_tabs() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        assert_eq!(app.active_tab, 2);
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Left)));
        assert_eq!(app.active_tab, 1);
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Left)));
        assert_eq!(app.active_tab, 0);
        // Already at 0, should stay
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Left)));
        assert_eq!(app.active_tab, 0);
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Right)));
        assert_eq!(app.active_tab, 1);
    }

    #[test]
    fn alt_number_jumps_to_tab() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Char('1'))));
        assert_eq!(app.active_tab, 0);
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Char('3'))));
        assert_eq!(app.active_tab, 2);
        // Out of range — no change
        app.handle_event(AppEvent::Key(key_alt(KeyCode::Char('9'))));
        assert_eq!(app.active_tab, 2);
    }

    // ── Layout cycling ────────────────────────────────────────

    #[test]
    fn layout_cycles_through_all_presets() {
        let mut app = App::new();
        assert_eq!(app.layout_preset, LayoutPreset::Default);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('l'))));
        assert_eq!(app.layout_preset, LayoutPreset::WideEditor);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('l'))));
        assert_eq!(app.layout_preset, LayoutPreset::ResultsFocus);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('l'))));
        assert_eq!(app.layout_preset, LayoutPreset::Default);
    }

    // ── Context mode cycling ──────────────────────────────────

    #[test]
    fn context_mode_cycles() {
        let mut app = App::new();
        assert_eq!(app.context_mode, ContextMode::ServerInfo);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('j'))));
        assert_eq!(app.context_mode, ContextMode::QueryHistory);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('j'))));
        assert_eq!(app.context_mode, ContextMode::ConnectionManager);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('j'))));
        assert_eq!(app.context_mode, ContextMode::StreamMonitor);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('j'))));
        assert_eq!(app.context_mode, ContextMode::ServerInfo);
    }

    // ── Theme cycling ─────────────────────────────────────────

    #[test]
    fn theme_cycles() {
        let mut app = App::new();
        let initial = app.theme_kind;
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('k'))));
        assert_ne!(app.theme_kind, initial);
    }

    // ── Quit ──────────────────────────────────────────────────

    #[test]
    fn ctrl_q_quits() {
        let mut app = App::new();
        let should_quit = app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('q'))));
        assert!(should_quit);
    }

    #[test]
    fn normal_keys_dont_quit() {
        let mut app = App::new();
        let should_quit = app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        assert!(!should_quit);
    }

    // ── Editor input ──────────────────────────────────────────

    #[test]
    fn typing_inserts_characters() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('S'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('E'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('L'))));
        assert_eq!(app.tabs[0].content[0], "SEL");
        assert_eq!(app.tabs[0].cursor_col, 3);
    }

    #[test]
    fn backspace_deletes_char() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Backspace)));
        assert_eq!(app.tabs[0].content[0], "a");
        assert_eq!(app.tabs[0].cursor_col, 1);
    }

    #[test]
    fn backspace_at_line_start_merges_lines() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        assert_eq!(app.tabs[0].content.len(), 2);
        // Move to start of line 2
        app.handle_event(AppEvent::Key(key(KeyCode::Home)));
        app.handle_event(AppEvent::Key(key(KeyCode::Backspace)));
        assert_eq!(app.tabs[0].content.len(), 1);
        assert_eq!(app.tabs[0].content[0], "ab");
    }

    #[test]
    fn enter_splits_line() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Home)));
        app.handle_event(AppEvent::Key(key(KeyCode::Right)));
        app.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        assert_eq!(app.tabs[0].content, vec!["a", "b"]);
        assert_eq!(app.tabs[0].cursor_row, 1);
        assert_eq!(app.tabs[0].cursor_col, 0);
    }

    #[test]
    fn delete_key_removes_char_or_merges() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Enter)));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        // Go to end of first line
        app.handle_event(AppEvent::Key(key(KeyCode::Up)));
        app.handle_event(AppEvent::Key(key(KeyCode::End)));
        // Delete merges with next line
        app.handle_event(AppEvent::Key(key(KeyCode::Delete)));
        assert_eq!(app.tabs[0].content, vec!["ab"]);
    }

    #[test]
    fn arrow_keys_move_cursor() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('c'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Left)));
        assert_eq!(app.tabs[0].cursor_col, 2);
        app.handle_event(AppEvent::Key(key(KeyCode::Left)));
        assert_eq!(app.tabs[0].cursor_col, 1);
        app.handle_event(AppEvent::Key(key(KeyCode::Right)));
        assert_eq!(app.tabs[0].cursor_col, 2);
    }

    #[test]
    fn home_end_move_to_line_boundaries() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Char('a'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Char('b'))));
        app.handle_event(AppEvent::Key(key(KeyCode::Home)));
        assert_eq!(app.tabs[0].cursor_col, 0);
        app.handle_event(AppEvent::Key(key(KeyCode::End)));
        assert_eq!(app.tabs[0].cursor_col, 2);
    }

    #[test]
    fn tab_inserts_four_spaces() {
        let mut app = App::new();
        app.handle_event(AppEvent::Key(key(KeyCode::Tab)));
        assert_eq!(app.tabs[0].content[0], "    ");
        assert_eq!(app.tabs[0].cursor_col, 4);
    }

    // ── Results navigation ────────────────────────────────────

    #[test]
    fn results_navigation_bounds() {
        let mut app = App::new();
        app.focused_pane = Pane::Results;
        assert_eq!(app.result_selected_row, 0);

        // Can't go above 0
        app.handle_event(AppEvent::Key(key(KeyCode::Up)));
        assert_eq!(app.result_selected_row, 0);

        // Go down
        app.handle_event(AppEvent::Key(key(KeyCode::Down)));
        assert_eq!(app.result_selected_row, 1);

        // Jump to end
        app.handle_event(AppEvent::Key(key(KeyCode::Char('G'))));
        assert_eq!(app.result_selected_row, app.result_rows.len() - 1);

        // Can't go past end
        app.handle_event(AppEvent::Key(key(KeyCode::Down)));
        assert_eq!(app.result_selected_row, app.result_rows.len() - 1);

        // Jump to start
        app.handle_event(AppEvent::Key(key(KeyCode::Char('g'))));
        assert_eq!(app.result_selected_row, 0);
    }

    // ── Command palette ───────────────────────────────────────

    #[test]
    fn command_palette_opens_and_closes() {
        let mut app = App::new();
        assert!(!app.command_palette_open);
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('p'))));
        assert!(app.command_palette_open);
        app.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(!app.command_palette_open);
    }

    #[test]
    fn command_palette_typing_filters() {
        let mut app = App::new();
        app.command_palette_open = true;
        app.command_palette_input = "quit".into();
        let filtered = app.filtered_commands();
        assert!(!filtered.is_empty());
        assert_eq!(filtered[0].action, CommandAction::Quit);
    }

    #[test]
    fn command_palette_fuzzy_matches() {
        let mut app = App::new();
        app.command_palette_open = true;
        app.command_palette_input = "ntab".into();
        let filtered = app.filtered_commands();
        assert!(filtered.iter().any(|c| c.action == CommandAction::NewTab));
    }

    #[test]
    fn command_palette_empty_returns_all() {
        let app = App::new();
        let filtered = app.filtered_commands();
        assert_eq!(filtered.len(), app.commands.len());
    }

    #[test]
    fn command_palette_execute_quit() {
        let mut app = App::new();
        let should_quit = app.execute_command(CommandAction::Quit);
        assert!(should_quit);
    }

    #[test]
    fn command_palette_execute_new_tab() {
        let mut app = App::new();
        let tabs_before = app.tabs.len();
        app.execute_command(CommandAction::NewTab);
        assert_eq!(app.tabs.len(), tabs_before + 1);
    }

    #[test]
    fn command_palette_pin_toggle() {
        let mut app = App::new();
        assert!(!app.tabs[0].pinned);
        app.execute_command(CommandAction::PinTab);
        assert!(app.tabs[0].pinned);
        app.execute_command(CommandAction::PinTab);
        assert!(!app.tabs[0].pinned);
    }

    #[test]
    fn command_palette_duplicate_tab() {
        let mut app = App::new();
        // Type something in the editor first
        app.handle_event(AppEvent::Key(key(KeyCode::Char('x'))));
        app.execute_command(CommandAction::DuplicateTab);
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.tabs[1].content[0], "x");
        assert!(!app.tabs[1].pinned); // duplicate is never pinned
    }

    // ── Help popup ─────────────────────────────────────────

    #[test]
    fn f1_toggles_help() {
        let mut app = App::new();
        assert!(!app.help_open);
        app.handle_event(AppEvent::Key(key(KeyCode::F(1))));
        assert!(app.help_open);
        app.handle_event(AppEvent::Key(key(KeyCode::F(1))));
        assert!(!app.help_open);
    }

    #[test]
    fn question_mark_toggles_help_outside_editor() {
        let mut app = App::new();
        app.focused_pane = Pane::Results;
        app.handle_event(AppEvent::Key(key(KeyCode::Char('?'))));
        assert!(app.help_open);
        app.handle_event(AppEvent::Key(key(KeyCode::Char('?'))));
        assert!(!app.help_open);
    }

    #[test]
    fn question_mark_types_in_editor_instead() {
        let mut app = App::new();
        assert_eq!(app.focused_pane, Pane::Editor);
        app.handle_event(AppEvent::Key(key(KeyCode::Char('?'))));
        assert!(!app.help_open);
        assert_eq!(app.tabs[0].content[0], "?");
    }

    #[test]
    fn esc_closes_help() {
        let mut app = App::new();
        app.help_open = true;
        app.handle_event(AppEvent::Key(key(KeyCode::Esc)));
        assert!(!app.help_open);
    }

    #[test]
    fn help_blocks_other_keys() {
        let mut app = App::new();
        app.help_open = true;
        // Ctrl+T should NOT create a tab while help is open
        let tabs_before = app.tabs.len();
        app.handle_event(AppEvent::Key(key_ctrl(KeyCode::Char('t'))));
        assert_eq!(app.tabs.len(), tabs_before);
        assert!(app.help_open);
    }

    // ── Resize event ──────────────────────────────────────────

    #[test]
    fn resize_updates_dimensions() {
        let mut app = App::new();
        app.handle_event(AppEvent::Resize(120, 40));
        assert_eq!(app.terminal_width, 120);
        assert_eq!(app.terminal_height, 40);
    }

    // ── QueryTab ──────────────────────────────────────────────

    #[test]
    fn display_title_uses_first_line() {
        let mut tab = QueryTab::new(1);
        tab.content = vec!["SELECT * FROM users".into(), "WHERE id = 1".into()];
        assert_eq!(tab.display_title(), "SELECT * FROM users");
    }

    #[test]
    fn display_title_truncates_long_lines() {
        let mut tab = QueryTab::new(1);
        tab.content = vec!["SELECT very_long_column_name, another_column FROM table".into()];
        assert_eq!(tab.display_title().len(), 24);
    }

    #[test]
    fn display_title_skips_blank_lines() {
        let mut tab = QueryTab::new(1);
        tab.content = vec!["".into(), "  ".into(), "SELECT 1".into()];
        assert_eq!(tab.display_title(), "SELECT 1");
    }

    #[test]
    fn display_title_falls_back_to_default() {
        let tab = QueryTab::new(42);
        assert_eq!(tab.display_title(), "Query 42");
    }

    // ── Schema tree ───────────────────────────────────────────

    #[test]
    fn schema_tree_flatten_respects_expanded() {
        let tree = TreeNode {
            label: "catalog".into(),
            kind: TreeNodeKind::Catalog,
            depth: 0,
            expanded: false,
            children: vec![TreeNode {
                label: "schema".into(),
                kind: TreeNodeKind::Schema,
                depth: 1,
                expanded: false,
                children: vec![],
            }],
        };
        // Collapsed: only root visible
        assert_eq!(tree.flatten().len(), 1);
    }

    #[test]
    fn schema_tree_flatten_shows_children_when_expanded() {
        let tree = TreeNode {
            label: "catalog".into(),
            kind: TreeNodeKind::Catalog,
            depth: 0,
            expanded: true,
            children: vec![TreeNode {
                label: "schema".into(),
                kind: TreeNodeKind::Schema,
                depth: 1,
                expanded: false,
                children: vec![],
            }],
        };
        assert_eq!(tree.flatten().len(), 2);
    }
}
