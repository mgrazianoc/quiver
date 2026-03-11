pub mod command_palette;
pub mod help;
pub mod panes;
pub mod statusbar;
pub mod tabs;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear};

use crate::app::{App, ConnectAuthKind, ConnectField, LayoutPreset, Pane};

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

    // ── Connection dialog overlay ─────────────────────────────
    if app.connect_dialog_open {
        let dialog_width = 64u16.min(area.width.saturating_sub(4));
        let dialog_height = 24u16.min(area.height.saturating_sub(4));
        let dialog_area = centered_rect(dialog_width, dialog_height, area);
        frame.render_widget(Clear, dialog_area);
        render_connect_dialog(frame, app, dialog_area);
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

/// Render the connection dialog popup.
fn render_connect_dialog(frame: &mut Frame, app: &App, area: Rect) {
    use ratatui::widgets::Paragraph;

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.border_focused)
        .title(" Connect to Flight SQL Server ")
        .title_style(app.theme.border_focused.add_modifier(Modifier::BOLD))
        .style(Style::default().bg(app.theme.bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let field_style = |field: ConnectField| -> Style {
        if app.connect_field == field {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let cursor = |field: ConnectField| -> Span<'_> {
        if app.connect_field == field {
            Span::styled("_", Style::default().fg(app.theme.accent))
        } else {
            Span::raw("")
        }
    };

    let label_style = app.theme.result_header;

    let mut lines: Vec<Line> = Vec::new();

    // Name
    lines.push(Line::from(vec![
        Span::styled(" Name:     ", label_style),
        Span::styled(&app.connect_name, field_style(ConnectField::Name)),
        cursor(ConnectField::Name),
    ]));

    // Host
    lines.push(Line::from(vec![
        Span::styled(" Host:     ", label_style),
        Span::styled(&app.connect_host, field_style(ConnectField::Host)),
        cursor(ConnectField::Host),
    ]));

    // Port
    lines.push(Line::from(vec![
        Span::styled(" Port:     ", label_style),
        Span::styled(&app.connect_port, field_style(ConnectField::Port)),
        cursor(ConnectField::Port),
    ]));

    // TLS
    let tls_label = if app.connect_tls {
        "[x] TLS"
    } else {
        "[ ] TLS"
    };
    lines.push(Line::from(vec![
        Span::styled(" TLS:      ", label_style),
        Span::styled(tls_label, field_style(ConnectField::Tls)),
    ]));

    // Separator
    lines.push(Line::styled(
        " \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
        Style::default().fg(Color::DarkGray),
    ));

    // Auth method
    lines.push(Line::from(vec![
        Span::styled(" Auth:     ", label_style),
        Span::styled(
            format!("< {} >", app.connect_auth.label()),
            field_style(ConnectField::Auth),
        ),
    ]));

    // Auth-specific fields
    match app.connect_auth {
        ConnectAuthKind::None => {}
        ConnectAuthKind::Basic => {
            lines.push(Line::from(vec![
                Span::styled(" Username: ", label_style),
                Span::styled(&app.connect_username, field_style(ConnectField::Username)),
                cursor(ConnectField::Username),
            ]));
            let masked: String = "\u{2022}".repeat(app.connect_password.len());
            lines.push(Line::from(vec![
                Span::styled(" Password: ", label_style),
                Span::styled(masked, field_style(ConnectField::Password)),
                cursor(ConnectField::Password),
            ]));
        }
        ConnectAuthKind::Bearer => {
            lines.push(Line::from(vec![
                Span::styled(" Token:    ", label_style),
                Span::styled(&app.connect_token, field_style(ConnectField::Token)),
                cursor(ConnectField::Token),
            ]));
        }
    }

    // Advanced section
    let adv_arrow = if app.connect_advanced_open {
        "\u{25be}"
    } else {
        "\u{25b8}"
    };
    lines.push(Line::styled(
        format!(" {} Advanced (Ctrl+A)", adv_arrow),
        Style::default().fg(Color::DarkGray),
    ));

    if app.connect_advanced_open {
        lines.push(Line::from(vec![
            Span::styled("   Timeout: ", label_style),
            Span::styled(&app.connect_timeout, field_style(ConnectField::ConnTimeout)),
            cursor(ConnectField::ConnTimeout),
            Span::styled(" s", Style::default().fg(Color::DarkGray)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("   Retries: ", label_style),
            Span::styled(
                &app.connect_max_retries,
                field_style(ConnectField::MaxRetries),
            ),
            cursor(ConnectField::MaxRetries),
        ]));
    }

    // Separator before buttons
    lines.push(Line::from(""));

    // Test Connection button
    let test_style = if app.connect_field == ConnectField::TestButton {
        Style::default()
            .fg(app.theme.bg)
            .bg(app.theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD)
    };
    let test_label = if app.connect_testing {
        " [ Testing\u{2026} ] "
    } else {
        " [ Test Connection ] "
    };
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(test_label, test_style),
    ]));

    // Test result feedback
    if let Some((success, ref msg)) = app.connect_test_status {
        if !app.connect_testing {
            let (icon, color) = if success {
                ("\u{2713}", Color::Green)
            } else {
                ("\u{2717}", Color::Red)
            };
            // Truncate long error messages to fit dialog
            let max_msg_len = inner.width.saturating_sub(6) as usize;
            let display_msg = if msg.len() > max_msg_len {
                format!("{}...", &msg[..max_msg_len.saturating_sub(3)])
            } else {
                msg.clone()
            };
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(
                    format!(" {} {} ", icon, display_msg),
                    Style::default().fg(color),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        " Enter connect \u{2502} Ctrl+T test \u{2502} Tab/\u{2191}\u{2193} switch \u{2502} Esc cancel",
        Style::default().fg(Color::DarkGray),
    ));

    let paragraph = Paragraph::new(lines).style(Style::default().bg(app.theme.bg).fg(app.theme.fg));
    frame.render_widget(paragraph, inner);
}
