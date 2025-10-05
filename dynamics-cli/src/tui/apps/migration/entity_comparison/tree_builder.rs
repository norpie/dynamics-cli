//! Build tree items from entity metadata

use crate::api::EntityMetadata;
use crate::api::metadata::FieldType;
use super::tree_items::{ComparisonTreeItem, FieldNode, RelationshipNode, ViewNode, FormNode, ContainerNode, ContainerMatchType};
use super::ActiveTab;
use super::models::MatchInfo;
use std::collections::HashMap;

/// Build tree items for the active tab from metadata with match information
pub fn build_tree_items(
    metadata: &EntityMetadata,
    active_tab: ActiveTab,
    field_matches: &HashMap<String, MatchInfo>,
    relationship_matches: &HashMap<String, MatchInfo>,
) -> Vec<ComparisonTreeItem> {
    match active_tab {
        ActiveTab::Fields => build_fields_tree(&metadata.fields, field_matches),
        ActiveTab::Relationships => build_relationships_tree(&metadata.relationships, relationship_matches),
        ActiveTab::Views => build_views_tree(&metadata.views, field_matches),
        ActiveTab::Forms => build_forms_tree(&metadata.forms, field_matches),
    }
}

/// Build tree items for the Fields tab
/// Filters out lookup fields (those are shown in Relationships tab)
fn build_fields_tree(
    fields: &[crate::api::metadata::FieldMetadata],
    field_matches: &HashMap<String, MatchInfo>,
) -> Vec<ComparisonTreeItem> {
    fields
        .iter()
        .filter(|f| !is_relationship_field(f))
        .map(|f| ComparisonTreeItem::Field(FieldNode {
            metadata: f.clone(),
            match_info: field_matches.get(&f.logical_name).cloned(),
            example_value: None,  // TODO: Add from examples state
        }))
        .collect()
}

/// Build tree items for the Relationships tab
fn build_relationships_tree(
    relationships: &[crate::api::metadata::RelationshipMetadata],
    relationship_matches: &HashMap<String, MatchInfo>,
) -> Vec<ComparisonTreeItem> {
    relationships
        .iter()
        .map(|r| ComparisonTreeItem::Relationship(RelationshipNode {
            metadata: r.clone(),
            match_info: relationship_matches.get(&r.name).cloned(),
        }))
        .collect()
}

/// Build tree items for the Views tab
/// Hierarchy: ViewType → View → Column (as field reference)
fn build_views_tree(
    views: &[crate::api::metadata::ViewMetadata],
    field_matches: &HashMap<String, MatchInfo>,
) -> Vec<ComparisonTreeItem> {
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
                        match_info: field_matches.get(&col.name).cloned(),
                        example_value: None,
                    })
                })
                .collect();

            // Create container for this view
            let container_match_type = compute_container_match_type(&column_fields);
            view_containers.push(ComparisonTreeItem::Container(ContainerNode {
                id: format!("view_{}", view.id),
                label: format!("{} ({} columns)", view.name, view.columns.len()),
                children: column_fields,
                container_match_type,
            }));
        }

        // Create container for this view type
        let container_match_type = compute_container_match_type(&view_containers);
        result.push(ComparisonTreeItem::Container(ContainerNode {
            id: format!("viewtype_{}", view_type),
            label: format!("{} ({} views)", view_type, view_containers.len()),
            children: view_containers,
            container_match_type,
        }));
    }

    result
}

/// Build tree items for the Forms tab
/// Hierarchy: FormType → Form → Tab → Section → Field
fn build_forms_tree(
    forms: &[crate::api::metadata::FormMetadata],
    field_matches: &HashMap<String, MatchInfo>,
) -> Vec<ComparisonTreeItem> {
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
                                    match_info: field_matches.get(&field.logical_name).cloned(),
                                    example_value: None,
                                })
                            })
                            .collect();

                        // Create container for section
                        let container_match_type = compute_container_match_type(&field_nodes);
                        section_containers.push(ComparisonTreeItem::Container(ContainerNode {
                            id: format!("section_{}_{}", form.id, section.name),
                            label: format!("{} ({} fields)", section.label, section.fields.len()),
                            children: field_nodes,
                            container_match_type,
                        }));
                    }

                    // Create container for tab
                    let container_match_type = compute_container_match_type(&section_containers);
                    tab_containers.push(ComparisonTreeItem::Container(ContainerNode {
                        id: format!("tab_{}_{}", form.id, tab.name),
                        label: format!("{} ({} sections)", tab.label, tab.sections.len()),
                        children: section_containers,
                        container_match_type,
                    }));
                }

                tab_containers
            } else {
                // No structure available - empty form
                vec![]
            };

            // Create container for this form
            let container_match_type = compute_container_match_type(&form_children);
            form_containers.push(ComparisonTreeItem::Container(ContainerNode {
                id: format!("form_{}", form.id),
                label: if form_children.is_empty() {
                    format!("{} (no structure)", form.name)
                } else {
                    format!("{} ({} tabs)", form.name, form_children.len())
                },
                children: form_children,
                container_match_type,
            }));
        }

        // Create container for this form type
        let container_match_type = compute_container_match_type(&form_containers);
        result.push(ComparisonTreeItem::Container(ContainerNode {
            id: format!("formtype_{}", form_type),
            label: format!("{} ({} forms)", form_type, form_containers.len()),
            children: form_containers,
            container_match_type,
        }));
    }

    result
}

/// Check if a field is a relationship field (lookup)
fn is_relationship_field(field: &crate::api::metadata::FieldMetadata) -> bool {
    matches!(field.field_type, FieldType::Lookup)
}

/// Compute ContainerMatchType based on children's match status
/// For now, this only looks at children (not container-level matching)
fn compute_container_match_type(children: &[ComparisonTreeItem]) -> ContainerMatchType {
    if children.is_empty() {
        return ContainerMatchType::NoMatch;
    }

    let mut has_matched = false;
    let mut has_unmatched = false;

    for child in children {
        let child_has_match = match child {
            ComparisonTreeItem::Field(node) => node.match_info.is_some(),
            ComparisonTreeItem::Relationship(node) => node.match_info.is_some(),
            ComparisonTreeItem::Container(node) => {
                // Recursively check container status
                node.container_match_type != ContainerMatchType::NoMatch
            }
            _ => false,
        };

        if child_has_match {
            has_matched = true;
        } else {
            has_unmatched = true;
        }

        // Early exit if we know it's mixed
        if has_matched && has_unmatched {
            return ContainerMatchType::Mixed;
        }
    }

    if has_matched && !has_unmatched {
        ContainerMatchType::FullMatch
    } else if has_matched {
        ContainerMatchType::Mixed
    } else {
        ContainerMatchType::NoMatch
    }
}
