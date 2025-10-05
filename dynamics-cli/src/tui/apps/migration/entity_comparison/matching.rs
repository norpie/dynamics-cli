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
        if target_lookup.contains_key(source_name) {
            matches.insert(
                source_name.clone(),
                MatchInfo {
                    target_field: source_name.clone(),
                    match_type: MatchType::Exact,
                    confidence: 1.0,
                },
            );
            continue;
        }

        // 3. Check prefix-transformed matches
        if let Some(transformed) = apply_prefix_transform(source_name, prefix_mappings) {
            if target_lookup.contains_key(&transformed) {
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: transformed,
                        match_type: MatchType::Prefix,
                        confidence: 0.9,
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
        if target_lookup.contains_key(source_name) {
            matches.insert(
                source_name.clone(),
                MatchInfo {
                    target_field: source_name.clone(),
                    match_type: MatchType::Exact,
                    confidence: 1.0,
                },
            );
            continue;
        }

        // 3. Check prefix-transformed matches
        if let Some(transformed) = apply_prefix_transform(source_name, prefix_mappings) {
            if target_lookup.contains_key(&transformed) {
                matches.insert(
                    source_name.clone(),
                    MatchInfo {
                        target_field: transformed,
                        match_type: MatchType::Prefix,
                        confidence: 0.9,
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
