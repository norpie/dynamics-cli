//! Matching logic for fields, relationships, and containers

use super::models::{MatchInfo, MatchType};
use crate::api::metadata::{FieldMetadata, RelationshipMetadata};
use std::collections::HashMap;

/// Compute field matches between source and target
/// Returns map of source_field_name -> MatchInfo
pub fn compute_field_matches(
    source_fields: &[FieldMetadata],
    target_fields: &[FieldMetadata],
    manual_mappings: &HashMap<String, String>,
    prefix_mappings: &HashMap<String, String>,
) -> HashMap<String, MatchInfo> {
    let mut matches = HashMap::new();

    // Build target field lookup
    let target_lookup: HashMap<String, &FieldMetadata> = target_fields
        .iter()
        .map(|f| (f.logical_name.clone(), f))
        .collect();

    for source_field in source_fields {
        let source_name = &source_field.logical_name;

        // 1. Check manual mappings first (highest priority)
        if let Some(target_name) = manual_mappings.get(source_name) {
            if target_lookup.contains_key(target_name) {
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: target_name.clone(),
                        match_type: MatchType::Manual,
                        confidence: 1.0,
                    },
                );
                continue;
            }
        }

        // 2. Check exact name match
        if let Some(target_field) = target_lookup.get(source_name) {
            let types_match = source_field.field_type == target_field.field_type;
            matches.insert(
                source_name.clone(),
                MatchInfo {
                    target_field: source_name.clone(),
                    match_type: if types_match {
                        MatchType::Exact
                    } else {
                        MatchType::TypeMismatch
                    },
                    confidence: if types_match { 1.0 } else { 0.7 },
                },
            );
            continue;
        }

        // 3. Check prefix-transformed matches
        if let Some(transformed) = apply_prefix_transform(source_name, prefix_mappings) {
            if let Some(target_field) = target_lookup.get(&transformed) {
                let types_match = source_field.field_type == target_field.field_type;
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: transformed,
                        match_type: if types_match {
                            MatchType::Prefix
                        } else {
                            MatchType::TypeMismatch
                        },
                        confidence: if types_match { 0.9 } else { 0.6 },
                    },
                );
                continue;
            }
        }

        // No match found - don't insert anything
    }

    matches
}

/// Compute relationship matches between source and target
/// Returns map of source_relationship_name -> MatchInfo
pub fn compute_relationship_matches(
    source_relationships: &[RelationshipMetadata],
    target_relationships: &[RelationshipMetadata],
    manual_mappings: &HashMap<String, String>,
    prefix_mappings: &HashMap<String, String>,
) -> HashMap<String, MatchInfo> {
    let mut matches = HashMap::new();

    // Build target relationship lookup
    let target_lookup: HashMap<String, &RelationshipMetadata> = target_relationships
        .iter()
        .map(|r| (r.name.clone(), r))
        .collect();

    for source_rel in source_relationships {
        let source_name = &source_rel.name;

        // 1. Check manual mappings first
        if let Some(target_name) = manual_mappings.get(source_name) {
            if target_lookup.contains_key(target_name) {
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: target_name.clone(),
                        match_type: MatchType::Manual,
                        confidence: 1.0,
                    },
                );
                continue;
            }
        }

        // 2. Check exact name match
        if let Some(target_rel) = target_lookup.get(source_name) {
            // Compare relationship type and related entity
            let types_match = source_rel.relationship_type == target_rel.relationship_type
                && source_rel.related_entity == target_rel.related_entity;
            matches.insert(
                source_name.clone(),
                MatchInfo {
                    target_field: source_name.clone(),
                    match_type: if types_match {
                        MatchType::Exact
                    } else {
                        MatchType::TypeMismatch
                    },
                    confidence: if types_match { 1.0 } else { 0.7 },
                },
            );
            continue;
        }

        // 3. Check prefix-transformed matches
        if let Some(transformed) = apply_prefix_transform(source_name, prefix_mappings) {
            if let Some(target_rel) = target_lookup.get(&transformed) {
                // Compare relationship type and related entity
                let types_match = source_rel.relationship_type == target_rel.relationship_type
                    && source_rel.related_entity == target_rel.related_entity;
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: transformed,
                        match_type: if types_match {
                            MatchType::Prefix
                        } else {
                            MatchType::TypeMismatch
                        },
                        confidence: if types_match { 0.9 } else { 0.6 },
                    },
                );
                continue;
            }
        }
    }

    matches
}

/// Apply prefix transformation to a name
/// Returns transformed name if any prefix mapping applies
fn apply_prefix_transform(
    name: &str,
    prefix_mappings: &HashMap<String, String>,
) -> Option<String> {
    for (source_prefix, target_prefix) in prefix_mappings {
        if let Some(suffix) = name.strip_prefix(source_prefix) {
            return Some(format!("{}{}", target_prefix, suffix));
        }
    }
    None
}

/// Match container names (Forms, Views, Tabs, Sections, etc.)
/// Returns map of source_name -> target_name for matched containers
pub fn match_container_names(
    source_names: &[String],
    target_names: &[String],
) -> HashMap<String, String> {
    let mut matches = HashMap::new();

    let target_set: HashMap<String, ()> = target_names
        .iter()
        .map(|n| (n.clone(), ()))
        .collect();

    for source_name in source_names {
        // Simple exact match for now
        // TODO: Could add fuzzy matching, prefix transforms, etc.
        if target_set.contains_key(source_name) {
            matches.insert(source_name.clone(), source_name.clone());
        }
    }

    matches
}

