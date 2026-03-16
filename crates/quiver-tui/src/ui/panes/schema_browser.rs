use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, Paragraph};

use quiver_core::catalog::TreeNodeKind;

use crate::app::{App, Pane};

pub fn render_schema_browser(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // ── Filter input bar ──────────────────────────────────────
    let (filter_area, list_area) =
        if !app.schema_filter.is_empty() || app.focused_pane == Pane::SchemaBrowser {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1)])
                .split(area);

            let filter_text = if app.schema_filter.is_empty() {
                "/ type to filter…".to_string()
            } else {
                format!("/ {}_", app.schema_filter)
            };
            let filter_style = if app.schema_filter.is_empty() {
                Style::default().fg(Color::DarkGray).bg(app.theme.bg)
            } else {
                Style::default().fg(app.theme.accent).bg(app.theme.bg)
            };
            let filter_widget = Paragraph::new(filter_text).style(filter_style);
            frame.render_widget(filter_widget, chunks[0]);
            (Some(chunks[0]), chunks[1])
        } else {
            (None, area)
        };

    let flat_nodes = app.flat_schema_nodes();

    if flat_nodes.is_empty() {
        let msg = if !app.schema_filter.is_empty() {
            "No matching items."
        } else {
            "No schema loaded.\nConnect to a server to browse."
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray).bg(app.theme.bg))
            .alignment(Alignment::Center);
        frame.render_widget(empty, list_area);
        return;
    }

    // Ensure selected is in view
    let visible_start = if app.schema_selected >= list_area.height as usize {
        app.schema_selected - list_area.height as usize + 1
    } else {
        0
    };

    let items: Vec<ListItem> = flat_nodes
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(list_area.height as usize)
        .map(|(i, node)| {
            let is_selected = i == app.schema_selected;
            let indent = "  ".repeat(node.depth);

            let expand_icon = if node.has_children {
                if node.expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };

            let kind_icon = node.kind.icon();

            let name_style = if is_selected {
                app.theme.tree_node_selected
            } else {
                match node.kind {
                    TreeNodeKind::Catalog => app.theme.tree_icon.add_modifier(Modifier::BOLD),
                    TreeNodeKind::Schema => app.theme.tree_icon,
                    TreeNodeKind::Table => app.theme.tree_node,
                    TreeNodeKind::View => app.theme.tree_node.add_modifier(Modifier::ITALIC),
                    TreeNodeKind::Column => Style::default().fg(Color::DarkGray).bg(app.theme.bg),
                }
            };

            let line = Line::from(vec![
                Span::raw(indent),
                Span::styled(expand_icon, app.theme.tree_icon),
                Span::styled(format!("{} ", kind_icon), app.theme.tree_icon),
                Span::styled(&node.label, name_style),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).style(Style::default().bg(app.theme.bg));

    frame.render_widget(list, list_area);
}
