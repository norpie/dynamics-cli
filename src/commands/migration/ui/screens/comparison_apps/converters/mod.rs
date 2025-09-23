use crate::{
    commands::migration::ui::components::hierarchy_tree::TreeNode, dynamics::metadata::FieldInfo,
};
use std::collections::HashMap;

/// Comparison data structure for converters
#[derive(Clone)]
pub struct ComparisonData {
    pub source_fields: Vec<FieldInfo>,
    pub target_fields: Vec<FieldInfo>,
    pub source_views: Vec<crate::dynamics::metadata::ViewInfo>,
    pub target_views: Vec<crate::dynamics::metadata::ViewInfo>,
    pub source_forms: Vec<crate::dynamics::metadata::FormInfo>,
    pub target_forms: Vec<crate::dynamics::metadata::FormInfo>,
    pub field_mappings: HashMap<String, String>,
    pub prefix_mappings: HashMap<String, String>,
    pub hide_matched: bool,
}

/// Trait for converting comparison data into hierarchical tree nodes
/// This strategy pattern allows different app types to handle their data conversion
/// while sharing the same base UI implementation
pub trait DataConverter: Send + Sync {
    /// Convert comparison data into source and target tree nodes
    /// Now includes field metadata for accurate field resolution
    fn convert_data(
        &self,
        data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<TreeNode>, Vec<TreeNode>);

    /// Get the human-readable name for this app type
    fn get_app_name(&self) -> &'static str;
}

// Re-export converter implementation
pub mod unified_converter;

pub use unified_converter::UnifiedConverter;
