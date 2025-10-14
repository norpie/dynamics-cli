use crate::tui::command::Command;
use crate::tui::Resource;
use super::super::{Msg, ActiveTab};
use super::super::app::State;
use super::super::matching::recompute_all_matches;

pub fn handle_create_manual_mapping(state: &mut State) -> Command<Msg> {
    // Get selected items from both source and target trees
    let source_id = state.source_tree_for_tab().selected().map(|s| s.to_string());
    let target_id = state.target_tree_for_tab().selected().map(|s| s.to_string());

    if let (Some(source_id), Some(target_id)) = (source_id, target_id) {
        // Handle different ID formats based on tab type
        let (source_key, target_key) = match state.active_tab {
            ActiveTab::Fields => {
                // Fields tab: IDs are simple field names
                (source_id.clone(), target_id.clone())
            }
            ActiveTab::Relationships => {
                // Relationships tab: IDs have "rel_" prefix
                let source_name = source_id.strip_prefix("rel_").unwrap_or(&source_id).to_string();
                let target_name = target_id.strip_prefix("rel_").unwrap_or(&target_id).to_string();
                (source_name, target_name)
            }
            ActiveTab::Entities => {
                // Entities tab: IDs have "entity_" prefix
                let source_name = source_id.strip_prefix("entity_").unwrap_or(&source_id).to_string();
                let target_name = target_id.strip_prefix("entity_").unwrap_or(&target_id).to_string();
                (source_name, target_name)
            }
            ActiveTab::Forms | ActiveTab::Views => {
                // Forms/Views tabs: IDs are paths, support both fields and containers
                (source_id.clone(), target_id.clone())
            }
        };

        // Add to state mappings
        state.field_mappings.insert(source_key.clone(), target_key.clone());

        // Recompute matches
        if let (Resource::Success(source), Resource::Success(target)) =
            (&state.source_metadata, &state.target_metadata)
        {
            let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                recompute_all_matches(
                    source,
                    target,
                    &state.field_mappings,
                    &state.imported_mappings,
                    &state.prefix_mappings,
                    &state.examples,
                    &state.source_entity,
                    &state.target_entity,
                );
            state.field_matches = field_matches;
            state.relationship_matches = relationship_matches;
            state.entity_matches = entity_matches;
            state.source_entities = source_entities;
            state.target_entities = target_entities;
        }

        // Save to database
        let source_entity = state.source_entity.clone();
        let target_entity = state.target_entity.clone();
        tokio::spawn(async move {
            let config = crate::global_config();
            if let Err(e) = config.set_field_mapping(&source_entity, &target_entity, &source_key, &target_key).await {
                log::error!("Failed to save field mapping: {}", e);
            }
        });
    }
    Command::None
}

pub fn handle_delete_manual_mapping(state: &mut State) -> Command<Msg> {
    // Get selected item from source tree
    let source_id = state.source_tree_for_tab().selected().map(|s| s.to_string());

    if let Some(source_id) = source_id {
        // Extract the key based on tab type (same logic as CreateManualMapping)
        let source_key = match state.active_tab {
            ActiveTab::Fields => source_id.clone(),
            ActiveTab::Relationships => {
                source_id.strip_prefix("rel_").unwrap_or(&source_id).to_string()
            }
            ActiveTab::Entities => {
                source_id.strip_prefix("entity_").unwrap_or(&source_id).to_string()
            }
            ActiveTab::Forms | ActiveTab::Views => source_id.clone(),
        };

        // Try to remove from field_mappings
        if state.field_mappings.remove(&source_key).is_some() {
            // Recompute matches
            if let (Resource::Success(source), Resource::Success(target)) =
                (&state.source_metadata, &state.target_metadata)
            {
                let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
                    recompute_all_matches(
                        source,
                        target,
                        &state.field_mappings,
                        &state.imported_mappings,
                        &state.prefix_mappings,
                        &state.examples,
                        &state.source_entity,
                        &state.target_entity,
                    );
                state.field_matches = field_matches;
                state.relationship_matches = relationship_matches;
                state.entity_matches = entity_matches;
                state.source_entities = source_entities;
                state.target_entities = target_entities;
            }

            // Delete from database
            let source_entity = state.source_entity.clone();
            let target_entity = state.target_entity.clone();
            tokio::spawn(async move {
                let config = crate::global_config();
                if let Err(e) = config.delete_field_mapping(&source_entity, &target_entity, &source_key).await {
                    log::error!("Failed to delete field mapping: {}", e);
                }
            });
        }
    }
    Command::None
}

pub fn handle_toggle_hide_matched(state: &mut State) -> Command<Msg> {
    state.hide_matched = !state.hide_matched;
    Command::None
}

pub fn handle_toggle_sort_mode(state: &mut State) -> Command<Msg> {
    state.sort_mode = state.sort_mode.toggle();
    Command::None
}

pub fn handle_toggle_technical_names(state: &mut State) -> Command<Msg> {
    state.show_technical_names = !state.show_technical_names;
    Command::None
}

pub fn handle_export_to_excel(state: &mut State) -> Command<Msg> {
    // Check if metadata is loaded
    if !matches!(state.source_metadata, Resource::Success(_)) ||
       !matches!(state.target_metadata, Resource::Success(_)) {
        log::warn!("Cannot export: metadata not fully loaded");
        return Command::None;
    }

    // Generate filename with timestamp
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!(
        "{}_{}_to_{}_{}.xlsx",
        state.migration_name,
        state.source_entity,
        state.target_entity,
        timestamp
    );

    // Get output directory from config or use current directory
    let output_path = std::path::PathBuf::from(&filename);

    // Perform export in background
    let state_clone = state.clone();
    tokio::spawn(async move {
        match super::super::export::MigrationExporter::export_and_open(&state_clone, output_path.to_str().unwrap()) {
            Ok(_) => {
                log::info!("Successfully exported to {}", filename);
            }
            Err(e) => {
                log::error!("Failed to export to Excel: {}", e);
            }
        }
    });

    Command::None
}
