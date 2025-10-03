use super::{ComparisonData, DataConverter};
use crate::{
    commands::migration::ui::{
        components::hierarchy_tree::TreeNode,
        screens::comparison_apps::unified_hierarchy_node::{
            NodeType, UnifiedHierarchyNode, UnifiedMatcher,
        },
    },
    dynamics::metadata::{FieldInfo, FormInfo, ViewInfo},
};

/// Unified converter that produces UnifiedHierarchyNode for any data type
/// This replaces the old specialized converters
pub struct UnifiedConverter {
    app_type: AppType,
}

#[derive(Debug, Clone)]
pub enum AppType {
    Fields,
    Views,
    Forms,
    Relationships,
}

impl UnifiedConverter {
    pub fn new_fields() -> Self {
        Self {
            app_type: AppType::Fields,
        }
    }

    pub fn new_views() -> Self {
        Self {
            app_type: AppType::Views,
        }
    }

    pub fn new_forms() -> Self {
        Self {
            app_type: AppType::Forms,
        }
    }

    pub fn new_relationships() -> Self {
        Self {
            app_type: AppType::Relationships,
        }
    }
}

impl DataConverter for UnifiedConverter {
    fn convert_data(
        &self,
        data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<TreeNode>, Vec<TreeNode>) {
        // Create unified hierarchy based on app type
        let (mut source_nodes, mut target_nodes) = match self.app_type {
            AppType::Fields => self.convert_fields_data(data, source_fields, target_fields),
            AppType::Views => self.convert_views_data(data, source_fields, target_fields),
            AppType::Forms => self.convert_forms_data(data, source_fields, target_fields),
            AppType::Relationships => {
                self.convert_relationships_data(data, source_fields, target_fields)
            }
        };

        // Apply unified matching
        UnifiedMatcher::match_hierarchies(
            &mut source_nodes,
            &mut target_nodes,
            &data.field_mappings,
            &data.prefix_mappings,
        );

        // Convert to TreeNodes for backward compatibility with current system
        // TODO: Remove this conversion once we fully migrate to UnifiedTree
        let source_tree_nodes = Self::convert_unified_to_tree_nodes(source_nodes, 0);
        let target_tree_nodes = Self::convert_unified_to_tree_nodes(target_nodes, 0);

        (source_tree_nodes, target_tree_nodes)
    }

    fn get_app_name(&self) -> &'static str {
        match self.app_type {
            AppType::Fields => "Fields",
            AppType::Views => "Views",
            AppType::Forms => "Forms",
            AppType::Relationships => "Relationships",
        }
    }
}

impl UnifiedConverter {
    /// Convert field data to unified hierarchy (flat structure)
    fn convert_fields_data(
        &self,
        data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<UnifiedHierarchyNode>, Vec<UnifiedHierarchyNode>) {
        // Filter out relationship fields from the Fields tab
        let source_nodes: Vec<UnifiedHierarchyNode> = source_fields
            .iter()
            .filter(|field| !Self::is_relationship_field(field))
            .map(|field| {
                UnifiedHierarchyNode::new_field(
                    field.name.clone(),
                    field.clone(),
                    NodeType::Field,
                    0,
                )
            })
            .collect();

        let target_nodes: Vec<UnifiedHierarchyNode> = target_fields
            .iter()
            .filter(|field| !Self::is_relationship_field(field))
            .map(|field| {
                UnifiedHierarchyNode::new_field(
                    field.name.clone(),
                    field.clone(),
                    NodeType::Field,
                    0,
                )
            })
            .collect();

        (source_nodes, target_nodes)
    }

    /// Convert view data to unified hierarchy with proper view type grouping
    fn convert_views_data(
        &self,
        data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<UnifiedHierarchyNode>, Vec<UnifiedHierarchyNode>) {
        let source_nodes = Self::build_view_type_hierarchy(&data.source_views, source_fields);
        let target_nodes = Self::build_view_type_hierarchy(&data.target_views, target_fields);

        (source_nodes, target_nodes)
    }

