//! Schema catalog types and future introspection logic.

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
