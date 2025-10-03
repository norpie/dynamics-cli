use crate::{
    commands::migration::ui::components::{
        field_renderer::{MappingSource, MatchState},
        hierarchy_tree::{FieldRenderingInfo, HierarchyNode as HierarchyNodeTrait},
    },
    dynamics::metadata::FieldInfo,
};
use std::collections::HashMap;

/// Unified hierarchy node that replaces ViewNode, FormNode, etc.
/// All tree structures use this same node type with different metadata
#[derive(Debug, Clone)]
pub struct UnifiedHierarchyNode {
    // Core data - every node has these
    pub name: String,
    pub children: Vec<UnifiedHierarchyNode>, // Always present, empty for leaf nodes

    // Field-specific data - only Some() for actual field nodes
    pub field_info: Option<FieldInfo>,

    // Mapping data - every node can have mappings
    pub mapping_target: Option<String>,
    pub mapping_type: MappingType,

    // UI/Display data
    pub icon: String,      // "📁", "📄", "📝", etc.
    pub node_level: u8,    // 0=top level, 1=second level, etc.
    pub is_expanded: bool, // UI state for expand/collapse
    pub item_count: usize, // Number of items for display purposes

    // Optional metadata for different node types
    pub node_type: NodeType, // What kind of node this represents
}

/// Type of node for display and behavioral hints
#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    // Form hierarchy
    FormType,
    Form,
    Tab,
    Section,
    FormField,

    // View hierarchy
    ViewType,
    View,
    ViewComponent,
    ViewItem,

    // Relationship hierarchy
    RelationshipGroup,
    RelationshipField,

    // Generic field node
    Field,
}

/// Mapping type for field relationships
#[derive(Debug, Clone, PartialEq)]
pub enum MappingType {
    Unmapped,
    Exact,
    Prefix,
    Manual,
    FullMatch,
    Mixed, // Hierarchical node with partial child matches (Yellow)
}

impl MappingType {
    pub fn icon(&self) -> &'static str {
        match self {
            MappingType::Unmapped => "",
            MappingType::Exact => "✓",
            MappingType::Prefix => "≈",
            MappingType::Manual => "✋",
            MappingType::FullMatch => "✅",
            MappingType::Mixed => "🟨",
        }
    }
}

impl UnifiedHierarchyNode {
    /// Create a new container node (non-field)
    pub fn new_container(name: String, node_type: NodeType, icon: String, node_level: u8) -> Self {
        Self {
            name,
            children: Vec::new(),
            field_info: None,
            mapping_target: None,
            mapping_type: MappingType::Unmapped,
            icon,
            node_level,
            is_expanded: false, // Start collapsed
            item_count: 0,
            node_type,
        }
    }

    /// Create a new field node (leaf)
    pub fn new_field(
        name: String,
        field_info: FieldInfo,
        node_type: NodeType,
        node_level: u8,
    ) -> Self {
        let icon = Self::get_field_icon(&field_info);
        Self {
            name,
            children: Vec::new(), // Fields never have children
            field_info: Some(field_info),
            mapping_target: None,
            mapping_type: MappingType::Unmapped,
            icon,
            node_level,
            is_expanded: false, // Not relevant for fields
            item_count: 0,      // Not relevant for fields
            node_type,
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: UnifiedHierarchyNode) {
        self.children.push(child);
        self.item_count = self.children.len();
    }

    /// Add multiple children
    pub fn add_children(&mut self, children: Vec<UnifiedHierarchyNode>) {
        self.children.extend(children);
        self.item_count = self.children.len();
    }

    /// Check if this node can be expanded (has children)
    pub fn is_expandable(&self) -> bool {
        !self.children.is_empty()
    }

    /// Check if this is a field node
    pub fn is_field_node(&self) -> bool {
        self.field_info.is_some()
    }

    /// Toggle expansion state
    pub fn toggle_expansion(&mut self) {
        if self.is_expandable() {
            self.is_expanded = !self.is_expanded;

            // If collapsing, recursively collapse all children
            if !self.is_expanded {
                self.collapse_all_children();
            }
        }
    }

    /// Recursively collapse all children
    fn collapse_all_children(&mut self) {
        for child in &mut self.children {
            child.is_expanded = false;
            child.collapse_all_children();
        }
    }

    /// Get appropriate icon for a field based on its properties
    fn get_field_icon(field_info: &FieldInfo) -> String {
        let name_lower = field_info.name.to_lowercase();

        // Check field name patterns for appropriate icons
        if name_lower.contains("name") {
            "📝".to_string()
        } else if name_lower.contains("phone") || name_lower.contains("telephone") {
            "📞".to_string()
        } else if name_lower.contains("email") {
            "📧".to_string()
        } else if name_lower.contains("address") || name_lower.contains("street") {
            "🏠".to_string()
        } else if name_lower.contains("city") {
            "🏙️".to_string()
        } else if name_lower.contains("country") || name_lower.contains("region") {
            "🌍".to_string()
        } else if name_lower.contains("contact") || name_lower.contains("lookup") {
            "🔗".to_string()
        } else if field_info.is_required {
            "📝".to_string()
        } else {
            "📄".to_string()
        }
    }

    /// Get icon for the node type
    pub fn get_type_icon(node_type: &NodeType) -> String {
        match node_type {
            NodeType::FormType | NodeType::ViewType => "📁".to_string(),
            NodeType::Form | NodeType::View => "📄".to_string(),
            NodeType::Tab => "📑".to_string(),
            NodeType::Section => "📋".to_string(),
            NodeType::ViewComponent => "🔧".to_string(),
            NodeType::RelationshipGroup => "🔗".to_string(),
            _ => "📝".to_string(), // Default for fields
        }
    }
}

impl HierarchyNodeTrait for UnifiedHierarchyNode {
    fn display_name(&self) -> String {
        // Simple, clean display format - no mapping indicators here
        // Mapping indicators are handled by the renderer separately
        let base_name = match self.node_type {
            NodeType::FormType | NodeType::ViewType => {
                format!("{} ({})", self.name, self.item_count)
            }
            _ => self.name.clone(),
        };

        format!("{} {}", self.icon, base_name)
    }

