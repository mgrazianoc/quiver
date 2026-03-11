//! Schema catalog types and introspection logic.
//!
//! Provides [`TreeNode`] types for the schema browser and helper
//! functions to build a tree from Flight SQL catalog RecordBatches.

use std::collections::BTreeMap;

use arrow::array::{Array, RecordBatch, StringArray};
use serde::{Deserialize, Serialize};

// ── Schema tree types ─────────────────────────────────────────

/// A node in the schema browser tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    pub label: String,
    pub kind: TreeNodeKind,
    pub depth: usize,
    #[serde(default)]
    pub expanded: bool,
    #[serde(default)]
    pub children: Vec<TreeNode>,
}

/// The kind of object a tree node represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreeNodeKind {
    Catalog,
    Schema,
    Table,
    View,
    Column,
}

impl TreeNodeKind {
    pub fn icon(&self) -> &'static str {
        match self {
            TreeNodeKind::Catalog => "◆",
            TreeNodeKind::Schema => "◇",
            TreeNodeKind::Table => "▦",
            TreeNodeKind::View => "▤",
            TreeNodeKind::Column => "│",
        }
    }
}

/// A flattened tree node for display in a list.
#[derive(Debug, Clone)]
pub struct FlatNode {
    pub depth: usize,
    pub label: String,
    pub kind: TreeNodeKind,
    pub has_children: bool,
    pub expanded: bool,
}

impl TreeNode {
    /// Flatten tree into a display list for rendering.
    pub fn flatten(&self) -> Vec<FlatNode> {
        let mut out = Vec::new();
        self.flatten_into(&mut out);
        out
    }

    fn flatten_into(&self, out: &mut Vec<FlatNode>) {
        out.push(FlatNode {
            depth: self.depth,
            label: self.label.clone(),
            kind: self.kind,
            has_children: !self.children.is_empty(),
            expanded: self.expanded,
        });
        if self.expanded {
            for child in &self.children {
                child.flatten_into(out);
            }
        }
    }
}

// ── Table descriptor ──────────────────────────────────────────

/// A table/view with its owning catalog and schema.
#[derive(Debug, Clone)]
pub struct TableInfo {
    pub catalog: String,
    pub schema: String,
    pub table_name: String,
    pub table_type: String,
}

// ── Column descriptor ─────────────────────────────────────────

/// A column with type information.
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

// ── RecordBatch extraction helpers ────────────────────────────

/// Extract a string column from a RecordBatch by name.
fn string_column<'a>(batch: &'a RecordBatch, name: &str) -> Option<&'a StringArray> {
    let idx = batch.schema().index_of(name).ok()?;
    batch.column(idx).as_any().downcast_ref::<StringArray>()
}

/// Extract tables from `get_tables` response batches.
pub fn extract_tables(batches: &[RecordBatch]) -> Vec<TableInfo> {
    let mut tables = Vec::new();
    for batch in batches {
        let catalogs = string_column(batch, "catalog_name");
        let schemas = string_column(batch, "db_schema_name");
        let names = match string_column(batch, "table_name") {
            Some(a) => a,
            None => continue,
        };
        let types = string_column(batch, "table_type");

        for row in 0..batch.num_rows() {
            tables.push(TableInfo {
                catalog: catalogs
                    .and_then(|a| {
                        if a.is_null(row) {
                            None
                        } else {
                            Some(a.value(row))
                        }
                    })
                    .unwrap_or("")
                    .to_string(),
                schema: schemas
                    .and_then(|a| {
                        if a.is_null(row) {
                            None
                        } else {
                            Some(a.value(row))
                        }
                    })
                    .unwrap_or("")
                    .to_string(),
                table_name: names.value(row).to_string(),
                table_type: types
                    .and_then(|a| {
                        if a.is_null(row) {
                            None
                        } else {
                            Some(a.value(row))
                        }
                    })
                    .unwrap_or("TABLE")
                    .to_string(),
            });
        }
    }
    tables
}

