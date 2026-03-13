use ratatui::prelude::*;

use crate::app::App;

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let bg = app.theme.status_bar_bg;
    let fg = app.theme.status_bar_fg;

    // ── Left side: connection + schema + pane focus ────────────
    let (connection_dot, dot_color, connection_label) = if app.query_running {
        ("●", Color::Yellow, "Running...".to_string())
    } else if let Some(ref profile) = app.connected_profile {
        ("●", Color::Green, profile.name.clone())
    } else {
        (
            "●",
            Color::Rgb(100, 100, 100),
            "No Connection (Ctrl+O)".to_string(),
        )
    };

    let pane_label = app.focused_pane.label();
    let mode_label = app.key_mode.label();

    let left = vec![
        Span::styled(
            format!(" {} ", connection_dot),
            Style::default().fg(dot_color).bg(bg),
        ),
        Span::styled(
            format!("{} ", connection_label),
            Style::default().fg(fg).bg(bg),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(bg)),
        Span::styled(
            format!("{} ", pane_label),
            Style::default().fg(app.theme.accent).bg(bg),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(bg)),
        Span::styled(
            format!(" {} ", mode_label),
            Style::default()
                .fg(bg)
                .bg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    // ── Right side: row count + theme + notification ──────────
    let row_info = if app.result_total_rows > 0 {
        format!(
            " {}/{} rows ",
            app.result_selected_row + 1,
            app.result_total_rows
        )
    } else {
        " 0 rows ".to_string()
    };

    let theme_label = format!(" {} ", app.theme_kind.label());

    let notification = if let Some((msg, _)) = &app.notification {
        format!(" {} ", msg)
    } else {
        String::new()
    };

    let right = vec![
        Span::styled(notification, Style::default().fg(app.theme.accent).bg(bg)),
        Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(bg)),
        Span::styled(row_info, Style::default().fg(fg).bg(bg)),
        Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(bg)),
        Span::styled(
            theme_label,
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::DIM),
        ),
        Span::styled("│ ", Style::default().fg(Color::DarkGray).bg(bg)),
        Span::styled(
            " F1",
            Style::default()
                .fg(app.theme.accent)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Help ", Style::default().fg(fg).bg(bg)),
        Span::styled(
            "Ctrl+P",
            Style::default()
                .fg(app.theme.accent)
                .bg(bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(":Commands ", Style::default().fg(fg).bg(bg)),
    ];

    // Compose into a single line
    let left_line = Line::from(left);
    let right_line = Line::from(right);

    // Render left-aligned
    frame.render_widget(
        ratatui::widgets::Paragraph::new(left_line).style(Style::default().bg(bg)),
        area,
    );

    // Render right-aligned
    let right_width: u16 = right_line.width() as u16;
    if area.width > right_width {
        let right_area = Rect::new(area.x + area.width - right_width, area.y, right_width, 1);
        frame.render_widget(
            ratatui::widgets::Paragraph::new(right_line).style(Style::default().bg(bg)),
            right_area,
        );
    }
}
