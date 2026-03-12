use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::{App, CommandAction, Pane};

/// A command entry in the palette.
#[derive(Debug, Clone)]
pub struct CommandEntry {
    pub label: String,
    pub category: String,
    pub shortcut: String,
    pub action: CommandAction,
}

impl CommandEntry {
    fn new(
        label: impl Into<String>,
        category: impl Into<String>,
        shortcut: impl Into<String>,
        action: CommandAction,
    ) -> Self {
        Self {
            label: label.into(),
            category: category.into(),
            shortcut: shortcut.into(),
            action,
        }
    }

    pub fn default_commands() -> Vec<Self> {
        vec![
            Self::new("Quit", "App", "Ctrl+Q", CommandAction::Quit),
            Self::new("New Tab", "Tab", "Ctrl+T", CommandAction::NewTab),
            Self::new("Close Tab", "Tab", "Ctrl+W", CommandAction::CloseTab),
            Self::new("Pin/Unpin Tab", "Tab", "", CommandAction::PinTab),
            Self::new("Duplicate Tab", "Tab", "", CommandAction::DuplicateTab),
            Self::new(
                "Focus: Schema Browser",
                "Navigation",
                "Ctrl+1",
                CommandAction::FocusPane(Pane::SchemaBrowser),
            ),
            Self::new(
                "Focus: Editor",
                "Navigation",
                "Ctrl+2",
                CommandAction::FocusPane(Pane::Editor),
            ),
            Self::new(
                "Focus: Results",
                "Navigation",
                "Ctrl+3",
                CommandAction::FocusPane(Pane::Results),
            ),
            Self::new(
                "Focus: Context Panel",
                "Navigation",
                "Ctrl+4",
                CommandAction::FocusPane(Pane::ContextPanel),
            ),
            Self::new("Toggle Zoom", "Layout", "Ctrl+Z", CommandAction::ToggleZoom),
            Self::new(
                "Cycle Layout",
                "Layout",
                "Ctrl+L",
                CommandAction::CycleLayout,
            ),
            Self::new(
                "Cycle Theme",
                "Settings",
                "Ctrl+K",
                CommandAction::CycleTheme,
            ),
            Self::new(
                "Cycle Context Panel",
                "Navigation",
                "Ctrl+J",
                CommandAction::CycleContext,
            ),
            Self::new(
                "Show Keybindings",
                "Help",
                "F1 / ?",
                CommandAction::ShowHelp,
            ),
            Self::new(
                "Connect to Server",
                "Connection",
                "Ctrl+O",
                CommandAction::Connect,
            ),
            Self::new(
                "Disconnect",
                "Connection",
                "Ctrl+D",
                CommandAction::Disconnect,
            ),
            Self::new(
                "Execute Query",
                "Query",
                "Ctrl+E",
                CommandAction::ExecuteQuery,
            ),
            Self::new(
                "Cancel Query",
                "Query",
                "Ctrl+Shift+C",
                CommandAction::CancelQuery,
            ),
            Self::new(
                "Refresh Schema",
                "Connection",
                "Ctrl+R",
                CommandAction::RefreshSchema,
            ),
        ]
    }
}

pub fn render_palette(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.palette_border)
        .title(" Command Palette ")
        .title_style(app.theme.palette_border.add_modifier(Modifier::BOLD))
        .style(Style::default().bg(app.theme.palette_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // ── Input line ────────────────────────────────────────────
    let input_area = Rect::new(inner.x, inner.y, inner.width, 1);
    let prompt = format!("❯ {}", app.command_palette_input);
    let input_widget = Paragraph::new(prompt).style(app.theme.palette_input);
    frame.render_widget(input_widget, input_area);

    // Position cursor
    let cursor_x = inner.x + 2 + app.command_palette_cursor as u16;
    frame.set_cursor_position((cursor_x.min(inner.x + inner.width - 1), inner.y));

    // ── Separator ─────────────────────────────────────────────
    let sep_area = Rect::new(inner.x, inner.y + 1, inner.width, 1);
    let sep = Paragraph::new("─".repeat(inner.width as usize)).style(
        Style::default()
            .fg(Color::DarkGray)
            .bg(app.theme.palette_bg),
    );
    frame.render_widget(sep, sep_area);

    // ── Filtered command list ─────────────────────────────────
    let list_area = Rect::new(
        inner.x,
        inner.y + 2,
        inner.width,
        inner.height.saturating_sub(2),
    );
    let filtered = app.filtered_commands();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .take(list_area.height as usize)
        .map(|(i, cmd)| {
            let is_selected = i == app.command_palette_selected;

            let style = if is_selected {
                app.theme.palette_item_selected
            } else {
                app.theme.palette_item
            };

            let shortcut_display = if cmd.shortcut.is_empty() {
                String::new()
            } else {
                format!("  {}", cmd.shortcut)
            };

            let cat_display = format!("[{}]", cmd.category);

            let line = Line::from(vec![
                Span::styled(&cmd.label, style),
                Span::styled(
                    shortcut_display,
                    if is_selected {
                        style
                    } else {
                        Style::default()
                            .fg(Color::DarkGray)
                            .bg(app.theme.palette_bg)
                    },
                ),
                Span::raw("  "),
                Span::styled(
                    cat_display,
                    if is_selected {
                        style
                    } else {
                        Style::default()
                            .fg(Color::DarkGray)
                            .bg(app.theme.palette_bg)
                    },
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).style(Style::default().bg(app.theme.palette_bg));
    frame.render_widget(list, list_area);
}
