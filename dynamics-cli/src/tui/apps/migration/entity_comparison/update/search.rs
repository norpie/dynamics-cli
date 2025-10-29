use crate::tui::command::Command;
use crate::tui::widgets::TextInputEvent;
use crate::tui::FocusId;
use super::super::Msg;
use super::super::app::State;

/// Handle toggle search - show and focus the search box
pub fn handle_toggle_search(state: &mut State) -> Command<Msg> {
    state.search_is_focused = true;
    Command::SetFocus(FocusId::new("entity-search-input"))
}

/// Handle search input event
pub fn handle_search_input_event(state: &mut State, event: TextInputEvent) -> Command<Msg> {
    state.search_input.handle_event(event, None);
    Command::None
}

/// Handle clear search - clear text and hide box
pub fn handle_clear_search(state: &mut State) -> Command<Msg> {
    state.search_input.set_value(String::new());
    state.search_is_focused = false;
    Command::ClearFocus
}

/// Handle search select first match - select first filtered item
pub fn handle_search_select_first_match(state: &mut State) -> Command<Msg> {
    // We'll need to get the first filtered item
    // For now, we'll just keep the search open as per requirements
    // The actual selection logic will be handled in view.rs by passing filtered items
    // to the tree, which already handles selection

    // Since the tree already shows filtered results, we can leverage the tree's
    // built-in selection. We just need to ensure the search stays focused and
    // the search term remains in the box.

    // For now, return None to keep search open.
    // In a more complete implementation, we'd need to:
    // 1. Get the filtered tree items
    // 2. Find the first item's node ID
    // 3. Send a SourceTreeNodeClicked or TargetTreeNodeClicked message
    // 4. Keep search_is_focused = true

    Command::None
}
