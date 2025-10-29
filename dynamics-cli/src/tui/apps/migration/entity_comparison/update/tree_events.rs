use crate::tui::command::Command;
use crate::tui::widgets::TreeEvent;
use super::super::{Msg, ActiveTab};
use super::super::app::State;
use super::super::tree_sync::{update_mirrored_selection, mirror_container_toggle};

pub fn handle_source_tree_event(state: &mut State, event: TreeEvent) -> Command<Msg> {
    // Update focused side
    state.focused_side = super::super::Side::Source;

    // Handle source tree navigation/interaction
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.source_fields_tree,
        ActiveTab::Relationships => &mut state.source_relationships_tree,
        ActiveTab::Views => &mut state.source_views_tree,
        ActiveTab::Forms => &mut state.source_forms_tree,
        ActiveTab::Entities => &mut state.source_entities_tree,
    };

    // Check if this is a toggle event before handling
    let is_toggle = matches!(event, TreeEvent::Toggle);
    let node_id_before_toggle = if is_toggle {
        tree_state.selected().map(|s| s.to_string())
    } else {
        None
    };

    tree_state.handle_event(event);

    // Get selected ID before releasing the borrow
    let selected_id = tree_state.selected().map(|s| s.to_string());

    // Check if node is expanded (for toggle mirroring)
    let is_expanded = if let Some(id) = &node_id_before_toggle {
        tree_state.is_expanded(id)
    } else {
        false
    };

    // Release the borrow by dropping tree_state reference
    drop(tree_state);

    // Mirrored selection: update target tree when source selection changes
    if let Some(source_id) = selected_id {
        update_mirrored_selection(state, &source_id);
    }

    // Mirror container expansion/collapse
    if let Some(toggled_id) = node_id_before_toggle {
        mirror_container_toggle(state, &toggled_id, is_expanded);
    }

    Command::None
}

pub fn handle_target_tree_event(state: &mut State, event: TreeEvent) -> Command<Msg> {
    // Update focused side
    state.focused_side = super::super::Side::Target;

    // Handle target tree navigation/interaction
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.target_fields_tree,
        ActiveTab::Relationships => &mut state.target_relationships_tree,
        ActiveTab::Views => &mut state.target_views_tree,
        ActiveTab::Forms => &mut state.target_forms_tree,
        ActiveTab::Entities => &mut state.target_entities_tree,
    };
    tree_state.handle_event(event);
    Command::None
}

pub fn handle_source_viewport_height(state: &mut State, height: usize) -> Command<Msg> {
    // Renderer calls this with actual viewport height
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.source_fields_tree,
        ActiveTab::Relationships => &mut state.source_relationships_tree,
        ActiveTab::Views => &mut state.source_views_tree,
        ActiveTab::Forms => &mut state.source_forms_tree,
        ActiveTab::Entities => &mut state.source_entities_tree,
    };
    tree_state.set_viewport_height(height);
    Command::None
}

pub fn handle_target_viewport_height(state: &mut State, height: usize) -> Command<Msg> {
    // Renderer calls this with actual viewport height
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.target_fields_tree,
        ActiveTab::Relationships => &mut state.target_relationships_tree,
        ActiveTab::Views => &mut state.target_views_tree,
        ActiveTab::Forms => &mut state.target_forms_tree,
        ActiveTab::Entities => &mut state.target_entities_tree,
    };
    tree_state.set_viewport_height(height);
    Command::None
}

pub fn handle_source_node_clicked(state: &mut State, node_id: String) -> Command<Msg> {
    // Update focused side
    state.focused_side = super::super::Side::Source;

    // Get the tree state for the active tab
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.source_fields_tree,
        ActiveTab::Relationships => &mut state.source_relationships_tree,
        ActiveTab::Views => &mut state.source_views_tree,
        ActiveTab::Forms => &mut state.source_forms_tree,
        ActiveTab::Entities => &mut state.source_entities_tree,
    };

    // Update selection and scroll to ensure visibility
    tree_state.select_and_scroll(Some(node_id.clone()));

    // Release the borrow
    drop(tree_state);

    // Trigger mirrored selection to update target tree
    update_mirrored_selection(state, &node_id);

    Command::None
}

pub fn handle_target_node_clicked(state: &mut State, node_id: String) -> Command<Msg> {
    // Update focused side
    state.focused_side = super::super::Side::Target;

    // Get the tree state for the active tab
    let tree_state = match state.active_tab {
        ActiveTab::Fields => &mut state.target_fields_tree,
        ActiveTab::Relationships => &mut state.target_relationships_tree,
        ActiveTab::Views => &mut state.target_views_tree,
        ActiveTab::Forms => &mut state.target_forms_tree,
        ActiveTab::Entities => &mut state.target_entities_tree,
    };

    // Update selection and scroll to ensure visibility
    tree_state.select_and_scroll(Some(node_id.clone()));

    Command::None
}
