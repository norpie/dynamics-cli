use crate::tui::command::Command;
use crate::tui::widgets::TextInputEvent;
use crate::tui::FocusId;
use super::super::Msg;
use super::super::app::State;

/// Handle toggle search - focus the search box
pub fn handle_toggle_search(state: &mut State) -> Command<Msg> {
    // Clear multi-selection when starting a search
    // to avoid confusion with filtered items
    clear_all_multi_selections(state);

    Command::SetFocus(FocusId::new("entity-search-input"))
}

/// Handle search input blur - called when search input loses focus
pub fn handle_search_input_blur(state: &mut State) -> Command<Msg> {
    log::debug!("Search input blurred");
    Command::None
}

/// Handle search input event
pub fn handle_search_input_event(state: &mut State, event: TextInputEvent) -> Command<Msg> {
    let old_value = state.search_input.value().to_string();
    state.search_input.handle_event(event, None);
    let new_value = state.search_input.value();

    // Clear multi-selection AND invalidate tree cache when search text changes
    // This prevents selecting items that are filtered out
    if old_value != new_value {
        clear_all_multi_selections(state);

        // Invalidate tree caches so they rebuild with new filtered items
        state.source_fields_tree.invalidate_cache();
        state.target_fields_tree.invalidate_cache();
        state.source_relationships_tree.invalidate_cache();
        state.target_relationships_tree.invalidate_cache();
        state.source_views_tree.invalidate_cache();
        state.target_views_tree.invalidate_cache();
        state.source_forms_tree.invalidate_cache();
        state.target_forms_tree.invalidate_cache();
        state.source_entities_tree.invalidate_cache();
        state.target_entities_tree.invalidate_cache();
    }

    Command::None
}

/// Handle clear search - clear text
pub fn handle_clear_search(state: &mut State) -> Command<Msg> {
    state.search_input.set_value(String::new());

    // Clear multi-selection when clearing search
    clear_all_multi_selections(state);

    // Invalidate tree caches so they rebuild without filtering
    state.source_fields_tree.invalidate_cache();
    state.target_fields_tree.invalidate_cache();
    state.source_relationships_tree.invalidate_cache();
    state.target_relationships_tree.invalidate_cache();
    state.source_views_tree.invalidate_cache();
    state.target_views_tree.invalidate_cache();
    state.source_forms_tree.invalidate_cache();
    state.target_forms_tree.invalidate_cache();
    state.source_entities_tree.invalidate_cache();
    state.target_entities_tree.invalidate_cache();

    Command::ClearFocus
}

/// Helper to clear multi-selections from all tree states
fn clear_all_multi_selections(state: &mut State) {
    state.source_fields_tree.clear_multi_selection();
    state.target_fields_tree.clear_multi_selection();
    state.source_relationships_tree.clear_multi_selection();
    state.target_relationships_tree.clear_multi_selection();
    state.source_views_tree.clear_multi_selection();
    state.target_views_tree.clear_multi_selection();
    state.source_forms_tree.clear_multi_selection();
    state.target_forms_tree.clear_multi_selection();
    state.source_entities_tree.clear_multi_selection();
    state.target_entities_tree.clear_multi_selection();
}

/// Handle search select first match - select first filtered item
pub fn handle_search_select_first_match(state: &mut State) -> Command<Msg> {
    // We'll need to get the first filtered item
    // For now, we'll just keep the search open as per requirements
    // The actual selection logic will be handled in view.rs by passing filtered items
    // to the tree, which already handles selection

    // Since the tree already shows filtered results, we can leverage the tree's
    // built-in selection. We just need to ensure the search term remains in the box.

    // For now, return None to keep search open.
    // In a more complete implementation, we'd need to:
    // 1. Get the filtered tree items
    // 2. Find the first item's node ID
    // 3. Send a SourceTreeNodeClicked or TargetTreeNodeClicked message

    Command::None
}