    fn clean_name(&self) -> &str {
        &self.name
    }

    fn node_key(&self) -> String {
        // Create a unique key for this node based on name and level
        format!("{}_{}", self.name, self.node_level)
    }

    fn is_field_node(&self) -> bool {
        self.field_info.is_some()
    }

    fn get_field_info(&self) -> Option<FieldRenderingInfo> {
        if let Some(field_info) = &self.field_info {
            let match_state = match self.mapping_type {
                MappingType::FullMatch => MatchState::FullMatch,
                MappingType::Exact | MappingType::Prefix | MappingType::Manual => {
                    MatchState::FullMatch
                }
                MappingType::Mixed => MatchState::MixedMatch,
                MappingType::Unmapped => MatchState::NoMatch,
            };

            let mapping_source = match self.mapping_type {
                MappingType::Exact => Some(MappingSource::Exact),
                MappingType::Prefix => Some(MappingSource::Prefix),
                MappingType::Manual => Some(MappingSource::Manual),
                _ => None,
            };

            Some(FieldRenderingInfo {
                field_name: field_info.name.clone(),
                field_type: field_info.field_type.clone(),
                is_required: field_info.is_required,
                match_state,
                mapping_target: self.mapping_target.clone(),
                mapping_source,
            })
        } else {
            None
        }
    }

    fn is_expandable(&self) -> bool {
        self.is_expandable()
    }

    fn mapping_target(&self) -> Option<String> {
        self.mapping_target.clone()
    }

    fn mapping_type(&self) -> Option<String> {
        match self.mapping_type {
            MappingType::Unmapped => None,
            MappingType::Exact => Some("exact".to_string()),
            MappingType::Prefix => Some("prefix".to_string()),
            MappingType::Manual => Some("manual".to_string()),
            MappingType::FullMatch => Some("exact".to_string()), // FullMatch is similar to exact
            MappingType::Mixed => Some("mixed".to_string()),
        }
    }

    fn item_count(&self) -> usize {
        self.item_count
    }
}

/// Unified matching logic that works for any hierarchy type
pub struct UnifiedMatcher;

impl UnifiedMatcher {
    /// Universal matching function that works on any tree structure
    pub fn match_hierarchies(
        source: &mut [UnifiedHierarchyNode],
        target: &mut [UnifiedHierarchyNode],
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) {
        // Match nodes at current level by name
        Self::match_nodes_by_name(source, target);

        // Recursively match children of matched nodes
        Self::match_children_recursively(source, target, field_mappings, prefix_mappings);

        // For field nodes, apply field-level matching
        Self::match_field_nodes(source, target, field_mappings, prefix_mappings);
    }

    /// Match nodes by exact name at current level
    fn match_nodes_by_name(
        source: &mut [UnifiedHierarchyNode],
        target: &mut [UnifiedHierarchyNode],
    ) {
        for source_node in source.iter_mut() {
            for target_node in target.iter_mut() {
                if source_node.name == target_node.name {
                    // Set bidirectional mapping
                    source_node.mapping_target = Some(target_node.name.clone());
                    source_node.mapping_type = MappingType::Exact;

                    target_node.mapping_target = Some(source_node.name.clone());
                    target_node.mapping_type = MappingType::Exact;
                }
            }
        }
    }

