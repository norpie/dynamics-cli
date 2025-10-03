use std::sync::Arc;

use crate::commands::migration::ui::components::field_renderer::{MappingSource, MatchState};

/// Trait for items that can be displayed in a hierarchical tree structure
pub trait HierarchyNode: std::fmt::Debug + Send + Sync {
    /// Get the display name for this node
    fn display_name(&self) -> String;

    /// Get the clean name without icon or count formatting
    fn clean_name(&self) -> &str;

    /// Get the count of child items (for collapsed view)
    fn item_count(&self) -> usize;

    /// Check if this node can be expanded/collapsed
    fn is_expandable(&self) -> bool;

    /// Get the mapping target name if this node has a mapping
    fn mapping_target(&self) -> Option<String>;

    /// Get the mapping type (e.g., "exact", "prefix", "manual") if this node has a mapping
    fn mapping_type(&self) -> Option<String> {
        None
    }

    /// Get the unique key for this node (used for tracking expand/collapse state)
    fn node_key(&self) -> String;

    /// Check if this node represents a field that should use rich field rendering
    fn is_field_node(&self) -> bool {
        false
    }

    /// Get field information for rich rendering (if this is a field node)
    fn get_field_info(&self) -> Option<FieldRenderingInfo> {
        None
    }
}

/// Information needed for rich field rendering
#[derive(Debug, Clone)]
pub struct FieldRenderingInfo {
    pub field_name: String,
    pub field_type: String,
    pub is_required: bool,
    pub mapping_target: Option<String>,
    pub mapping_source: Option<MappingSource>,
    pub match_state: MatchState,
}

/// Represents a node in the hierarchy tree with expansion state
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub data: Arc<dyn HierarchyNode>,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
    pub level: usize,
}

// Re-export all components
pub mod tree_node;
pub mod tree_renderer;
pub mod tree_state;

pub use tree_state::{HierarchyTree, SortMode};
