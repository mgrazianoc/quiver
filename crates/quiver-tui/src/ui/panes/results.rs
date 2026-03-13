use arrow::array::{Array, BooleanArray, RecordBatch};
use arrow::datatypes::{DataType, TimeUnit};
use ratatui::prelude::*;
use ratatui::widgets::{Cell, Row, Table};

use crate::app::App;
use crate::theme::Theme;

// ── Arrow type → compact badge string ─────────────────────────

fn type_badge(dt: &DataType) -> String {
    match dt {
        DataType::Null => "null".into(),
        DataType::Boolean => "bool".into(),
        DataType::Int8 => "i8".into(),
        DataType::Int16 => "i16".into(),
        DataType::Int32 => "i32".into(),
        DataType::Int64 => "i64".into(),
        DataType::UInt8 => "u8".into(),
        DataType::UInt16 => "u16".into(),
        DataType::UInt32 => "u32".into(),
        DataType::UInt64 => "u64".into(),
        DataType::Float16 => "f16".into(),
        DataType::Float32 => "f32".into(),
        DataType::Float64 => "f64".into(),
        DataType::Decimal128(p, s) => format!("dec({p},{s})"),
        DataType::Decimal256(p, s) => format!("dec256({p},{s})"),
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => "utf8".into(),
        DataType::Binary | DataType::LargeBinary | DataType::BinaryView => "bin".into(),
        DataType::FixedSizeBinary(n) => format!("bin[{n}]"),
        DataType::Date32 | DataType::Date64 => "date".into(),
        DataType::Timestamp(unit, tz) => {
            let u = match unit {
                TimeUnit::Second => "s",
                TimeUnit::Millisecond => "ms",
                TimeUnit::Microsecond => "μs",
                TimeUnit::Nanosecond => "ns",
            };
            match tz {
                Some(tz) => format!("ts[{u},{tz}]"),
                None => format!("ts[{u}]"),
            }
        }
        DataType::Time32(_) | DataType::Time64(_) => "time".into(),
        DataType::Duration(_) => "dur".into(),
        DataType::Interval(_) => "interval".into(),
        DataType::List(f) | DataType::LargeList(f) | DataType::ListView(f) => {
            format!("list<{}>", type_badge(f.data_type()))
        }
        DataType::FixedSizeList(f, n) => format!("[{};{n}]", type_badge(f.data_type())),
        DataType::Struct(_) => "struct{…}".into(),
        DataType::Map(_, _) => "map".into(),
        DataType::Dictionary(_, v) => format!("§{}", type_badge(v)),
        DataType::Union(_, _) => "union".into(),
        DataType::RunEndEncoded(_, _) => "ree".into(),
        _ => "·".into(),
    }
}

// ── Arrow type family → theme style for badge coloring ────────

fn type_family_style(dt: &DataType, theme: &Theme) -> Style {
    match dt {
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => theme.type_integer,
        DataType::Float16
        | DataType::Float32
        | DataType::Float64
        | DataType::Decimal128(..)
        | DataType::Decimal256(..) => theme.type_float,
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => theme.type_string,
        DataType::Boolean => theme.type_boolean,
        DataType::Date32
        | DataType::Date64
        | DataType::Timestamp(..)
        | DataType::Time32(..)
        | DataType::Time64(..)
        | DataType::Duration(..)
        | DataType::Interval(..) => theme.type_temporal,
        DataType::Binary
        | DataType::LargeBinary
        | DataType::BinaryView
        | DataType::FixedSizeBinary(..) => theme.type_binary,
        DataType::List(..)
        | DataType::LargeList(..)
        | DataType::ListView(..)
        | DataType::FixedSizeList(..)
        | DataType::Struct(..)
        | DataType::Map(..)
        | DataType::Union(..) => theme.type_nested,
        DataType::Dictionary(_, v) => type_family_style(v, theme),
        _ => theme.result_cell,
    }
}

// ── Format a single cell value from an Arrow array ────────────

fn format_cell_value(array: &dyn Array, row: usize) -> String {
    if array.is_null(row) {
        return String::new(); // NULL styling handled by caller
    }
    // Booleans as ✓/✗
    if let Some(arr) = array.as_any().downcast_ref::<BooleanArray>() {
        return if arr.value(row) {
            "✓".into()
        } else {
            "✗".into()
        };
    }
    arrow::util::display::array_value_to_string(array, row).unwrap_or_else(|_| "?".into())
}

// ── Resolve absolute row index → (batch, row_within_batch) ───

