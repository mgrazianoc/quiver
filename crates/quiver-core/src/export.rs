//! Export query results to CSV, JSON, Parquet, and clipboard.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow::array::RecordBatch;
use arrow::datatypes::SchemaRef;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Json,
    Parquet,
}

impl ExportFormat {
    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Json => "JSON",
            ExportFormat::Parquet => "Parquet",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Parquet => "parquet",
        }
    }
}

/// Export `RecordBatch`es to a file in the given format.
pub fn export_to_file(
    batches: &[RecordBatch],
    schema: &SchemaRef,
    path: &Path,
    format: ExportFormat,
) -> Result<usize> {
    if batches.is_empty() {
        anyhow::bail!("No results to export");
    }

    match format {
        ExportFormat::Csv => write_csv(batches, path),
        ExportFormat::Json => write_json(batches, schema, path),
        ExportFormat::Parquet => write_parquet(batches, schema, path),
    }
}

/// Export results to a CSV string (for clipboard).
pub fn export_to_csv_string(batches: &[RecordBatch]) -> Result<String> {
    if batches.is_empty() {
        anyhow::bail!("No results to export");
    }

    let mut buf = Vec::new();
    {
        let mut writer = arrow::csv::WriterBuilder::new()
            .with_header(true)
            .build(&mut buf);
        for batch in batches {
            writer
                .write(batch)
                .context("Failed to write CSV to buffer")?;
        }
    }
    String::from_utf8(buf).context("CSV output is not valid UTF-8")
}

// ── CSV ───────────────────────────────────────────────────────

fn write_csv(batches: &[RecordBatch], path: &Path) -> Result<usize> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create file: {}", path.display()))?;
    let mut writer = arrow::csv::WriterBuilder::new()
        .with_header(true)
        .build(file);

    let mut total_rows = 0;
    for batch in batches {
        total_rows += batch.num_rows();
        writer.write(batch).context("Failed to write CSV batch")?;
    }
    Ok(total_rows)
}

// ── JSON ──────────────────────────────────────────────────────

fn write_json(batches: &[RecordBatch], schema: &SchemaRef, path: &Path) -> Result<usize> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create file: {}", path.display()))?;
    let mut writer = arrow::json::LineDelimitedWriter::new(file);

    let mut total_rows = 0;
    for batch in batches {
        total_rows += batch.num_rows();
        writer.write(batch).context("Failed to write JSON batch")?;
    }
    writer.finish().context("Failed to finalize JSON output")?;
    Ok(total_rows)
}

// ── Parquet ───────────────────────────────────────────────────

fn write_parquet(batches: &[RecordBatch], schema: &SchemaRef, path: &Path) -> Result<usize> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Cannot create file: {}", path.display()))?;

    let props = parquet::file::properties::WriterProperties::builder()
        .set_compression(parquet::basic::Compression::SNAPPY)
        .build();

    let mut writer = parquet::arrow::ArrowWriter::try_new(file, Arc::clone(schema), Some(props))?;

    let mut total_rows = 0;
    for batch in batches {
        total_rows += batch.num_rows();
        writer
            .write(batch)
            .context("Failed to write Parquet batch")?;
    }
    writer.close().context("Failed to finalize Parquet file")?;
    Ok(total_rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Float64Array, Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    fn test_batches() -> (Vec<RecordBatch>, SchemaRef) {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("value", DataType::Float64, true),
        ]));
        let batch = RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(Int64Array::from(vec![1, 2, 3])),
                Arc::new(StringArray::from(vec![
                    Some("alice"),
                    None,
                    Some("charlie"),
                ])),
                Arc::new(Float64Array::from(vec![Some(1.1), Some(2.2), None])),
            ],
        )
        .unwrap();
        (vec![batch], schema)
    }

    #[test]
    fn export_csv_file() {
        let (batches, schema) = test_batches();
        let dir = std::env::temp_dir().join("quiver_test_csv");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.csv");
        let rows = export_to_file(&batches, &schema, &path, ExportFormat::Csv).unwrap();
        assert_eq!(rows, 3);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("id,name,value"));
        assert!(content.contains("alice"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_json_file() {
        let (batches, schema) = test_batches();
        let dir = std::env::temp_dir().join("quiver_test_json");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.json");
        let rows = export_to_file(&batches, &schema, &path, ExportFormat::Json).unwrap();
        assert_eq!(rows, 3);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("alice"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_parquet_file() {
        let (batches, schema) = test_batches();
        let dir = std::env::temp_dir().join("quiver_test_parquet");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.parquet");
        let rows = export_to_file(&batches, &schema, &path, ExportFormat::Parquet).unwrap();
        assert_eq!(rows, 3);
        assert!(path.exists());
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_csv_string() {
        let (batches, _) = test_batches();
        let csv = export_to_csv_string(&batches).unwrap();
        assert!(csv.contains("id,name,value"));
        assert!(csv.contains("alice"));
    }

    #[test]
    fn export_empty_batches_errors() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", DataType::Int64, false)]));
        let result = export_to_file(&[], &schema, Path::new("/tmp/nope.csv"), ExportFormat::Csv);
        assert!(result.is_err());
    }

    #[test]
    fn format_labels_and_extensions() {
        assert_eq!(ExportFormat::Csv.label(), "CSV");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Json.label(), "JSON");
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Parquet.label(), "Parquet");
        assert_eq!(ExportFormat::Parquet.extension(), "parquet");
    }
}
