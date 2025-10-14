//! Import handlers for C# mapping file import

use super::super::{State, Msg};
use crate::tui::Command;
use crate::tui::widgets::{FileBrowserEvent, FileBrowserAction};
use std::path::PathBuf;
use std::collections::HashMap;
use crossterm::event::KeyCode;

/// Open the import modal with file browser
pub fn handle_open_modal(state: &mut State) -> Command<Msg> {
    state.show_import_modal = true;

    // Set filter to show only .cs files and directories
    state.import_file_browser.set_filter(|entry| {
        entry.is_dir || entry.name.to_lowercase().ends_with(".cs")
    });

    // Refresh to apply filter
    let _ = state.import_file_browser.refresh();

    Command::set_focus(crate::tui::FocusId::new("import-file-browser"))
}

/// Close the import modal
pub fn handle_close_modal(state: &mut State) -> Command<Msg> {
    state.show_import_modal = false;
    Command::None
}

/// Handle navigation in file browser
pub fn handle_navigate(state: &mut State, key: KeyCode) -> Command<Msg> {
    match key {
        KeyCode::Up => {
            state.import_file_browser.navigate_up();
            Command::None
        }
        KeyCode::Down => {
            state.import_file_browser.navigate_down();
            Command::None
        }
        KeyCode::Enter => {
            if let Some(action) = state.import_file_browser.handle_event(FileBrowserEvent::Activate) {
                match action {
                    FileBrowserAction::FileSelected(path) => {
                        Command::perform(async move { path }, Msg::ImportFileSelected)
                    }
                    FileBrowserAction::DirectoryEntered(_) => {
                        // Just stay in the modal, directory already changed
                        Command::None
                    }
                    _ => Command::None
                }
            } else {
                Command::None
            }
        }
        KeyCode::Backspace => {
            if let Some(_action) = state.import_file_browser.handle_event(FileBrowserEvent::GoUp) {
                Command::None
            } else {
                Command::None
            }
        }
        _ => {
            state.import_file_browser.handle_navigation_key(key);
            Command::None
        }
    }
}

/// Handle file selection - read and parse the C# file
pub fn handle_file_selected(_state: &mut State, path: PathBuf) -> Command<Msg> {
    Command::perform(
        async move {
            // Read file
            let content = tokio::fs::read_to_string(&path).await
                .map_err(|e| format!("Failed to read file: {}", e))?;

            // Parse C# mappings
            let mappings = crate::cs_parser::parse_cs_field_mappings(&content)?;

            // Extract filename
            let filename = path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            log::info!("Successfully parsed {} mappings from {}", mappings.len(), filename);
            Ok((mappings, filename))
        },
        |result: Result<(HashMap<String, String>, String), String>| {
            match result {
                Ok((mappings, filename)) => Msg::ImportMappingsLoaded(mappings, filename),
                Err(err) => {
                    log::error!("Failed to parse C# mappings: {}", err);
                    // TODO: Show error modal instead of just closing
                    Msg::CloseImportModal
                }
            }
        }
    )
}

/// Handle imported mappings loaded - update state and recompute matches
pub fn handle_mappings_loaded(state: &mut State, mappings: HashMap<String, String>, filename: String) -> Command<Msg> {
    log::info!("Loading {} imported mappings from {}", mappings.len(), filename);

    state.imported_mappings = mappings;
    state.import_source_file = Some(filename.clone());
    state.show_import_modal = false;

    // Recompute all matches with imported mappings
    if let (Some(source_metadata), Some(target_metadata)) = (
        state.source_metadata.as_ref(),
        state.target_metadata.as_ref(),
    ) {
        let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
            super::super::matching::recompute_all_matches(
                source_metadata,
                target_metadata,
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

        // Rebuild all trees
        super::super::tree_builder::rebuild_all_trees(state);
    }

    // Persist to config (async, don't wait)
    let source_entity = state.source_entity.clone();
    let target_entity = state.target_entity.clone();
    let imported = state.imported_mappings.clone();
    let file = state.import_source_file.clone();

    Command::perform(
        async move {
            let config = crate::global_config();
            if let Err(e) = config.save_imported_mappings(&source_entity, &target_entity, &imported, file.as_deref()).await {
                log::error!("Failed to save imported mappings: {}", e);
            }
        },
        |_| Msg::CloseImportModal  // Dummy message, already closed
    )
}

/// Clear imported mappings
pub fn handle_clear_imported(state: &mut State) -> Command<Msg> {
    log::info!("Clearing imported mappings");

    state.imported_mappings.clear();
    state.import_source_file = None;

    // Recompute matches without imported mappings
    if let (Some(source_metadata), Some(target_metadata)) = (
        state.source_metadata.as_ref(),
        state.target_metadata.as_ref(),
    ) {
        let (field_matches, relationship_matches, entity_matches, source_entities, target_entities) =
            super::super::matching::recompute_all_matches(
                source_metadata,
                target_metadata,
                &state.field_mappings,
                &state.imported_mappings, // Now empty
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

        // Rebuild all trees
        super::super::tree_builder::rebuild_all_trees(state);
    }

    // Persist cleared state to config
    let source_entity = state.source_entity.clone();
    let target_entity = state.target_entity.clone();

    Command::perform(
        async move {
            let config = crate::global_config();
            if let Err(e) = config.save_imported_mappings(&source_entity, &target_entity, &HashMap::new(), None).await {
                log::error!("Failed to clear imported mappings in config: {}", e);
            }
        },
        |_| Msg::CloseImportModal
    )
}

/// Update viewport height for file browser scrolling
pub fn handle_set_viewport_height(state: &mut State, height: usize) -> Command<Msg> {
    let item_count = state.import_file_browser.entries().len();
    let list_state = state.import_file_browser.list_state_mut();
    list_state.set_viewport_height(height);
    list_state.update_scroll(height, item_count);
    Command::None
}
