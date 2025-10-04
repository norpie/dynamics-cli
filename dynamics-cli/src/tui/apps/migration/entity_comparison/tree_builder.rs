//! Build tree items from entity metadata

use crate::api::EntityMetadata;
use crate::api::metadata::FieldType;
use super::tree_items::{ComparisonTreeItem, FieldNode, RelationshipNode, ViewNode, FormNode};
use super::ActiveTab;

/// Build tree items for the active tab from metadata
pub fn build_tree_items(metadata: &EntityMetadata, active_tab: ActiveTab) -> Vec<ComparisonTreeItem> {
    match active_tab {
        ActiveTab::Fields => build_fields_tree(&metadata.fields),
        ActiveTab::Relationships => build_relationships_tree(&metadata.relationships),
        ActiveTab::Views => build_views_tree(&metadata.views),
        ActiveTab::Forms => build_forms_tree(&metadata.forms),
    }
}

/// Build tree items for the Fields tab
/// Filters out lookup fields (those are shown in Relationships tab)
fn build_fields_tree(fields: &[crate::api::metadata::FieldMetadata]) -> Vec<ComparisonTreeItem> {
    fields
        .iter()
        .filter(|f| !is_relationship_field(f))
        .map(|f| ComparisonTreeItem::Field(FieldNode {
            metadata: f.clone(),
            match_info: None,  // TODO: Add matching logic
            example_value: None,  // TODO: Add from examples state
        }))
        .collect()
}

/// Build tree items for the Relationships tab
fn build_relationships_tree(relationships: &[crate::api::metadata::RelationshipMetadata]) -> Vec<ComparisonTreeItem> {
    relationships
        .iter()
        .map(|r| ComparisonTreeItem::Relationship(RelationshipNode {
            metadata: r.clone(),
            match_info: None,  // TODO: Add matching logic
        }))
        .collect()
}

/// Build tree items for the Views tab
fn build_views_tree(views: &[crate::api::metadata::ViewMetadata]) -> Vec<ComparisonTreeItem> {
    views
        .iter()
        .map(|v| ComparisonTreeItem::View(ViewNode {
            metadata: v.clone(),
        }))
        .collect()
}

/// Build tree items for the Forms tab
fn build_forms_tree(forms: &[crate::api::metadata::FormMetadata]) -> Vec<ComparisonTreeItem> {
    forms
        .iter()
        .map(|f| ComparisonTreeItem::Form(FormNode {
            metadata: f.clone(),
        }))
        .collect()
}

/// Check if a field is a relationship field (lookup)
fn is_relationship_field(field: &crate::api::metadata::FieldMetadata) -> bool {
    matches!(field.field_type, FieldType::Lookup)
}
