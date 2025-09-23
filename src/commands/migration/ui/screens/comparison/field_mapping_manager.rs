use super::data_models::{ComparisonData, Match, SharedState};
use crate::{
    commands::migration::ui::components::{
        FieldMapping, ManualMappingAction, MappingSource, MatchState, PrefixMappingAction,
    },
    config::Config,
    dynamics::metadata::FieldInfo,
};

/// Manages field mappings including exact matches, prefix mappings, and manual mappings
pub struct FieldMappingManager;

impl FieldMappingManager {
    /// Compute field matches using exact matching, prefix mappings, and manual mappings
    pub fn compute_field_matches(
        shared_state: &SharedState,
        source_fields: &[FieldInfo],
        target_fields: &[FieldInfo],
    ) -> Vec<Match<FieldInfo>> {
        log::debug!(
            "Computing field matches for {} source fields and {} target fields",
            source_fields.len(),
            target_fields.len()
        );
        log::debug!(
            "Available prefix mappings: {:?}",
            shared_state.prefix_mappings
        );

        let mut matches = Vec::new();

        for source_field in source_fields {
            // 1. Check exact match first
            if let Some(target_field) = target_fields.iter().find(|f| f.name == source_field.name) {
                matches.push(Match {
                    source: source_field.clone(),
                    target: Some(target_field.clone()),
                    match_score: 1.0,
                    is_manual: false,
                });
                continue;
            }

            // 2. Check prefix mappings exact match
            let mapped_name = Self::apply_prefix_mappings(shared_state, &source_field.name);
            if mapped_name != source_field.name
                && let Some(target_field) = target_fields.iter().find(|f| f.name == mapped_name)
            {
                matches.push(Match {
                    source: source_field.clone(),
                    target: Some(target_field.clone()),
                    match_score: 1.0,
                    is_manual: false,
                });
                continue;
            }

            // 3. Check manual mappings
            if let Some(manual_target) = shared_state.field_mappings.get(&source_field.name)
                && let Some(target_field) = target_fields.iter().find(|f| &f.name == manual_target)
            {
                matches.push(Match {
                    source: source_field.clone(),
                    target: Some(target_field.clone()),
                    match_score: 1.0,
                    is_manual: true,
                });
                continue;
            }

            // No match found
            matches.push(Match {
                source: source_field.clone(),
                target: None,
                match_score: 0.0,
                is_manual: false,
            });
        }

        matches
    }

    /// Apply prefix mappings to a field name
    pub fn apply_prefix_mappings(shared_state: &SharedState, field_name: &str) -> String {
        // Debug: Log prefix mappings
        if !shared_state.prefix_mappings.is_empty() {
            log::debug!("Prefix mappings: {:?}", shared_state.prefix_mappings);
            log::debug!("Checking field '{}' against prefixes", field_name);
        }

        for (source_prefix, target_prefix) in &shared_state.prefix_mappings {
            log::debug!(
                "Checking if '{}' starts with '{}'",
                field_name,
                source_prefix
            );
            if field_name.starts_with(source_prefix) {
                let mapped_name = field_name.replacen(source_prefix, target_prefix, 1);
                log::debug!("Mapped '{}' -> '{}'", field_name, mapped_name);
                return mapped_name;
            }
        }
        field_name.to_string()
    }

    /// Get the mapping information for a source field
    pub fn get_source_field_mapping(
        shared_state: &SharedState,
        comparison_data: &ComparisonData,
        field: &FieldInfo,
    ) -> Option<FieldMapping> {
        // Check for manual field mappings first
        if let Some(mapped_name) = shared_state.field_mappings.get(&field.name) {
            return Some(FieldMapping {
                mapped_field_name: mapped_name.clone(),
                mapping_source: MappingSource::Manual,
                match_state: MatchState::FullMatch,
            });
        }

        // Check for prefix mappings
        let mapped_name = Self::apply_prefix_mappings(shared_state, &field.name);
        if mapped_name != field.name {
            return Some(FieldMapping {
                mapped_field_name: mapped_name,
                mapping_source: MappingSource::Prefix,
                match_state: MatchState::FullMatch,
            });
        }

        // Check for exact matches (auto-detected)
        if comparison_data
            .target_fields
            .iter()
            .any(|target| target.name == field.name)
        {
            return Some(FieldMapping {
                mapped_field_name: field.name.clone(),
                mapping_source: MappingSource::Exact,
                match_state: MatchState::FullMatch,
            });
        }

        None
    }

