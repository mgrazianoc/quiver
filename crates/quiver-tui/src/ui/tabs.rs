use ratatui::prelude::*;
use ratatui::widgets::Tabs;

use crate::app::App;

pub fn render_tab_bar(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let icon = tab.state.icon();
            let pin = if tab.pinned { "📌 " } else { "" };
            let title = tab.display_title();
            let label = format!(" {}{}{} ", pin, icon, title);

            if i == app.active_tab {
                Line::from(label).style(app.theme.tab_active)
            } else {
                Line::from(label).style(app.theme.tab_inactive)
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .style(Style::default().bg(app.theme.tab_bar_bg))
        .highlight_style(app.theme.tab_active)
        .select(app.active_tab)
        .divider("│");

    frame.render_widget(tabs, area);
}
