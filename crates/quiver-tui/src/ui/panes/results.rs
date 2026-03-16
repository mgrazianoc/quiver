use arrow::array::{Array, BooleanArray, RecordBatch};
use arrow::compute::kernels::aggregate;
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

pub fn render_results(frame: &mut Frame, app: &mut App, area: Rect) {
    let schema = match &app.result_schema {
        Some(s) if !s.fields().is_empty() => s.clone(),
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

    // Get the display batches (sorted if sort is active)
    let batches = app.display_batches();

    let fields = schema.fields();
    let col_offset = app.result_col_offset;
    let visible_count = fields.len().saturating_sub(col_offset);
    if visible_count == 0 {
        return;
    }

    // ── Row-index column width ────────────────────────────────
    let row_index_width = {
        let digits = if app.result_total_rows == 0 {
            1
        } else {
            (app.result_total_rows as f64).log10().floor() as u16 + 1
        };
        digits.max(1)
    };

    // ── Column widths (adaptive, sampled from Arrow arrays) ───
    let col_widths: Vec<u16> = (col_offset..fields.len())
        .map(|col_idx| {
            let field = &fields[col_idx];
            let badge = type_badge(field.data_type());
            let sort_indicator_len = if app.result_sort_column == Some(col_idx) {
                2
            } else {
                0
            };
            let header_w = field.name().len() + badge.len() + 2 + sort_indicator_len;
            sample_column_width(&batches, col_idx, header_w, 100)
        })
        .collect();

    // ── Header row with real Arrow type badges + sort indicators ──
    let mut header_cells: Vec<Cell> = Vec::with_capacity(visible_count + 1);
    header_cells.push(Cell::from("").style(Style::default().fg(Color::DarkGray).bg(app.theme.bg)));
    header_cells.extend((col_offset..fields.len()).map(|col_idx| {
        let field = &fields[col_idx];
        let badge = type_badge(field.data_type());
        let badge_style = type_family_style(field.data_type(), &app.theme);
        let nullable = if field.is_nullable() { "?" } else { "" };

        let sort_indicator = if app.result_sort_column == Some(col_idx) {
            if app.result_sort_ascending {
                " ▲"
            } else {
                " ▼"
            }
        } else {
            ""
        };

        let line = Line::from(vec![
            Span::styled(field.name().as_str(), app.theme.result_header),
            Span::raw(" "),
            Span::styled(badge, badge_style),
            Span::styled(nullable, badge_style),
            Span::styled(sort_indicator, Style::default().fg(app.theme.accent)),
        ]);
        Cell::from(line)
    }));
    let header_row = Row::new(header_cells)
        .style(app.theme.result_header)
        .height(1);

    // ── Stats row height ──────────────────────────────────────
    let stats_height: u16 = if app.show_column_stats { 1 } else { 0 };

    // ── Ensure scroll offset keeps selection visible ──────────
    let table_height = area.height.saturating_sub(2 + stats_height) as usize;
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
            let (batch, row_in_batch) = resolve_row(&batches, abs_row)?;
            let is_cursor = abs_row == app.result_selected_row;
            let is_multi_selected = app.result_selected_rows.contains(&abs_row);
            let is_alt = abs_row % 2 == 1;

            let base_style = if is_cursor {
                app.theme.result_selected
            } else if is_multi_selected {
                Style::default()
                    .fg(app.theme.accent)
                    .bg(app.theme.bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_alt {
                app.theme.result_cell_alt
            } else {
                app.theme.result_cell
            };

            let mut cells: Vec<Cell> = Vec::with_capacity(visible_count + 1);
            // Row number (1-based)
            cells.push(
                Cell::from(format!("{}", abs_row + 1))
                    .style(Style::default().fg(Color::DarkGray).bg(app.theme.bg)),
            );
            cells.extend((col_offset..fields.len()).map(|col_idx| {
                let col = batch.column(col_idx);
                if col.is_null(row_in_batch) {
                    let null_style = if is_cursor {
                        app.theme.result_selected
                    } else {
                        app.theme.result_null
                    };
                    Cell::from("NULL").style(null_style)
                } else {
                    let value = format_cell_value(col.as_ref(), row_in_batch);
                    Cell::from(value).style(base_style)
                }
            }));

            Some(Row::new(cells).style(base_style))
        })
        .collect();

    // ── Build table ───────────────────────────────────────────
    let mut constraints: Vec<Constraint> = Vec::with_capacity(col_widths.len() + 1);
    constraints.push(Constraint::Length(row_index_width));
    constraints.extend(col_widths.iter().map(|&w| Constraint::Length(w)));

    // ── Column stats footer ──────────────────────────────────
    let (main_area, stats_area) = if app.show_column_stats {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let table = Table::new(rows, &constraints)
        .header(header_row)
        .style(Style::default().bg(app.theme.bg))
        .column_spacing(1)
        .row_highlight_style(app.theme.result_selected);

    frame.render_widget(table, main_area);

    // ── Render column statistics row ──────────────────────────
    if let Some(stats_area) = stats_area {
        let mut stats_cells: Vec<Cell> = Vec::with_capacity(visible_count + 1);
        stats_cells
            .push(Cell::from("").style(Style::default().fg(Color::DarkGray).bg(app.theme.bg)));
        stats_cells.extend((col_offset..fields.len()).map(|col_idx| {
            let stats = compute_column_stats(&batches, col_idx);
            Cell::from(stats).style(Style::default().fg(Color::DarkGray).bg(app.theme.bg))
        }));
        let stats_row = Row::new(stats_cells);
        let stats_table = Table::new(vec![stats_row], &constraints)
            .style(Style::default().bg(app.theme.bg))
            .column_spacing(1);
        frame.render_widget(stats_table, stats_area);
    }
}

// ── Compute column statistics (null count, min, max) ──────────

fn compute_column_stats(batches: &[RecordBatch], col_idx: usize) -> String {
    if batches.is_empty() {
        return String::new();
    }

    let mut total_rows = 0usize;
    let mut null_count = 0usize;

    for batch in batches {
        if col_idx >= batch.num_columns() {
            return String::new();
        }
        let col = batch.column(col_idx);
        total_rows += col.len();
        null_count += col.null_count();
    }

    // Try to compute min/max for numeric types
    let dt = batches[0].schema().field(col_idx).data_type().clone();
    let has_numeric_stats = matches!(
        dt,
        DataType::Int8
            | DataType::Int16
            | DataType::Int32
            | DataType::Int64
            | DataType::UInt8
            | DataType::UInt16
            | DataType::UInt32
            | DataType::UInt64
            | DataType::Float32
            | DataType::Float64
    );

    if has_numeric_stats && batches.len() == 1 {
        let col = batches[0].column(col_idx);
        let min_str = aggregate_min_string(col.as_ref());
        let max_str = aggregate_max_string(col.as_ref());
        format!(
            "n={} null={} min={} max={}",
            total_rows, null_count, min_str, max_str
        )
    } else {
        format!("n={} null={}", total_rows, null_count)
    }
}

fn aggregate_min_string(array: &dyn Array) -> String {
    use arrow::array::*;
    macro_rules! min_val {
        ($arr_type:ty, $array:expr) => {
            if let Some(arr) = $array.as_any().downcast_ref::<$arr_type>() {
                return aggregate::min(arr)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into());
            }
        };
    }
    min_val!(Int8Array, array);
    min_val!(Int16Array, array);
    min_val!(Int32Array, array);
    min_val!(Int64Array, array);
    min_val!(UInt8Array, array);
    min_val!(UInt16Array, array);
    min_val!(UInt32Array, array);
    min_val!(UInt64Array, array);
    min_val!(Float32Array, array);
    min_val!(Float64Array, array);
    "—".into()
}

fn aggregate_max_string(array: &dyn Array) -> String {
    use arrow::array::*;
    macro_rules! max_val {
        ($arr_type:ty, $array:expr) => {
            if let Some(arr) = $array.as_any().downcast_ref::<$arr_type>() {
                return aggregate::max(arr)
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".into());
            }
        };
    }
    max_val!(Int8Array, array);
    max_val!(Int16Array, array);
    max_val!(Int32Array, array);
    max_val!(Int64Array, array);
    max_val!(UInt8Array, array);
    max_val!(UInt16Array, array);
    max_val!(UInt32Array, array);
    max_val!(UInt64Array, array);
    max_val!(Float32Array, array);
    max_val!(Float64Array, array);
    "—".into()
}