    /// Get the mapping information for a target field (which source fields map to it)
    pub fn get_target_field_mapping(
        shared_state: &SharedState,
        comparison_data: &ComparisonData,
        field: &FieldInfo,
    ) -> Option<FieldMapping> {
        // Check for manual field mappings first (reverse lookup)
        for (source_field, target_field) in &shared_state.field_mappings {
            if target_field == &field.name {
                return Some(FieldMapping {
                    mapped_field_name: source_field.clone(),
                    mapping_source: MappingSource::Manual,
                    match_state: MatchState::FullMatch,
                });
            }
        }

        // Check for prefix mappings (reverse lookup)
        for source_field in &comparison_data.source_fields {
            let mapped_name = Self::apply_prefix_mappings(shared_state, &source_field.name);
            if mapped_name == field.name && mapped_name != source_field.name {
                return Some(FieldMapping {
                    mapped_field_name: source_field.name.clone(),
                    mapping_source: MappingSource::Prefix,
                    match_state: MatchState::FullMatch,
                });
            }
        }

        // Check for exact matches (reverse lookup)
        if comparison_data
            .source_fields
            .iter()
            .any(|source| source.name == field.name)
        {
            return Some(FieldMapping {
                mapped_field_name: field.name.clone(),
                mapping_source: MappingSource::Exact,
                match_state: MatchState::FullMatch,
            });
        }

        None
    }

    /// Handle prefix mapping actions (add/delete/close)
    pub fn handle_prefix_action(
        config: &mut Config,
        shared_state: &mut SharedState,
        comparison_data: &ComparisonData,
        action: PrefixMappingAction,
    ) -> Result<(), String> {
        match action {
            PrefixMappingAction::Add {
                source_prefix,
                target_prefix,
            } => {
                // Add the prefix mapping to shared state
                shared_state
                    .prefix_mappings
                    .insert(source_prefix.clone(), target_prefix.clone());

                // Update the config with the new prefix mapping
                let source_entity = &comparison_data.source_entity;
                let target_entity = &comparison_data.target_entity;

                if let Err(e) = config.add_prefix_mapping(
                    source_entity,
                    target_entity,
                    &source_prefix,
                    &target_prefix,
                ) {
                    return Err(format!("Failed to save prefix mapping: {}", e));
                }
            }
            PrefixMappingAction::Delete(source_prefix) => {
                // Remove from shared state
                shared_state.prefix_mappings.remove(&source_prefix);

                // Update the config
                let source_entity = &comparison_data.source_entity;
                let target_entity = &comparison_data.target_entity;

                if let Err(e) =
                    config.remove_prefix_mapping(source_entity, target_entity, &source_prefix)
                {
                    return Err(format!("Failed to remove prefix mapping: {}", e));
                }
            }
            PrefixMappingAction::Close => {
                // Modal already closed, nothing to do
            }
        }
        Ok(())
    }

    /// Recompute field matches after changes to mappings
    pub fn recompute_field_matches(
        shared_state: &SharedState,
        comparison_data: &mut ComparisonData,
    ) {
        let source_fields = comparison_data.source_fields.clone();
        let target_fields = comparison_data.target_fields.clone();
        let field_matches =
            Self::compute_field_matches(shared_state, &source_fields, &target_fields);

        comparison_data.field_matches = field_matches;
    }

    /// Check if a field has any kind of match
    pub fn has_field_match(shared_state: &SharedState, field_name: &str) -> bool {
        // Check manual mappings
        if shared_state.field_mappings.contains_key(field_name) {
            return true;
        }

        // Check if prefix mapping applies
        let mapped_name = Self::apply_prefix_mappings(shared_state, field_name);
        if mapped_name != field_name {
            return true;
        }

        // For exact matches, we'd need the target fields list which isn't available here
        // This method might need to be refactored to accept target fields if needed
        false
    }

    /// Handle manual mapping actions (delete)
    pub fn handle_manual_action(
        config: &mut Config,
        shared_state: &mut SharedState,
        comparison_data: &ComparisonData,
        action: ManualMappingAction,
    ) -> Result<(), String> {
        match action {
            ManualMappingAction::Delete(source_field) => {
                // Remove from shared state
                shared_state.field_mappings.remove(&source_field);

                // Update the config
                let source_entity = &comparison_data.source_entity;
                let target_entity = &comparison_data.target_entity;

                if let Err(e) =
                    config.remove_field_mapping(source_entity, target_entity, &source_field)
                {
                    return Err(format!("Failed to remove field mapping: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Add a manual field mapping
    pub fn add_manual_mapping(
        config: &mut Config,
        shared_state: &mut SharedState,
        comparison_data: &ComparisonData,
        source_field: &str,
        target_field: &str,
    ) -> Result<(), String> {
        // Add to shared state
        shared_state
            .field_mappings
            .insert(source_field.to_string(), target_field.to_string());

        // Update the config
        let source_entity = &comparison_data.source_entity;
        let target_entity = &comparison_data.target_entity;

        if let Err(e) =
            config.add_field_mapping(source_entity, target_entity, source_field, target_field)
        {
            return Err(format!("Failed to save field mapping: {}", e));
        }

        Ok(())
    }
}
