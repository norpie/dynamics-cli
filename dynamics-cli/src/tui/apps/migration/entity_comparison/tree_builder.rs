//! Build tree items from entity metadata

use crate::api::EntityMetadata;
use crate::api::metadata::FieldType;
use super::tree_items::FieldNode;
use super::ActiveTab;

/// Build tree items for the active tab from metadata
pub fn build_tree_items(metadata: &EntityMetadata, active_tab: ActiveTab) -> Vec<FieldNode> {
    match active_tab {
        ActiveTab::Fields => build_fields_tree(&metadata.fields),
        ActiveTab::Relationships => build_relationships_tree(&metadata.relationships),
        ActiveTab::Views => build_views_tree(&metadata.views),
        ActiveTab::Forms => build_forms_tree(&metadata.forms),
    }
}

/// Build tree items for the Fields tab
/// Filters out lookup fields (those are shown in Relationships tab)
fn build_fields_tree(fields: &[crate::api::metadata::FieldMetadata]) -> Vec<FieldNode> {
    fields
        .iter()
        .filter(|f| !is_relationship_field(f))
        .map(|f| FieldNode {
            metadata: f.clone(),
            match_info: None,  // TODO: Add matching logic
            example_value: None,  // TODO: Add from examples state
        })
        .collect()
}

/// Build tree items for the Relationships tab
fn build_relationships_tree(relationships: &[crate::api::metadata::RelationshipMetadata]) -> Vec<FieldNode> {
    // TODO: Convert RelationshipMetadata to tree items
    // For now, extract lookup fields from the parent metadata
    vec![]
}

/// Build tree items for the Views tab
fn build_views_tree(views: &[crate::api::metadata::ViewMetadata]) -> Vec<FieldNode> {
    // TODO: Convert ViewMetadata to tree items
    vec![]
}

/// Build tree items for the Forms tab
fn build_forms_tree(forms: &[crate::api::metadata::FormMetadata]) -> Vec<FieldNode> {
    // TODO: Convert FormMetadata to tree items
    vec![]
}

/// Check if a field is a relationship field (lookup)
fn is_relationship_field(field: &crate::api::metadata::FieldMetadata) -> bool {
    matches!(field.field_type, FieldType::Lookup)
}