/// Extract column info from a table's Arrow schema.
pub fn extract_columns(schema: &arrow_schema::Schema) -> Vec<ColumnInfo> {
    schema
        .fields()
        .iter()
        .map(|f| ColumnInfo {
            name: f.name().clone(),
            data_type: format!("{}", f.data_type()),
        })
        .collect()
}

// ── Tree builder ──────────────────────────────────────────────

/// Build a schema browser tree from tables and their optional column info.
///
/// `tables_with_columns` pairs each [`TableInfo`] with an optional list of
/// [`ColumnInfo`]. When columns are `None`, the table node has no children.
pub fn build_schema_tree(
    tables_with_columns: &[(TableInfo, Option<Vec<ColumnInfo>>)],
) -> Vec<TreeNode> {
    // catalog → schema → Vec<(table_info, columns)>
    type SchemaMap<'a> = BTreeMap<&'a str, Vec<(&'a TableInfo, &'a Option<Vec<ColumnInfo>>)>>;
    let mut catalogs: BTreeMap<&str, SchemaMap> = BTreeMap::new();

    for (table, cols) in tables_with_columns {
        catalogs
            .entry(&table.catalog)
            .or_default()
            .entry(&table.schema)
            .or_default()
            .push((table, cols));
    }

    catalogs
        .into_iter()
        .map(|(cat_name, schemas)| {
            let schema_children: Vec<TreeNode> = schemas
                .into_iter()
                .map(|(schema_name, tables)| {
                    let table_children: Vec<TreeNode> = tables
                        .into_iter()
                        .map(|(tinfo, cols)| {
                            let kind = if tinfo.table_type.eq_ignore_ascii_case("VIEW") {
                                TreeNodeKind::View
                            } else {
                                TreeNodeKind::Table
                            };

                            let col_children = cols
                                .as_ref()
                                .map(|cols| {
                                    cols.iter()
                                        .map(|c| TreeNode {
                                            label: format!("{}: {}", c.name, c.data_type),
                                            kind: TreeNodeKind::Column,
                                            depth: 3,
                                            expanded: false,
                                            children: vec![],
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            TreeNode {
                                label: tinfo.table_name.clone(),
                                kind,
                                depth: 2,
                                expanded: false,
                                children: col_children,
                            }
                        })
                        .collect();

                    TreeNode {
                        label: schema_name.to_string(),
                        kind: TreeNodeKind::Schema,
                        depth: 1,
                        expanded: true,
                        children: table_children,
                    }
                })
                .collect();

            let cat_label = if cat_name.is_empty() {
                "(default)".to_string()
            } else {
                cat_name.to_string()
            };

            TreeNode {
                label: cat_label,
                kind: TreeNodeKind::Catalog,
                depth: 0,
                expanded: true,
                children: schema_children,
            }
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use arrow::array::StringBuilder;
    use arrow_schema::{DataType, Field, Schema};

    fn make_tables_batch(rows: &[(&str, &str, &str, &str)]) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("catalog_name", DataType::Utf8, true),
            Field::new("db_schema_name", DataType::Utf8, true),
            Field::new("table_name", DataType::Utf8, false),
            Field::new("table_type", DataType::Utf8, false),
        ]));

        let mut cat = StringBuilder::new();
        let mut sch = StringBuilder::new();
        let mut name = StringBuilder::new();
        let mut typ = StringBuilder::new();

        for (c, s, n, t) in rows {
            cat.append_value(c);
            sch.append_value(s);
            name.append_value(n);
            typ.append_value(t);
        }

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(cat.finish()),
                Arc::new(sch.finish()),
                Arc::new(name.finish()),
                Arc::new(typ.finish()),
            ],
        )
        .unwrap()
    }

    #[test]
    fn extract_tables_from_batch() {
        let batch = make_tables_batch(&[
            ("mycat", "public", "users", "TABLE"),
            ("mycat", "public", "orders", "TABLE"),
            ("mycat", "public", "active_users", "VIEW"),
        ]);
        let tables = extract_tables(&[batch]);
        assert_eq!(tables.len(), 3);
        assert_eq!(tables[0].table_name, "users");
        assert_eq!(tables[2].table_type, "VIEW");
    }

    #[test]
    fn extract_columns_from_schema() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("score", DataType::Float64, true),
        ]);
        let cols = extract_columns(&schema);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].name, "id");
        assert_eq!(cols[0].data_type, "Int64");
        assert_eq!(cols[1].data_type, "Utf8");
    }

    #[test]
    fn build_tree_groups_by_catalog_and_schema() {
        let tables = vec![
            (
                TableInfo {
                    catalog: "db".into(),
                    schema: "public".into(),
                    table_name: "users".into(),
                    table_type: "TABLE".into(),
                },
                Some(vec![
                    ColumnInfo {
                        name: "id".into(),
                        data_type: "Int64".into(),
                    },
                    ColumnInfo {
                        name: "name".into(),
                        data_type: "Utf8".into(),
                    },
                ]),
            ),
            (
                TableInfo {
                    catalog: "db".into(),
                    schema: "public".into(),
                    table_name: "logs".into(),
                    table_type: "TABLE".into(),
                },
                None,
            ),
            (
                TableInfo {
                    catalog: "db".into(),
                    schema: "stats".into(),
                    table_name: "summary".into(),
                    table_type: "VIEW".into(),
                },
                None,
            ),
        ];

        let tree = build_schema_tree(&tables);

        // One catalog
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].label, "db");
        assert_eq!(tree[0].kind, TreeNodeKind::Catalog);

        // Two schemas
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].label, "public");
        assert_eq!(tree[0].children[1].label, "stats");

        // public has 2 tables
        let public = &tree[0].children[0];
        assert_eq!(public.children.len(), 2);
        assert_eq!(public.children[0].label, "users");
        assert_eq!(public.children[0].kind, TreeNodeKind::Table);
        assert_eq!(public.children[0].children.len(), 2); // columns
        assert_eq!(public.children[0].children[0].label, "id: Int64");

        // logs has no columns
        assert_eq!(public.children[1].label, "logs");
        assert!(public.children[1].children.is_empty());

        // stats has a view
        let stats = &tree[0].children[1];
        assert_eq!(stats.children[0].label, "summary");
        assert_eq!(stats.children[0].kind, TreeNodeKind::View);
    }

    #[test]
    fn empty_catalog_name_becomes_default() {
        let tables = vec![(
            TableInfo {
                catalog: String::new(),
                schema: "main".into(),
                table_name: "t".into(),
                table_type: "TABLE".into(),
            },
            None,
        )];
        let tree = build_schema_tree(&tables);
        assert_eq!(tree[0].label, "(default)");
    }

    #[test]
    fn empty_input_produces_empty_tree() {
        let tree = build_schema_tree(&[]);
        assert!(tree.is_empty());
    }

    #[test]
    fn flatten_preserves_existing_behavior() {
        let node = TreeNode {
            label: "cat".into(),
            kind: TreeNodeKind::Catalog,
            depth: 0,
            expanded: true,
            children: vec![TreeNode {
                label: "schema".into(),
                kind: TreeNodeKind::Schema,
                depth: 1,
                expanded: false,
                children: vec![TreeNode {
                    label: "tbl".into(),
                    kind: TreeNodeKind::Table,
                    depth: 2,
                    expanded: false,
                    children: vec![],
                }],
            }],
        };

        let flat = node.flatten();
        // Schema is collapsed, so only catalog + schema are visible
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].label, "cat");
        assert!(flat[0].has_children);
        assert_eq!(flat[1].label, "schema");
        assert!(flat[1].has_children);
        assert!(!flat[1].expanded);
    }
}