    /// Build view hierarchy: ViewType -> View -> ViewItems (columns)
    fn build_view_type_hierarchy(
        views: &[ViewInfo],
        fields: &[FieldInfo],
    ) -> Vec<UnifiedHierarchyNode> {
        use std::collections::HashMap;

        let mut view_types: HashMap<String, Vec<&ViewInfo>> = HashMap::new();

        // Group views by type
        for view in views {
            let view_type = if view.view_type.is_empty() {
                "Main Views".to_string()
            } else {
                format!("{} Views", view.view_type)
            };
            view_types.entry(view_type).or_default().push(view);
        }

        // Convert to UnifiedHierarchyNode hierarchy
        view_types
            .into_iter()
            .map(|(type_name, views)| {
                let mut view_type_node = UnifiedHierarchyNode::new_container(
                    type_name,
                    NodeType::ViewType,
                    "📁".to_string(),
                    0,
                );

                let view_count = views.len();

                for view in views {
                    let mut view_node = UnifiedHierarchyNode::new_container(
                        view.name.clone(),
                        NodeType::View,
                        "📄".to_string(),
                        1,
                    );

                    // Add columns as field children
                    for column in &view.columns {
                        if let Some(field_info) = fields.iter().find(|f| f.name == column.name) {
                            let field_node = UnifiedHierarchyNode::new_field(
                                column.name.clone(),
                                field_info.clone(),
                                NodeType::ViewItem,
                                2,
                            );
                            view_node.add_child(field_node);
                        }
                    }

                    view_type_node.add_child(view_node);
                }

                // Set item count for the view type
                view_type_node.item_count = view_count;
                view_type_node
            })
            .collect()
    }

    /// Convert form data to unified hierarchy with proper form type grouping
    fn convert_forms_data(
        &self,
        data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<UnifiedHierarchyNode>, Vec<UnifiedHierarchyNode>) {
        let source_nodes = Self::build_form_type_hierarchy(&data.source_forms, source_fields);
        let target_nodes = Self::build_form_type_hierarchy(&data.target_forms, target_fields);

        (source_nodes, target_nodes)
    }

    /// Build form hierarchy: FormType -> Form -> Tab -> Section -> Field
    fn build_form_type_hierarchy(
        forms: &[FormInfo],
        fields: &[FieldInfo],
    ) -> Vec<UnifiedHierarchyNode> {
        use std::collections::HashMap;

        let mut form_types: HashMap<String, Vec<&FormInfo>> = HashMap::new();

        // Group forms by type
        for form in forms {
            let form_type = format!("{} Forms", form.form_type);
            form_types.entry(form_type).or_default().push(form);
        }

        // Convert to UnifiedHierarchyNode hierarchy
        form_types
            .into_iter()
            .map(|(type_name, forms)| {
                let mut form_type_node = UnifiedHierarchyNode::new_container(
                    type_name,
                    NodeType::FormType,
                    "📁".to_string(),
                    0,
                );

                let form_count = forms.len();

                for form in forms {
                    let mut form_node = UnifiedHierarchyNode::new_container(
                        form.name.clone(),
                        NodeType::Form,
                        "📄".to_string(),
                        1,
                    );

                    // Parse form structure if available
                    if let Some(form_structure) = &form.form_structure {
                        for tab in &form_structure.tabs {
                            let mut tab_node = UnifiedHierarchyNode::new_container(
                                tab.label.clone(),
                                NodeType::Tab,
                                "📋".to_string(),
                                2,
                            );

                            for section in &tab.sections {
                                let mut section_node = UnifiedHierarchyNode::new_container(
                                    section.label.clone(),
                                    NodeType::Section,
                                    "📦".to_string(),
                                    3,
                                );

                                for field in &section.fields {
                                    // Try to resolve field metadata
                                    if let Some(field_info) =
                                        fields.iter().find(|f| f.name == field.logical_name)
                                    {
                                        let field_node = UnifiedHierarchyNode::new_field(
                                            field.logical_name.clone(),
                                            field_info.clone(),
                                            NodeType::FormField,
                                            4,
                                        );
                                        section_node.add_child(field_node);
                                    }
                                }

                                section_node.item_count = section.fields.len();
                                tab_node.add_child(section_node);
                            }

                            tab_node.item_count = tab.sections.len();
                            form_node.add_child(tab_node);
                        }

                        form_node.item_count = form_structure.tabs.len();
                    } else {
                        // If no form structure, show as placeholder
                        form_node.item_count = 0;
                    }

                    form_type_node.add_child(form_node);
                }

                // Set item count for the form type
                form_type_node.item_count = form_count;
                form_type_node
            })
            .collect()
    }