fn resolve_row(batches: &[RecordBatch], absolute_row: usize) -> Option<(&RecordBatch, usize)> {
    let mut remaining = absolute_row;
    for batch in batches {
        if remaining < batch.num_rows() {
            return Some((batch, remaining));
        }
        remaining -= batch.num_rows();
    }
    None
}

// ── Sample first N rows to compute column display widths ──────

fn sample_column_width(
    batches: &[RecordBatch],
    col_idx: usize,
    header_width: usize,
    max_sample: usize,
) -> u16 {
    let mut max_w = header_width;
    let mut count = 0;
    for batch in batches {
        let col = batch.column(col_idx);
        for row in 0..batch.num_rows() {
            if count >= max_sample {
                return max_w.min(48) as u16;
            }
            let w = format_cell_value(col.as_ref(), row).len();
            // Account for NULL display width
            let w = if col.is_null(row) { 4 } else { w }; // "NULL" = 4 chars
            max_w = max_w.max(w);
            count += 1;
        }
    }
    max_w.min(48) as u16
}

pub fn render_results(frame: &mut Frame, app: &App, area: Rect) {
    let schema = match &app.result_schema {
        Some(s) if !s.fields().is_empty() => s,
        _ => {
            let empty =
                ratatui::widgets::Paragraph::new("No results. Execute a query to see data here.")
                    .style(Style::default().fg(Color::DarkGray).bg(app.theme.bg))
                    .alignment(Alignment::Center);
            frame.render_widget(empty, area);
            return;
        }
    };

    if area.height == 0 || area.width == 0 {
        return;
    }

    let fields = schema.fields();
    let col_offset = app.result_col_offset;
    let visible_count = fields.len().saturating_sub(col_offset);
    if visible_count == 0 {
        return;
    }

    // ── Column widths (adaptive, sampled from Arrow arrays) ───
    let col_widths: Vec<u16> = (col_offset..fields.len())
        .map(|col_idx| {
            let field = &fields[col_idx];
            let badge = type_badge(field.data_type());
            let header_w = field.name().len() + badge.len() + 2; // +2 for spacing
            sample_column_width(&app.result_batches, col_idx, header_w, 100)
        })
        .collect();

    // ── Header row with real Arrow type badges ────────────────
    let header_cells: Vec<Cell> = (col_offset..fields.len())
        .map(|col_idx| {
            let field = &fields[col_idx];
            let badge = type_badge(field.data_type());
            let badge_style = type_family_style(field.data_type(), &app.theme);
            let nullable = if field.is_nullable() { "?" } else { "" };
            let line = Line::from(vec![
                Span::styled(field.name().as_str(), app.theme.result_header),
                Span::raw(" "),
                Span::styled(badge, badge_style),
                Span::styled(nullable, badge_style),
            ]);
            Cell::from(line)
        })
        .collect();
    let header_row = Row::new(header_cells)
        .style(app.theme.result_header)
        .height(1);

    // ── Ensure scroll offset keeps selection visible ──────────
    let table_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = if app.result_selected_row >= app.result_scroll_offset + table_height {
        app.result_selected_row.saturating_sub(table_height) + 1
    } else if app.result_selected_row < app.result_scroll_offset {
        app.result_selected_row
    } else {
        app.result_scroll_offset
    };

    // ── Data rows (formatted on-the-fly from Arrow arrays) ───
    let end_row = (scroll_offset + table_height).min(app.result_total_rows);
    let rows: Vec<Row> = (scroll_offset..end_row)
        .filter_map(|abs_row| {
            let (batch, row_in_batch) = resolve_row(&app.result_batches, abs_row)?;
            let is_selected = abs_row == app.result_selected_row;
            let is_alt = abs_row % 2 == 1;

            let base_style = if is_selected {
                app.theme.result_selected
            } else if is_alt {
                app.theme.result_cell_alt
            } else {
                app.theme.result_cell
            };

            let cells: Vec<Cell> = (col_offset..fields.len())
                .map(|col_idx| {
                    let col = batch.column(col_idx);
                    if col.is_null(row_in_batch) {
                        let null_style = if is_selected {
                            app.theme.result_selected
                        } else {
                            app.theme.result_null
                        };
                        Cell::from("NULL").style(null_style)
                    } else {
                        let value = format_cell_value(col.as_ref(), row_in_batch);
                        Cell::from(value).style(base_style)
                    }
                })
                .collect();

            Some(Row::new(cells).style(base_style))
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
