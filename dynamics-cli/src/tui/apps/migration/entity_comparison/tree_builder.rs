//! Build tree items from entity metadata

use crate::api::EntityMetadata;
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
fn build_fields_tree(fields: &[crate::api::metadata::FieldMetadata]) -> Vec<FieldNode> {
    // TODO: Convert FieldMetadata to FieldNode tree items
    // For now, return empty - will implement when FieldNode is ready
    vec![]
}

/// Build tree items for the Relationships tab
fn build_relationships_tree(relationships: &[crate::api::metadata::RelationshipMetadata]) -> Vec<FieldNode> {
    // TODO: Convert RelationshipMetadata to tree items
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
