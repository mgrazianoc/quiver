use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, ContextMode};

pub fn render_context_panel(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    // ── Mode selector tabs ────────────────────────────────────
    let modes = [
        ContextMode::ServerInfo,
        ContextMode::QueryHistory,
        ContextMode::ConnectionManager,
        ContextMode::StreamMonitor,
    ];

    let tab_line: Vec<Span> = modes
        .iter()
        .map(|mode| {
            let is_active = *mode == app.context_mode;
            let label = mode.label();
            if is_active {
                Span::styled(
                    format!(" {} ", label),
                    Style::default()
                        .fg(app.theme.bg)
                        .bg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!(" {} ", label),
                    Style::default().fg(Color::DarkGray).bg(app.theme.bg),
                )
            }
        })
        .collect();

    let tab_bar = Paragraph::new(Line::from(tab_line)).style(Style::default().bg(app.theme.bg));

    let tab_area = Rect::new(area.x, area.y, area.width, 1);
    frame.render_widget(tab_bar, tab_area);

    // ── Content area ──────────────────────────────────────────
    let content_area = Rect::new(
        area.x,
        area.y + 1,
        area.width,
        area.height.saturating_sub(1),
    );

    match app.context_mode {
        ContextMode::ServerInfo => render_server_info(frame, app, content_area),
        ContextMode::QueryHistory => render_query_history(frame, app, content_area),
        ContextMode::ConnectionManager => render_connection_manager(frame, app, content_area),
        ContextMode::StreamMonitor => render_stream_monitor(frame, app, content_area),
    }
}

fn render_server_info(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(ref profile) = app.connected_profile {
        let mut l = vec![
            Line::from(vec![
                Span::styled("Server: ", app.theme.result_header),
                Span::styled(&profile.name, Style::default().fg(Color::Green)),
            ]),
            Line::from(vec![
                Span::styled("  Endpoint: ", app.theme.info),
                Span::styled(profile.endpoint_uri(), Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
        ];

        if app.server_info.is_empty() {
            l.push(Line::styled(
                "No additional server info available.",
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            for (key, val) in &app.server_info {
                l.push(Line::from(vec![
                    Span::styled(format!("  {}: ", key), app.theme.info),
                    Span::styled(val.as_str(), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
        l
    } else {
        vec![
            Line::from(vec![
                Span::styled("Server: ", app.theme.result_header),
                Span::styled("Not connected", Style::default().fg(Color::DarkGray)),
            ]),
            Line::from(""),
            Line::styled(
                "Press Ctrl+O to connect to a",
                Style::default().fg(Color::DarkGray),
            ),
            Line::styled("Flight SQL server.", Style::default().fg(Color::DarkGray)),
        ]
    };

    let widget = Paragraph::new(lines).style(Style::default().bg(app.theme.bg).fg(app.theme.fg));
    frame.render_widget(widget, area);
}

fn render_query_history(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![Span::styled("Query History", app.theme.result_header)]),
        Line::from(""),
        Line::styled(
            "No queries executed yet.",
            Style::default().fg(Color::DarkGray),
        ),
        Line::from(""),
        Line::styled(
            "Queries will appear here as you execute them.",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let widget = Paragraph::new(lines).style(Style::default().bg(app.theme.bg).fg(app.theme.fg));
    frame.render_widget(widget, area);
}

fn render_connection_manager(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![Span::styled(
            "Connection Profiles",
            app.theme.result_header,
        )]),
        Line::from(""),
        Line::styled(
            "No profiles configured.",
            Style::default().fg(Color::DarkGray),
        ),
        Line::from(""),
        Line::styled(
            "Add connections in ~/.config/quiver/connections.toml",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let widget = Paragraph::new(lines).style(Style::default().bg(app.theme.bg).fg(app.theme.fg));
    frame.render_widget(widget, area);
}

fn render_stream_monitor(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(vec![Span::styled(
            "Stream Monitor",
            app.theme.result_header,
        )]),
        Line::from(""),
        Line::styled("No active streams.", Style::default().fg(Color::DarkGray)),
        Line::from(""),
        Line::styled(
            "Stream metrics will appear here during",
            Style::default().fg(Color::DarkGray),
        ),
        Line::styled(
            "active DoGet / DoPut operations:",
            Style::default().fg(Color::DarkGray),
        ),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Throughput:    ", app.theme.info),
            Span::styled("— rows/s", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Bandwidth:     ", app.theme.info),
            Span::styled("— MB/s", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Batches:       ", app.theme.info),
            Span::styled("—", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  Backpressure:  ", app.theme.info),
            Span::styled("—", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let widget = Paragraph::new(lines).style(Style::default().bg(app.theme.bg).fg(app.theme.fg));
    frame.render_widget(widget, area);
}
