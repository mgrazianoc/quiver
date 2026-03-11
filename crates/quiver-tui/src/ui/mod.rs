pub mod command_palette;
pub mod help;
pub mod panes;
pub mod statusbar;
pub mod tabs;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear};

use crate::app::{App, LayoutPreset, Pane};

/// Top-level render function. Computes layout, renders panes, overlays.
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.terminal_width = area.width;
    app.terminal_height = area.height;

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(app.theme.bg));
    frame.render_widget(bg_block, area);

    // Vertical split: [tab bar (1)] [main area] [status bar (1)]
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(5),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let tab_area = outer[0];
    let main_area = outer[1];
    let status_area = outer[2];

    // ── Tab bar ───────────────────────────────────────────────
    tabs::render_tab_bar(frame, app, tab_area);

    // ── Main panes ────────────────────────────────────────────
    if let Some(zoomed) = app.zoomed_pane {
        // Zoomed: single pane fills the entire main area
        app.pane_areas.clear();
        app.pane_areas.insert(zoomed, main_area);
        render_pane(frame, app, zoomed, main_area, true);
    } else {
        render_layout(frame, app, main_area);
    }

    // ── Status bar ────────────────────────────────────────────
    statusbar::render_status_bar(frame, app, status_area);

    // ── Help overlay ──────────────────────────────────────────
    if app.help_open {
        let help_width = 56u16.min(area.width.saturating_sub(4));
        let help_height = 28u16.min(area.height.saturating_sub(4));
        let help_area = centered_rect(help_width, help_height, area);
        frame.render_widget(Clear, help_area);
        help::render_help(frame, app, help_area);
    }

    // ── Command palette overlay ───────────────────────────────
    if app.command_palette_open {
        // Center the palette
        let palette_width = 60u16.min(area.width.saturating_sub(4));
        let palette_height = 16u16.min(area.height.saturating_sub(4));
        let palette_area = centered_rect(palette_width, palette_height, area);

        // Clear the area behind the palette
        frame.render_widget(Clear, palette_area);
        command_palette::render_palette(frame, app, palette_area);
    }
}

/// Render the multi-pane layout based on the current preset.
fn render_layout(frame: &mut Frame, app: &mut App, area: Rect) {
    app.pane_areas.clear();

    match app.layout_preset {
        LayoutPreset::Default => {
            // 4-pane: top (schema | editor), bottom (context | results)
            let vsplit = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage((app.vsplit_top_ratio * 100.0) as u16),
                    Constraint::Percentage(((1.0 - app.vsplit_top_ratio) * 100.0) as u16),
                ])
                .split(area);

            let top = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((app.hsplit_ratio * 100.0) as u16),
                    Constraint::Percentage(((1.0 - app.hsplit_ratio) * 100.0) as u16),
                ])
                .split(vsplit[0]);

            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((app.hsplit_ratio * 100.0) as u16),
                    Constraint::Percentage(((1.0 - app.hsplit_ratio) * 100.0) as u16),
                ])
                .split(vsplit[1]);

            app.pane_areas.insert(Pane::SchemaBrowser, top[0]);
            app.pane_areas.insert(Pane::Editor, top[1]);
            app.pane_areas.insert(Pane::ContextPanel, bottom[0]);
            app.pane_areas.insert(Pane::Results, bottom[1]);

            render_pane(frame, app, Pane::SchemaBrowser, top[0], false);
            render_pane(frame, app, Pane::Editor, top[1], false);
            render_pane(frame, app, Pane::ContextPanel, bottom[0], false);
            render_pane(frame, app, Pane::Results, bottom[1], false);
        }
        LayoutPreset::WideEditor => {
            // Top: editor (full width), Bottom: schema | results | context
            let vsplit = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
                .split(area);

            let bottom = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(55),
                    Constraint::Percentage(25),
                ])
                .split(vsplit[1]);

            app.pane_areas.insert(Pane::Editor, vsplit[0]);
            app.pane_areas.insert(Pane::SchemaBrowser, bottom[0]);
            app.pane_areas.insert(Pane::Results, bottom[1]);
            app.pane_areas.insert(Pane::ContextPanel, bottom[2]);

            render_pane(frame, app, Pane::Editor, vsplit[0], false);
            render_pane(frame, app, Pane::SchemaBrowser, bottom[0], false);
            render_pane(frame, app, Pane::Results, bottom[1], false);
            render_pane(frame, app, Pane::ContextPanel, bottom[2], false);
        }
        LayoutPreset::ResultsFocus => {
            // Left: thin sidebar (schema + context stacked), Right: results (large)
            let hsplit = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(area);

            let sidebar = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40),
                    Constraint::Percentage(30),
                    Constraint::Percentage(30),
                ])
                .split(hsplit[0]);

            app.pane_areas.insert(Pane::SchemaBrowser, sidebar[0]);
            app.pane_areas.insert(Pane::Editor, sidebar[1]);
            app.pane_areas.insert(Pane::ContextPanel, sidebar[2]);
            app.pane_areas.insert(Pane::Results, hsplit[1]);

            render_pane(frame, app, Pane::SchemaBrowser, sidebar[0], false);
            render_pane(frame, app, Pane::Editor, sidebar[1], false);
            render_pane(frame, app, Pane::ContextPanel, sidebar[2], false);
            render_pane(frame, app, Pane::Results, hsplit[1], false);
        }
    }
}

/// Render a single pane with its border.
fn render_pane(frame: &mut Frame, app: &mut App, pane: Pane, area: Rect, zoomed: bool) {
    let is_focused = app.focused_pane == pane;

    let border_style = if is_focused {
        app.theme.border_focused
    } else {
        app.theme.border
    };

    let title = if zoomed {
        format!(" {} [ZOOMED] ", pane.label())
    } else {
        format!(" {} ", pane.label())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(title)
        .title_style(if is_focused {
            border_style.add_modifier(Modifier::BOLD)
        } else {
            border_style
        })
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    match pane {
        Pane::Editor => panes::editor::render_editor(frame, app, inner),
        Pane::Results => panes::results::render_results(frame, app, inner),
        Pane::SchemaBrowser => panes::schema_browser::render_schema_browser(frame, app, inner),
        Pane::ContextPanel => panes::context_panel::render_context_panel(frame, app, inner),
    }
}

/// Create a centered rectangle of given size within `area`.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