/// Compute hierarchical field matches for Forms/Views tabs
/// Uses full path matching: fields only match if their container paths also match
pub fn compute_hierarchical_field_matches(
    source_metadata: &crate::api::EntityMetadata,
    target_metadata: &crate::api::EntityMetadata,
    manual_mappings: &HashMap<String, String>,
    tab_type: &str, // "forms" or "views"
) -> HashMap<String, MatchInfo> {
    let mut matches = HashMap::new();

    // Build path maps for source and target
    let source_paths = build_metadata_paths(source_metadata, tab_type);
    let target_paths = build_metadata_paths(target_metadata, tab_type);

    // Create target lookup by path
    let target_lookup: HashMap<String, PathInfo> = target_paths
        .into_iter()
        .map(|p| (p.path.clone(), p))
        .collect();

    for source_path_info in source_paths {
        let source_path = &source_path_info.path;

        // 1. Check manual mappings first (highest priority)
        if let Some(target_path) = manual_mappings.get(source_path) {
            if let Some(target_info) = target_lookup.get(target_path) {
                matches.insert(
                    source_path.clone(),
                    MatchInfo {
                        target_field: target_path.clone(),
                        match_type: MatchType::Manual,
                        confidence: 1.0,
                    },
                );
                continue;
            }
        }

        // 2. Check exact path match
        if let Some(target_info) = target_lookup.get(source_path) {
            // Paths match - check if it's a field or container
            if source_path_info.is_field && target_info.is_field {
                // It's a field - compare types
                let types_match = source_path_info.field_type == target_info.field_type;
                matches.insert(
                    source_path.clone(),
                    MatchInfo {
                        target_field: source_path.clone(),
                        match_type: if types_match {
                            MatchType::Exact
                        } else {
                            MatchType::TypeMismatch
                        },
                        confidence: if types_match { 1.0 } else { 0.7 },
                    },
                );
            } else if !source_path_info.is_field && !target_info.is_field {
                // It's a container - mark as exact match
                matches.insert(
                    source_path.clone(),
                    MatchInfo {
                        target_field: source_path.clone(),
                        match_type: MatchType::Exact,
                        confidence: 1.0,
                    },
                );
            }
        }
    }

    matches
}

/// Information about a path in the metadata tree
#[derive(Clone, Debug)]
struct PathInfo {
    path: String,
    is_field: bool,
    field_type: Option<crate::api::metadata::FieldType>,
}

/// Build paths from metadata for a specific tab type
fn build_metadata_paths(metadata: &crate::api::EntityMetadata, tab_type: &str) -> Vec<PathInfo> {
    let mut paths = Vec::new();

    match tab_type {
        "forms" => {
            // Build paths for forms
            for form in &metadata.forms {
                let formtype_path = format!("formtype/{}", form.form_type);
                // Add formtype container
                paths.push(PathInfo {
                    path: formtype_path.clone(),
                    is_field: false,
                    field_type: None,
                });

                let form_path = format!("{}/form/{}", formtype_path, form.name);
                // Add form container
                paths.push(PathInfo {
                    path: form_path.clone(),
                    is_field: false,
                    field_type: None,
                });

                if let Some(structure) = &form.form_structure {
                    for tab in &structure.tabs {
                        let tab_path = format!("{}/tab/{}", form_path, tab.name);
                        // Add tab container
                        paths.push(PathInfo {
                            path: tab_path.clone(),
                            is_field: false,
                            field_type: None,
                        });

                        for section in &tab.sections {
                            let section_path = format!("{}/section/{}", tab_path, section.name);
                            // Add section container
                            paths.push(PathInfo {
                                path: section_path.clone(),
                                is_field: false,
                                field_type: None,
                            });

                            for field in &section.fields {
                                let field_path = format!("{}/{}", section_path, field.logical_name);
                                // Add field
                                paths.push(PathInfo {
                                    path: field_path,
                                    is_field: true,
                                    field_type: Some(crate::api::metadata::FieldType::Other("FormField".to_string())),
                                });
                            }
                        }
                    }
                }
            }
        }
        "views" => {
            // Build paths for views
            for view in &metadata.views {
                let viewtype_path = format!("viewtype/{}", view.view_type);
                // Add viewtype container
                paths.push(PathInfo {
                    path: viewtype_path.clone(),
                    is_field: false,
                    field_type: None,
                });

                let view_path = format!("{}/view/{}", viewtype_path, view.name);
                // Add view container
                paths.push(PathInfo {
                    path: view_path.clone(),
                    is_field: false,
                    field_type: None,
                });

                for column in &view.columns {
                    let column_path = format!("{}/{}", view_path, column.name);
                    // Add column
                    paths.push(PathInfo {
                        path: column_path,
                        is_field: true,
                        field_type: Some(crate::api::metadata::FieldType::Other("Column".to_string())),
                    });
                }
            }
        }
        _ => {}
    }

    paths
}
