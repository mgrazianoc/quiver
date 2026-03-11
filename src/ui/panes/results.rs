use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row, Table};

use crate::app::App;

/// Infer a type badge from the header name (placeholder until Arrow types are available).
fn type_badge_for_header(header: &str) -> (&'static str, fn(&crate::theme::Theme) -> Style) {
    match header {
        h if h.contains("id") => ("i64", |t| t.type_integer),
        h if h == "ts" || h.contains("time") || h.contains("date") || h.contains("created") => {
            ("ts", |t| t.type_temporal)
        }
        h if h.contains("temperature")
            || h.contains("pressure")
            || h.contains("lat")
            || h.contains("lon")
            || h.contains("value")
            || h.contains("amount") =>
        {
            ("f64", |t| t.type_float)
        }
        h if h.contains("status")
            || h.contains("type")
            || h.contains("name")
            || h.contains("label")
            || h.contains("event")
            || h.contains("payload") =>
        {
            ("utf8", |t| t.type_string)
        }
        h if h.contains("active") || h.contains("enabled") || h.contains("flag") => {
            ("bool", |t| t.type_boolean)
        }
        _ => ("·", |t| t.result_cell),
    }
}

pub fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 || app.result_headers.is_empty() {
        let empty =
            ratatui::widgets::Paragraph::new("No results. Execute a query to see data here.")
                .style(Style::default().fg(Color::DarkGray).bg(app.theme.bg))
                .alignment(Alignment::Center);
        frame.render_widget(empty, area);
        return;
    }

    let col_offset = app.result_col_offset;
    let visible_headers: Vec<&str> = app
        .result_headers
        .iter()
        .skip(col_offset)
        .map(|s| s.as_str())
        .collect();

    if visible_headers.is_empty() {
        return;
    }

    // ── Column widths (adaptive) ──────────────────────────────
    let col_widths: Vec<u16> = visible_headers
        .iter()
        .enumerate()
        .map(|(vi, header)| {
            let actual_col = vi + col_offset;
            let header_w = header.len() + 6; // +6 for type badge and padding
            let max_data_w = app
                .result_rows
                .iter()
                .take(100) // sample first 100 rows
                .map(|row| row.get(actual_col).map(|c| c.len()).unwrap_or(0))
                .max()
                .unwrap_or(4);
            let w = header_w.max(max_data_w).min(48);
            w as u16
        })
        .collect();

    // ── Header row with type badges ───────────────────────────
    let header_cells: Vec<Cell> = visible_headers
        .iter()
        .map(|h| {
            let (badge, style_fn) = type_badge_for_header(h);
            let badge_style = style_fn(&app.theme);
            let line = Line::from(vec![
                Span::styled(*h, app.theme.result_header),
                Span::raw(" "),
                Span::styled(badge, badge_style),
            ]);
            Cell::from(line)
        })
        .collect();
    let header_row = Row::new(header_cells)
        .style(app.theme.result_header)
        .height(1);

    // ── Ensure scroll offset keeps selection visible ──────────
    let table_height = area.height.saturating_sub(2) as usize; // -2 for header + border area
    let scroll_offset = if app.result_selected_row >= app.result_scroll_offset + table_height {
        app.result_selected_row.saturating_sub(table_height) + 1
    } else if app.result_selected_row < app.result_scroll_offset {
        app.result_selected_row
    } else {
        app.result_scroll_offset
    };

    // ── Data rows ─────────────────────────────────────────────
    let rows: Vec<Row> = app
        .result_rows
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(table_height)
        .map(|(row_idx, row)| {
            let is_selected = row_idx == app.result_selected_row;
            let is_alt = row_idx % 2 == 1;

            let base_style = if is_selected {
                app.theme.result_selected
            } else if is_alt {
                app.theme.result_cell_alt
            } else {
                app.theme.result_cell
            };

            let cells: Vec<Cell> = visible_headers
                .iter()
                .enumerate()
                .map(|(vi, _)| {
                    let actual_col = vi + col_offset;
                    let value = row.get(actual_col).map(|s| s.as_str()).unwrap_or("");

                    let cell_style = if value == "NULL" {
                        if is_selected {
                            app.theme.result_selected
                        } else {
                            app.theme.result_null
                        }
                    } else if value == "WARN" || value == "ERROR" {
                        if is_selected {
                            app.theme.result_selected
                        } else {
                            app.theme.warning
                        }
                    } else {
                        base_style
                    };

                    Cell::from(value).style(cell_style)
                })
                .collect();

            Row::new(cells).style(base_style)
        })
        .collect();

    // ── Build table ───────────────────────────────────────────
    let constraints: Vec<Constraint> = col_widths.iter().map(|&w| Constraint::Length(w)).collect();

    let table = Table::new(rows, &constraints)
        .header(header_row)
        .style(Style::default().bg(app.theme.bg))
        .column_spacing(1)
        .row_highlight_style(app.theme.result_selected);

    frame.render_widget(table, area);
}