    /// Convert relationship data to unified hierarchy using proper Dynamics 365 relationship detection
    fn convert_relationships_data(
        &self,
        _data: &ComparisonData,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> (Vec<UnifiedHierarchyNode>, Vec<UnifiedHierarchyNode>) {
        // Use the same relationship field detection as the old system
        let source_nodes: Vec<UnifiedHierarchyNode> = source_fields
            .iter()
            .filter(|field| Self::is_relationship_field(field))
            .map(|field| {
                UnifiedHierarchyNode::new_field(
                    field.name.clone(),
                    field.clone(),
                    NodeType::RelationshipField,
                    0,
                )
            })
            .collect();

        let target_nodes: Vec<UnifiedHierarchyNode> = target_fields
            .iter()
            .filter(|field| Self::is_relationship_field(field))
            .map(|field| {
                UnifiedHierarchyNode::new_field(
                    field.name.clone(),
                    field.clone(),
                    NodeType::RelationshipField,
                    0,
                )
            })
            .collect();

        (source_nodes, target_nodes)
    }

    /// Check if a field is a relationship field using Dynamics 365 conventions
    fn is_relationship_field(field: &FieldInfo) -> bool {
        // Check for Dynamics 365 lookup fields (end with _value and type Edm.Guid)
        if field.name.ends_with("_value") && field.field_type == "Edm.Guid" {
            return true;
        }

        // Check for explicit relationship type strings (though rare in real data)
        if field.field_type.contains("→")
            || field.field_type.contains("N:1")
            || field.field_type.contains("1:N")
        {
            return true;
        }

        false
    }

    /// Convert UnifiedHierarchyNode to TreeNode for backward compatibility
    /// TODO: Remove this once we fully migrate to UnifiedTree
    fn convert_unified_to_tree_nodes(
        nodes: Vec<UnifiedHierarchyNode>,
        level: usize,
    ) -> Vec<TreeNode> {
        nodes
            .into_iter()
            .map(|node| {
                let children =
                    Self::convert_unified_to_tree_nodes(node.children.clone(), level + 1);

                // Create a wrapper that implements HierarchyNode
                let hierarchy_node = HierarchyNodeImpl::from_unified(node);

                TreeNode::with_children(hierarchy_node, children, level)
            })
            .collect()
    }
}

/// Helper implementation to wrap UnifiedHierarchyNode for TreeNode compatibility
/// TODO: Remove this once we fully migrate to UnifiedTree
mod tree_compatibility {
    use super::*;
    use crate::commands::migration::ui::components::hierarchy_tree::{
        FieldRenderingInfo, HierarchyNode,
    };

    #[derive(Debug)]
    pub struct HierarchyNodeImpl {
        unified_node: UnifiedHierarchyNode,
    }

    impl HierarchyNodeImpl {
        pub fn from_unified(node: UnifiedHierarchyNode) -> Self {
            Self { unified_node: node }
        }
    }

    impl HierarchyNode for HierarchyNodeImpl {
        fn display_name(&self) -> String {
            self.unified_node.display_name()
        }

        fn clean_name(&self) -> &str {
            self.unified_node.clean_name()
        }

        fn node_key(&self) -> String {
            self.unified_node.node_key()
        }

        fn is_field_node(&self) -> bool {
            self.unified_node.is_field_node()
        }

        fn get_field_info(&self) -> Option<FieldRenderingInfo> {
            self.unified_node.get_field_info()
        }

        fn is_expandable(&self) -> bool {
            self.unified_node.is_expandable()
        }

        fn mapping_target(&self) -> Option<String> {
            self.unified_node.mapping_target()
        }

        fn mapping_type(&self) -> Option<String> {
            self.unified_node.mapping_type()
        }

        fn item_count(&self) -> usize {
            self.unified_node.item_count()
        }
    }
}

// Re-export the compatibility wrapper
pub use tree_compatibility::HierarchyNodeImpl;
