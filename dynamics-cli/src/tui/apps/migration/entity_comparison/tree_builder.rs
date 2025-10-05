//! Build tree items from entity metadata

use crate::api::EntityMetadata;
use crate::api::metadata::FieldType;
use super::tree_items::{ComparisonTreeItem, FieldNode, RelationshipNode, ViewNode, FormNode, ContainerNode};
use super::ActiveTab;
use std::collections::HashMap;

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
/// Hierarchy: ViewType → View → Column (as field reference)
fn build_views_tree(views: &[crate::api::metadata::ViewMetadata]) -> Vec<ComparisonTreeItem> {
    // Group views by type
    let mut grouped: HashMap<String, Vec<&crate::api::metadata::ViewMetadata>> = HashMap::new();
    for view in views {
        grouped.entry(view.view_type.clone()).or_default().push(view);
    }

    let mut result = Vec::new();

    // Sort keys for deterministic ordering
    let mut view_types: Vec<_> = grouped.keys().cloned().collect();
    view_types.sort();

    for view_type in view_types {
        let mut type_views = grouped.get(&view_type).unwrap().clone();
        // Sort views alphabetically within each type
        type_views.sort_by(|a, b| a.name.cmp(&b.name));

        let mut view_containers = Vec::new();

        for view in type_views {
            // Create field nodes for each column
            let column_fields: Vec<ComparisonTreeItem> = view.columns.iter()
                .map(|col| {
                    // Create a placeholder FieldNode for the column
                    // TODO: Look up actual field metadata by column name
                    ComparisonTreeItem::Field(FieldNode {
                        metadata: crate::api::metadata::FieldMetadata {
                            logical_name: col.name.clone(),
                            display_name: None,
                            field_type: FieldType::Other("Column".to_string()),
                            is_required: false,
                            is_primary_key: col.is_primary,
                            max_length: None,
                            related_entity: None,
                        },
                        match_info: None,
                        example_value: None,
                    })
                })
                .collect();

            // Create container for this view
            view_containers.push(ComparisonTreeItem::Container(ContainerNode {
                id: format!("view_{}", view.id),
                label: format!("{} ({} columns)", view.name, view.columns.len()),
                children: column_fields,
            }));
        }

        // Create container for this view type
        result.push(ComparisonTreeItem::Container(ContainerNode {
            id: format!("viewtype_{}", view_type),
            label: format!("{} ({} views)", view_type, view_containers.len()),
            children: view_containers,
        }));
    }

    result
}

/// Build tree items for the Forms tab
/// Hierarchy: FormType → Form → Tab → Section → Field
fn build_forms_tree(forms: &[crate::api::metadata::FormMetadata]) -> Vec<ComparisonTreeItem> {
    // Group forms by type
    let mut grouped: HashMap<String, Vec<&crate::api::metadata::FormMetadata>> = HashMap::new();
    for form in forms {
        grouped.entry(form.form_type.clone()).or_default().push(form);
    }

    let mut result = Vec::new();

    // Sort keys for deterministic ordering
    let mut form_types: Vec<_> = grouped.keys().cloned().collect();
    form_types.sort();

    for form_type in form_types {
        let mut type_forms = grouped.get(&form_type).unwrap().clone();
        // Sort forms alphabetically within each type
        type_forms.sort_by(|a, b| a.name.cmp(&b.name));
        let mut form_containers = Vec::new();

        for form in type_forms {
            // If form has structure, build nested hierarchy
            let form_children = if let Some(structure) = &form.form_structure {
                let mut tab_containers = Vec::new();

                // Sort tabs by order
                let mut tabs = structure.tabs.clone();
                tabs.sort_by_key(|t| t.order);

                for tab in &tabs {
                    let mut section_containers = Vec::new();

                    // Sort sections by order
                    let mut sections = tab.sections.clone();
                    sections.sort_by_key(|s| s.order);

                    for section in &sections {
                        // Sort fields by row order
                        let mut fields = section.fields.clone();
                        fields.sort_by_key(|f| (f.row, f.column));

                        let field_nodes: Vec<ComparisonTreeItem> = fields.iter()
                            .map(|field| {
                                // Create FieldNode from FormField
                                ComparisonTreeItem::Field(FieldNode {
                                    metadata: crate::api::metadata::FieldMetadata {
                                        logical_name: field.logical_name.clone(),
                                        display_name: Some(field.label.clone()),
                                        field_type: FieldType::Other("FormField".to_string()),
                                        is_required: field.required_level != "None",
                                        is_primary_key: false,
                                        max_length: None,
                                        related_entity: None,
                                    },
                                    match_info: None,
                                    example_value: None,
                                })
                            })
                            .collect();

                        // Create container for section
                        section_containers.push(ComparisonTreeItem::Container(ContainerNode {
                            id: format!("section_{}_{}", form.id, section.name),
                            label: format!("{} ({} fields)", section.label, section.fields.len()),
                            children: field_nodes,
                        }));
                    }

                    // Create container for tab
                    tab_containers.push(ComparisonTreeItem::Container(ContainerNode {
                        id: format!("tab_{}_{}", form.id, tab.name),
                        label: format!("{} ({} sections)", tab.label, tab.sections.len()),
                        children: section_containers,
                    }));
                }

                tab_containers
            } else {
                // No structure available - empty form
                vec![]
            };

            // Create container for this form
            form_containers.push(ComparisonTreeItem::Container(ContainerNode {
                id: format!("form_{}", form.id),
                label: if form_children.is_empty() {
                    format!("{} (no structure)", form.name)
                } else {
                    format!("{} ({} tabs)", form.name, form_children.len())
                },
                children: form_children,
            }));
        }

        // Create container for this form type
        result.push(ComparisonTreeItem::Container(ContainerNode {
            id: format!("formtype_{}", form_type),
            label: format!("{} ({} forms)", form_type, form_containers.len()),
            children: form_containers,
        }));
    }

    result
}

/// Check if a field is a relationship field (lookup)
fn is_relationship_field(field: &crate::api::metadata::FieldMetadata) -> bool {
    matches!(field.field_type, FieldType::Lookup)
}
