use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::{App, Pane};

/// Simple keyword-based SQL highlighter.
/// This will be replaced by tree-sitter in a future version.
fn highlight_sql_line<'a>(line: &'a str, app: &App) -> Line<'a> {
    let keywords = [
        "SELECT",
        "FROM",
        "WHERE",
        "AND",
        "OR",
        "NOT",
        "IN",
        "IS",
        "NULL",
        "AS",
        "ON",
        "JOIN",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "FULL",
        "CROSS",
        "GROUP",
        "BY",
        "ORDER",
        "ASC",
        "DESC",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "UNION",
        "ALL",
        "INTERSECT",
        "EXCEPT",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "SET",
        "DELETE",
        "CREATE",
        "TABLE",
        "VIEW",
        "DROP",
        "ALTER",
        "INDEX",
        "WITH",
        "RECURSIVE",
        "CASE",
        "WHEN",
        "THEN",
        "ELSE",
        "END",
        "EXISTS",
        "BETWEEN",
        "LIKE",
        "ILIKE",
        "DISTINCT",
        "COUNT",
        "SUM",
        "AVG",
        "MIN",
        "MAX",
        "CAST",
        "COALESCE",
        "NULLIF",
        "EXPLAIN",
        "ANALYZE",
        "TRUE",
        "FALSE",
        "OVER",
        "PARTITION",
        "WINDOW",
        "ROWS",
        "RANGE",
        "UNBOUNDED",
        "PRECEDING",
        "FOLLOWING",
        "CURRENT",
        "ROW",
        "FETCH",
        "NEXT",
        "FIRST",
        "LAST",
        "ONLY",
    ];

    let functions = [
        "now",
        "date_trunc",
        "extract",
        "to_timestamp",
        "array_agg",
        "string_agg",
        "row_number",
        "rank",
        "dense_rank",
        "lag",
        "lead",
        "first_value",
        "last_value",
        "ntile",
    ];

    if line.is_empty() {
        return Line::from("");
    }

    let mut spans: Vec<Span> = Vec::new();
    let mut chars = line.char_indices().peekable();

    while let Some(&(start, ch)) = chars.peek() {
        // ── Comment (-- to end of line) ───────────────────────
        if ch == '-' {
            let rest = &line[start..];
            if rest.starts_with("--") {
                spans.push(Span::styled(&line[start..], app.theme.sql_comment));
                break;
            }
        }

        // ── String literal ────────────────────────────────────
        if ch == '\'' {
            chars.next();
            let mut end = start + 1;
            while let Some(&(i, c)) = chars.peek() {
                chars.next();
                end = i + c.len_utf8();
                if c == '\'' {
                    break;
                }
            }
            spans.push(Span::styled(&line[start..end], app.theme.sql_string));
            continue;
        }

        // ── Number ────────────────────────────────────────────
        if ch.is_ascii_digit()
            || (ch == '.'
                && line[start..].len() > 1
                && line
                    .as_bytes()
                    .get(start + 1)
                    .map(|b| b.is_ascii_digit())
                    .unwrap_or(false))
        {
            chars.next();
            let mut end = start + ch.len_utf8();
            while let Some(&(i, c)) = chars.peek() {
                if c.is_ascii_digit() || c == '.' {
                    chars.next();
                    end = i + c.len_utf8();
                } else {
                    break;
                }
            }
            spans.push(Span::styled(&line[start..end], app.theme.sql_number));
            continue;
        }

        // ── Identifier / keyword ──────────────────────────────
        if ch.is_alphanumeric() || ch == '_' {
            chars.next();
            let mut end = start + ch.len_utf8();
            while let Some(&(i, c)) = chars.peek() {
                if c.is_alphanumeric() || c == '_' {
                    chars.next();
                    end = i + c.len_utf8();
                } else {
                    break;
                }
            }
            let word = &line[start..end];
            let upper = word.to_uppercase();

            if keywords.contains(&upper.as_str()) {
                spans.push(Span::styled(word, app.theme.sql_keyword));
            } else if functions.contains(&word.to_lowercase().as_str()) {
                spans.push(Span::styled(word, app.theme.sql_function));
            } else {
                spans.push(Span::styled(word, app.theme.sql_identifier));
            }
            continue;
        }

        // ── Operators ─────────────────────────────────────────
        if "=<>!+-*/%|&".contains(ch) {
            chars.next();
            let end = start + ch.len_utf8();
            spans.push(Span::styled(&line[start..end], app.theme.sql_operator));
            continue;
        }

        // ── Parameter placeholder ($N) ────────────────────────
        if ch == '$' {
            chars.next();
            let mut end = start + 1;
            while let Some(&(i, c)) = chars.peek() {
                if c.is_ascii_digit() {
                    chars.next();
                    end = i + 1;
                } else {
                    break;
                }
            }
            spans.push(Span::styled(
                &line[start..end],
                app.theme.type_temporal, // distinct color for parameters
            ));
            continue;
        }

        // ── Everything else (whitespace, parens, commas, etc.) ─
        chars.next();
        let end = start + ch.len_utf8();
        spans.push(Span::styled(
            &line[start..end],
            Style::default().fg(app.theme.editor_fg),
        ));
    }

    Line::from(spans)
}

pub fn render_editor(frame: &mut Frame, app: &App, area: Rect) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let tab = app.active_tab_ref();
    let line_number_width = 4u16;
    let is_focused = app.focused_pane == Pane::Editor;

    // ── Line numbers ──────────────────────────────────────────
    let lnum_area = Rect::new(area.x, area.y, line_number_width, area.height);
    let code_area = Rect::new(
        area.x + line_number_width,
        area.y,
        area.width.saturating_sub(line_number_width),
        area.height,
    );

    let visible_start = tab.scroll_offset;
    let visible_end = (visible_start + area.height as usize).min(tab.content.len());

    // Line numbers
    let lnum_lines: Vec<Line> = (visible_start..visible_end)
        .map(|i| {
            let style = if i == tab.cursor_row {
                app.theme.editor_cursor_line
            } else {
                app.theme.editor_line_number
            };
            Line::styled(format!("{:>3} ", i + 1), style)
        })
        .collect();

    let lnum_widget = Paragraph::new(lnum_lines).style(Style::default().bg(app.theme.editor_bg));
    frame.render_widget(lnum_widget, lnum_area);

    // ── Code lines with syntax highlighting ───────────────────
    let code_lines: Vec<Line> = (visible_start..visible_end)
        .map(|i| {
            let line = &tab.content[i];
            let mut highlighted = highlight_sql_line(line, app);

            // Apply cursor line background
            if i == tab.cursor_row {
                for span in highlighted.spans.iter_mut() {
                    span.style = span.style.bg(app
                        .theme
                        .editor_cursor_line
                        .bg
                        .unwrap_or(app.theme.editor_bg));
                }
            }

            highlighted
        })
        .collect();

    let code_widget = Paragraph::new(code_lines).style(
        Style::default()
            .bg(app.theme.editor_bg)
            .fg(app.theme.editor_fg),
    );
    frame.render_widget(code_widget, code_area);

    // ── Cursor ────────────────────────────────────────────────
    if is_focused {
        let cursor_y = area.y + (tab.cursor_row - visible_start) as u16;
        let cursor_x = code_area.x + tab.cursor_col as u16;

        if cursor_y < area.y + area.height && cursor_x < code_area.x + code_area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
