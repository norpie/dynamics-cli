use crate::tui::widgets::TreeState;
use super::ActiveTab;
use super::app::State;

/// Update target tree selection to mirror source tree selection
/// Only updates if source item has a match in the target tree
pub fn update_mirrored_selection(state: &mut State, source_id: &str) {
    // Extract the key to lookup in match maps (strip prefixes for relationships/entities)
    let source_key = match state.active_tab {
        ActiveTab::Fields => source_id.to_string(),
        ActiveTab::Relationships => {
            source_id.strip_prefix("rel_").unwrap_or(source_id).to_string()
        }
        ActiveTab::Entities => {
            source_id.strip_prefix("entity_").unwrap_or(source_id).to_string()
        }
        ActiveTab::Forms | ActiveTab::Views => {
            // For hierarchical tabs, use full path as key
            source_id.to_string()
        }
    };

    // Lookup matched target node ID based on active tab
    let target_id = match state.active_tab {
        ActiveTab::Fields | ActiveTab::Forms | ActiveTab::Views => {
            // Use field_matches for Fields and hierarchical tabs
            state.field_matches.get(&source_key).map(|m| m.target_field.clone())
        }
        ActiveTab::Relationships => {
            // Use relationship_matches
            state.relationship_matches.get(&source_key).map(|m| {
                // Add back the "rel_" prefix for target tree ID
                format!("rel_{}", m.target_field)
            })
        }
        ActiveTab::Entities => {
            // Use entity_matches
            state.entity_matches.get(&source_key).map(|m| {
                // Add back the "entity_" prefix for target tree ID
                format!("entity_{}", m.target_field)
            })
        }
    };

    // Update target tree selection if match exists
    if let Some(target_id) = target_id {
        // For hierarchical tabs (Forms/Views), expand all parent containers
        if matches!(state.active_tab, ActiveTab::Forms | ActiveTab::Views) {
            expand_parent_path(state.target_tree_for_tab(), &target_id);
        }

        // Set target tree selection
        state.target_tree_for_tab().select(Some(target_id));
    }
}

/// Expand all parent containers in a path (for Forms/Views hierarchical trees)
/// Example: for path "formtype/main/form/MainForm/tab/General/fieldname"
/// Expands: "formtype/main", "formtype/main/form/MainForm", "formtype/main/form/MainForm/tab/General"
pub fn expand_parent_path(tree_state: &mut TreeState, path: &str) {
    let segments: Vec<&str> = path.split('/').collect();

    // Build each parent path and expand it
    for i in 1..segments.len() {
        let parent_path = segments[..i].join("/");
        tree_state.expand(&parent_path);
    }
}

/// Mirror container expansion/collapse from source to target tree
/// When user toggles a container in source, apply same toggle to matched container in target
pub fn mirror_container_toggle(state: &mut State, source_id: &str, is_expanded: bool) {
    // Extract the key to lookup in match maps (same logic as update_mirrored_selection)
    let source_key = match state.active_tab {
        ActiveTab::Fields => source_id.to_string(),
        ActiveTab::Relationships => {
            source_id.strip_prefix("rel_").unwrap_or(source_id).to_string()
        }
        ActiveTab::Entities => {
            source_id.strip_prefix("entity_").unwrap_or(source_id).to_string()
        }
        ActiveTab::Forms | ActiveTab::Views => {
            // For hierarchical tabs, use full path as key
            source_id.to_string()
        }
    };

    // Lookup matched target node ID based on active tab
    let target_id = match state.active_tab {
        ActiveTab::Fields | ActiveTab::Forms | ActiveTab::Views => {
            // Use field_matches for Fields and hierarchical tabs
            state.field_matches.get(&source_key).map(|m| m.target_field.clone())
        }
        ActiveTab::Relationships => {
            // Use relationship_matches
            state.relationship_matches.get(&source_key).map(|m| {
                // Add back the "rel_" prefix for target tree ID
                format!("rel_{}", m.target_field)
            })
        }
        ActiveTab::Entities => {
            // Use entity_matches
            state.entity_matches.get(&source_key).map(|m| {
                // Add back the "entity_" prefix for target tree ID
                format!("entity_{}", m.target_field)
            })
        }
    };

    // Toggle target container if match exists
    if let Some(target_id) = target_id {
        let target_tree = state.target_tree_for_tab();

        // Match the expansion state from source
        if is_expanded {
            target_tree.expand(&target_id);
        } else {
            target_tree.collapse(&target_id);
        }
    }
}
