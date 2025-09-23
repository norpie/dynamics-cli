use crate::{
    commands::migration::ui::screens::comparison_apps::unified_hierarchy_node::MappingType,
    dynamics::metadata::FieldInfo,
};
use std::collections::HashMap;

/// Utility functions for field matching operations
pub struct FieldUtils;

impl FieldUtils {
    /// Match fields within specific containers (Views, Forms)
    pub fn match_fields_within_containers(
        source_container_fields: &[String], // Field names in source container
        target_container_fields: &[String], // Field names in target container
        source_fields: &[FieldInfo],        // Global source field metadata
        target_fields: &[FieldInfo],        // Global target field metadata
    ) -> HashMap<String, (String, MappingType)> {
        let mut field_matches = HashMap::new();

        // Filter target fields to only those in the matching container
        let target_container_field_info: Vec<&FieldInfo> = target_fields
            .iter()
            .filter(|field| target_container_fields.contains(&field.name))
            .collect();

        // Convert to owned Vec<FieldInfo> once to avoid borrowing issues
        let target_container_field_info_owned: Vec<FieldInfo> =
            target_container_field_info.into_iter().cloned().collect();

        // Match each source field against only target fields in the same container
        for source_field_name in source_container_fields {
            if let Some(_source_field) = source_fields.iter().find(|f| f.name == *source_field_name)
                && let Some((target_name, mapping_type)) = Self::find_field_match_in_context(
                    source_field_name,
                    &target_container_field_info_owned,
                    "", // Context not needed here since we pre-filtered
                )
            {
                field_matches.insert(source_field_name.clone(), (target_name, mapping_type));
            }
        }

        field_matches
    }

    /// Find prefix-based field matches (e.g., cgk_name → nrq_name)
    pub fn find_prefix_match<'a>(
        source_field_name: &str,
        target_fields: &'a [FieldInfo],
    ) -> Option<&'a FieldInfo> {
        // Extract base name by removing common prefixes
        let source_base = Self::extract_field_base_name(source_field_name);

        // Look for target fields with same base name but different prefixes
        target_fields.iter().find(|target_field| {
            let target_base = Self::extract_field_base_name(&target_field.name);
            source_base == target_base && source_base != source_field_name
        })
    }

    /// Extract base field name by removing common prefixes
    pub fn extract_field_base_name(field_name: &str) -> &str {
        // Remove _value suffix first
        let mut name = if field_name.ends_with("_value") {
            &field_name[..field_name.len() - 6] // Remove "_value"
        } else {
            field_name
        };

        // Remove leading underscore (for _cgk_field_value patterns)
        if let Some(base) = name.strip_prefix("_") {
            name = base;
        }

        // Remove common prefixes
        if let Some(base) = name.strip_prefix("cgk_") {
            base
        } else if let Some(base) = name.strip_prefix("nrq_") {
            base
        } else if let Some(base) = name.strip_prefix("new_") {
            base
        } else {
            name
        }
    }

    /// Match fields globally using the same bidirectional logic as container-level matching
    /// For Fields app: operates on full field lists with global bidirectional matching
    pub fn match_fields_globally(
        source_field_names: &[String],
        target_field_names: &[String],
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) -> HashMap<String, (String, MappingType)> {
        let mut field_matches = HashMap::new();

        // Check each source field for matches using priority order:
        // 1. Manual mappings (highest priority)
        // 2. Exact matches
        // 3. Prefix matches (lowest priority)
        for source_field_name in source_field_names {
            if let Some(target_name) = Self::find_field_match_with_mappings(
                source_field_name,
                target_fields,
                field_mappings,
                prefix_mappings,
            ) {
                field_matches.insert(source_field_name.clone(), target_name);
            }
        }

        field_matches
    }

    /// Find a field match using manual mappings, exact matches, and prefix matches
    fn find_field_match_with_mappings(
        source_field_name: &str,
        target_fields: &[FieldInfo],
        field_mappings: &HashMap<String, String>,
        prefix_mappings: &HashMap<String, String>,
    ) -> Option<(String, MappingType)> {
        // 1. Check manual field mappings first (highest priority)
        if let Some(manual_target) = field_mappings.get(source_field_name) {
            // Verify the target field actually exists
            if target_fields.iter().any(|f| &f.name == manual_target) {
                return Some((manual_target.clone(), MappingType::Manual));
            }
        }

        // 2. Check exact match
        if let Some(exact_match) = target_fields.iter().find(|f| f.name == source_field_name) {
            return Some((exact_match.name.clone(), MappingType::Exact));
        }

        // 3. Check prefix mappings
        for (source_prefix, target_prefix) in prefix_mappings {
            if source_field_name.starts_with(source_prefix) {
                let mapped_name = source_field_name.replacen(source_prefix, target_prefix, 1);
                if let Some(prefix_match) = target_fields.iter().find(|f| f.name == mapped_name) {
                    return Some((prefix_match.name.clone(), MappingType::Prefix));
                }
            }
        }

        // 4. Check automatic prefix match (cgk_ → nrq_, etc.)
        if let Some(prefix_match) = Self::find_prefix_match(source_field_name, target_fields) {
            return Some((prefix_match.name.clone(), MappingType::Prefix));
        }

        None
    }

    /// Find a field match in the given context (container-filtered target fields)
    fn find_field_match_in_context(
        source_field_name: &str,
        target_fields: &[FieldInfo],
        _context: &str, // Context not currently used but reserved for future
    ) -> Option<(String, MappingType)> {
        // Try exact match first
        if let Some(exact_match) = target_fields.iter().find(|f| f.name == source_field_name) {
            return Some((exact_match.name.clone(), MappingType::Exact));
        }

        // Try prefix match
        if let Some(prefix_match) = Self::find_prefix_match(source_field_name, target_fields) {
            return Some((prefix_match.name.clone(), MappingType::Prefix));
        }

        None
    }
}