    /// Recursively match children of already matched nodes
    fn match_children_recursively(
        source: &mut [UnifiedHierarchyNode],
        target: &mut [UnifiedHierarchyNode],
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) {
        for source_node in source.iter_mut() {
            if let Some(target_name) = &source_node.mapping_target.clone() {
                // Find the matched target node
                if let Some(target_node) = target.iter_mut().find(|n| &n.name == target_name) {
                    // Recursively match their children
                    Self::match_hierarchies(
                        &mut source_node.children,
                        &mut target_node.children,
                        field_mappings,
                        prefix_mappings,
                    );
                }
            }
        }
    }

    /// Apply field-level matching for leaf nodes using existing FieldUtils logic
    fn match_field_nodes(
        source: &mut [UnifiedHierarchyNode],
        target: &mut [UnifiedHierarchyNode],
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) {
        // Extract field nodes
        let source_field_nodes: Vec<&mut UnifiedHierarchyNode> =
            source.iter_mut().filter(|n| n.is_field_node()).collect();

        let target_field_nodes: Vec<&mut UnifiedHierarchyNode> =
            target.iter_mut().filter(|n| n.is_field_node()).collect();

        // Apply field matching logic similar to FieldUtils
        for source_field in source_field_nodes {
            if source_field.mapping_target.is_none() {
                // Only match unmapped fields
                for target_field in &target_field_nodes {
                    if target_field.mapping_target.is_none() {
                        // Only match to unmapped fields
                        if let Some(mapping_type) = Self::check_field_match(
                            &source_field.name,
                            &target_field.name,
                            field_mappings,
                            prefix_mappings,
                        ) {
                            // Found a match - set bidirectional mapping
                            source_field.mapping_target = Some(target_field.name.clone());
                            source_field.mapping_type = mapping_type.clone();

                            // Note: target_field is behind a reference here, so we'll need to handle
                            // the bidirectional mapping in a second pass
                            break;
                        }
                    }
                }
            }
        }

        // Second pass to set target mappings
        for target_field in target.iter_mut().filter(|n| n.is_field_node()) {
            if target_field.mapping_target.is_none() {
                // Check if any source field maps to this target
                for source_field in source.iter().filter(|n| n.is_field_node()) {
                    if let Some(target_name) = &source_field.mapping_target
                        && target_name == &target_field.name
                    {
                        target_field.mapping_target = Some(source_field.name.clone());
                        target_field.mapping_type = source_field.mapping_type.clone();
                        break;
                    }
                }
            }
        }
    }

    /// Check if two field names match using the priority system
    fn check_field_match(
        source_name: &str,
        target_name: &str,
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) -> Option<MappingType> {
        // 1. Manual mappings (highest priority)
        if let Some(manual_target) = field_mappings.get(source_name)
            && manual_target == target_name
        {
            return Some(MappingType::Manual);
        }

        // 2. Exact match
        if source_name == target_name {
            return Some(MappingType::Exact);
        }

        // 3. Prefix mappings
        for (source_prefix, target_prefix) in prefix_mappings {
            if source_name.starts_with(source_prefix) && target_name.starts_with(target_prefix) {
                let source_base = source_name
                    .strip_prefix(source_prefix)
                    .unwrap_or(source_name);
                let target_base = target_name
                    .strip_prefix(target_prefix)
                    .unwrap_or(target_name);
                if source_base == target_base {
                    return Some(MappingType::Prefix);
                }
            }
        }

        // 4. Automatic prefix matching (cgk_ <-> nrq_, etc.)
        let source_base = Self::extract_field_base_name(source_name);
        let target_base = Self::extract_field_base_name(target_name);
        if source_base == target_base && !source_base.is_empty() {
            return Some(MappingType::Prefix);
        }

        None
    }

    /// Extract base field name by removing common prefixes
    fn extract_field_base_name(field_name: &str) -> &str {
        // Remove common prefixes like cgk_, nrq_, new_, etc.
        for prefix in &["cgk_", "nrq_", "new_", "_", "__"] {
            if let Some(base) = field_name.strip_prefix(prefix) {
                return Self::extract_field_base_name(base); // Recursive to handle multiple prefixes
            }
        }

        // Remove common suffixes like _value
        if let Some(base) = field_name.strip_suffix("_value") {
            return base;
        }

        field_name
    }
}
