use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, Pane};

/// Returns context-aware keybinding help text based on the focused pane.
pub fn help_lines(app: &App) -> Vec<Line<'static>> {
    let accent = app.theme.accent;
    let dim = Style::default().add_modifier(Modifier::DIM);
    let bold = Style::default().add_modifier(Modifier::BOLD);

    let mut lines: Vec<Line<'static>> = Vec::new();

    let heading = |title: &str| -> Line<'static> {
        Line::from(Span::styled(format!("── {} ──", title), bold.fg(accent)))
    };

    let row = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(format!("{:<18}", key), Style::default().fg(accent)),
            Span::styled(desc.to_string(), dim),
        ])
    };

    // Global keys — always shown
    lines.push(heading("Global"));
    lines.push(row("Ctrl+Q", "Quit"));
    lines.push(row("Ctrl+P", "Command palette"));
    lines.push(row("F1 / ?", "Toggle this help"));
    lines.push(row("Ctrl+1/2/3/4", "Focus pane"));
    lines.push(row("Tab / Shift+Tab", "Cycle pane focus"));
    lines.push(row("Ctrl+Z", "Toggle zoom"));
    lines.push(row("Ctrl+T", "New tab"));
    lines.push(row("Ctrl+W", "Close tab"));
    lines.push(row("Alt+←/→", "Switch tab"));
    lines.push(row("Alt+1-9", "Jump to tab N"));
    lines.push(row("Ctrl+L", "Cycle layout"));
    lines.push(row("Ctrl+K", "Cycle theme"));
    lines.push(row("Ctrl+J", "Cycle context panel"));
    lines.push(Line::from(""));

    // Pane-specific keys
    match app.focused_pane {
        Pane::Editor => {
            lines.push(heading("Editor"));
            lines.push(row("Type", "Insert text"));
            lines.push(row("Enter", "New line"));
            lines.push(row("Backspace", "Delete char / merge line"));
            lines.push(row("Delete", "Delete forward / merge"));
            lines.push(row("←/→/↑/↓", "Move cursor"));
            lines.push(row("Home / End", "Line start / end"));
            lines.push(row("Tab", "Insert 4 spaces"));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Note: Tab inserts spaces here (use Ctrl+1-4 to switch panes)",
                dim,
            )));
        }
        Pane::Results => {
            lines.push(heading("Results"));
            lines.push(row("j/k or ↑/↓", "Navigate rows"));
            lines.push(row("h/l or ←/→", "Scroll columns"));
            lines.push(row("g / G", "First / last row"));
            lines.push(row("PageUp / PageDown", "Page through results"));
        }
        Pane::SchemaBrowser => {
            lines.push(heading("Schema Browser"));
            lines.push(row("j/k or ↑/↓", "Navigate tree"));
            lines.push(row("Enter / →", "Expand node"));
            lines.push(row("←", "Collapse node"));
        }
        Pane::ContextPanel => {
            lines.push(heading("Context Panel"));
            lines.push(row("Ctrl+J", "Cycle mode"));
            lines.push(Line::from(Span::styled(
                "(Server Info / History / Connections / Stream Monitor)",
                dim,
            )));
        }
    }

    lines
}

pub fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(app.theme.palette_border)
        .title(" Keybindings (press Esc or ? to close) ")
        .title_style(app.theme.palette_border.add_modifier(Modifier::BOLD))
        .style(Style::default().bg(app.theme.palette_bg));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = help_lines(app);
    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}
