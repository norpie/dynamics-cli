use crate::tui::{
    element::Element,
    state::theme::Theme,
    Resource,
    widgets::TreeState,
    modals::ConfirmationModal,
    Alignment as LayerAlignment,
};
use crate::api::EntityMetadata;
use crate::{col, row, use_constraints};
use super::{Msg, ActiveTab};
use super::app::State;
use super::tree_builder::build_tree_items;
use super::tree_items::ComparisonTreeItem;
use std::collections::HashMap;
use super::models::MatchInfo;

/// Render the main side-by-side layout with source and target trees
pub fn render_main_layout(state: &mut State, theme: &Theme) -> Element<Msg> {
    use_constraints!();

    // Build tree items for the active tab from metadata
    let active_tab = state.active_tab;
    let hide_matched = state.hide_matched;
    let sort_mode = state.sort_mode;
    let mut source_items = if let Resource::Success(ref metadata) = state.source_metadata {
        build_tree_items(
            metadata,
            active_tab,
            &state.field_matches,
            &state.relationship_matches,
            &state.entity_matches,
            &state.source_entities,
            &state.examples,
            true, // is_source
            &state.source_entity,
            state.show_technical_names,
            sort_mode,
        )
    } else {
        vec![]
    };

    // Filter out matched items if hide_matched is enabled
    if hide_matched {
        source_items = filter_matched_items(source_items);
    }

    let mut target_items = if let Resource::Success(ref metadata) = state.target_metadata {
        // Create reverse matches for target side (target_field -> source_field)
        let reverse_field_matches: HashMap<String, MatchInfo> = state.field_matches.iter()
            .map(|(source_field, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_field.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        let reverse_relationship_matches: HashMap<String, MatchInfo> = state.relationship_matches.iter()
            .map(|(source_rel, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_rel.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        let reverse_entity_matches: HashMap<String, MatchInfo> = state.entity_matches.iter()
            .map(|(source_entity, match_info)| {
                (match_info.target_field.clone(), MatchInfo {
                    target_field: source_entity.clone(),  // Points back to source
                    match_type: match_info.match_type,
                    confidence: match_info.confidence,
                })
            })
            .collect();

        build_tree_items(
            metadata,
            active_tab,
            &reverse_field_matches,
            &reverse_relationship_matches,
            &reverse_entity_matches,
            &state.target_entities,
            &state.examples,
            false, // is_source
            &state.target_entity,
            state.show_technical_names,
            sort_mode,
        )
    } else {
        vec![]
    };

    // Filter out matched items if hide_matched is enabled
    if hide_matched {
        target_items = filter_matched_items(target_items);
    }

    // Apply SourceMatches sorting to target side if enabled
    if sort_mode == super::models::SortMode::SourceMatches {
        target_items = sort_target_by_source_order(&source_items, target_items);
    }

    // Cache entity names before borrowing tree states
    let source_entity_name = state.source_entity.clone();
    let target_entity_name = state.target_entity.clone();

    // Get the appropriate tree state for the active tab based on which side
    let (source_tree_state, target_tree_state) = match active_tab {
        ActiveTab::Fields => (&mut state.source_fields_tree, &mut state.target_fields_tree),
        ActiveTab::Relationships => (&mut state.source_relationships_tree, &mut state.target_relationships_tree),
        ActiveTab::Views => (&mut state.source_views_tree, &mut state.target_views_tree),
        ActiveTab::Forms => (&mut state.source_forms_tree, &mut state.target_forms_tree),
        ActiveTab::Entities => (&mut state.source_entities_tree, &mut state.target_entities_tree),
    };

    // Source panel with tree - renderer will call on_render with actual area.height
    let source_panel = Element::panel(
        Element::tree("source_tree", &source_items, source_tree_state, theme)
            .on_event(Msg::SourceTreeEvent)
            .on_select(Msg::SourceTreeNodeClicked)
            .on_render(Msg::SourceViewportHeight)
            .build()
    )
    .title(format!("Source: {}", source_entity_name))
    .build();

    // Target panel with tree - renderer will call on_render with actual area.height
    let target_panel = Element::panel(
        Element::tree("target_tree", &target_items, target_tree_state, theme)
            .on_event(Msg::TargetTreeEvent)
            .on_select(Msg::TargetTreeNodeClicked)
            .on_render(Msg::TargetViewportHeight)
            .build()
    )
    .title(format!("Target: {}", target_entity_name))
    .build();

    // Side-by-side layout
    row![
        source_panel => Fill(1),
        target_panel => Fill(1),
    ]
}

/// Render the back confirmation modal
pub fn render_back_confirmation_modal(theme: &Theme) -> Element<Msg> {
    ConfirmationModal::new("Go Back?")
        .message("Are you sure you want to go back to the comparison list?")
        .confirm_text("Yes")
        .cancel_text("No")
        .on_confirm(Msg::ConfirmBack)
        .on_cancel(Msg::CancelBack)
        .width(60)
        .height(10)
        .build()
}

pub fn render_examples_modal(state: &State, theme: &Theme) -> Element<Msg> {
    use crate::tui::modals::{ExamplesModal, ExamplePairItem};

    // Convert example pairs to list items
    let pair_items: Vec<ExamplePairItem<Msg>> = state.examples.pairs.iter().map(|pair| {
        ExamplePairItem {
            id: pair.id.clone(),
            source_id: pair.source_record_id.clone(),
            target_id: pair.target_record_id.clone(),
            label: pair.label.clone(),
            on_delete: Msg::DeleteExamplePair,
        }
    }).collect();

    ExamplesModal::new()
        .pairs(pair_items)
        .source_input_state(state.examples_source_input.clone())
        .target_input_state(state.examples_target_input.clone())
        .label_input_state(state.examples_label_input.clone())
        .list_state(state.examples_list_state.clone())
        .on_source_input_event(Msg::SourceInputEvent)
        .on_target_input_event(Msg::TargetInputEvent)
        .on_label_input_event(Msg::LabelInputEvent)
        .on_list_navigate(Msg::ExamplesListNavigate)
        .on_list_select(Msg::ExamplesListSelect)
        .on_add(Msg::AddExamplePair)
        .on_delete(Msg::DeleteExamplePair)
        .on_close(Msg::CloseExamplesModal)
        .build()
}

pub fn render_prefix_mappings_modal(state: &State, theme: &Theme) -> Element<Msg> {
    use crate::tui::modals::{PrefixMappingsModal, PrefixMappingItem};

    // Convert prefix mappings to list items
    let mapping_items: Vec<PrefixMappingItem<Msg>> = state.prefix_mappings.iter().map(|(source, target)| {
        PrefixMappingItem {
            source_prefix: source.clone(),
            target_prefix: target.clone(),
            on_delete: Msg::DeletePrefixMapping,
        }
    }).collect();

    PrefixMappingsModal::new()
        .mappings(mapping_items)
        .source_input_state(state.prefix_source_input.clone())
        .target_input_state(state.prefix_target_input.clone())
        .list_state(state.prefix_mappings_list_state.clone())
        .on_source_input_event(Msg::PrefixSourceInputEvent)
        .on_target_input_event(Msg::PrefixTargetInputEvent)
        .on_list_navigate(Msg::PrefixMappingsListNavigate)
        .on_list_select(Msg::PrefixMappingsListSelect)
        .on_add(Msg::AddPrefixMapping)
        .on_delete(Msg::DeletePrefixMapping)
        .on_close(Msg::ClosePrefixMappingsModal)
        .build()
}

/// Filter out matched items from tree based on hide_matched setting
pub fn filter_matched_items(items: Vec<super::tree_items::ComparisonTreeItem>) -> Vec<super::tree_items::ComparisonTreeItem> {
    use super::tree_items::ComparisonTreeItem;

    items.into_iter().filter_map(|item| {
        match item {
            ComparisonTreeItem::Field(ref node) => {
                // Keep if no match (unmatched field)
                if node.match_info.is_none() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Relationship(ref node) => {
                // Keep if no match (unmatched relationship)
                if node.match_info.is_none() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Entity(ref node) => {
                // Keep if no match (unmatched entity)
                if node.match_info.is_none() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Container(node) => {
                // Recursively filter container children
                let filtered_children = filter_matched_items(node.children.clone());

                // Keep container if it has any unmatched children OR if container itself is unmatched
                if !filtered_children.is_empty() || node.match_info.is_none() {
                    Some(ComparisonTreeItem::Container(super::tree_items::ContainerNode {
                        id: node.id,
                        label: node.label,
                        children: filtered_children,
                        container_match_type: node.container_match_type,
                        match_info: node.match_info,
                    }))
                } else {
                    None
                }
            }
            // Keep View and Form nodes (they don't have match info)
            ComparisonTreeItem::View(_) | ComparisonTreeItem::Form(_) => Some(item),
        }
    }).collect()
}

/// Sort target items to align with source order in SourceMatches mode
/// Matched target items appear at the same index as their source match
/// Unmatched target items are appended alphabetically at the end
fn sort_target_by_source_order(
    source_items: &[ComparisonTreeItem],
    mut target_items: Vec<ComparisonTreeItem>,
) -> Vec<ComparisonTreeItem> {
    use std::collections::HashMap;

    // Build a map of target item names to the items themselves
    let mut target_by_name: HashMap<String, ComparisonTreeItem> = target_items
        .iter()
        .map(|item| (get_item_name(item).to_string(), item.clone()))
        .collect();

    let mut result = Vec::new();
    let mut used_targets = std::collections::HashSet::new();

    // First pass: Add target items in source order (for matched items)
    for source_item in source_items {
        // Get the target field name from the source item's match
        if let Some(target_name) = get_item_match_target(source_item) {
            // Find and add the corresponding target item
            if let Some(target_item) = target_by_name.get(target_name) {
                result.push(target_item.clone());
                used_targets.insert(target_name.clone());
            }
        }
    }

    // Second pass: Add remaining unmatched target items alphabetically
    let mut unmatched: Vec<ComparisonTreeItem> = target_items
        .into_iter()
        .filter(|item| !used_targets.contains(get_item_name(item)))
        .collect();

    unmatched.sort_by(|a, b| {
        let a_name = get_item_name(a);
        let b_name = get_item_name(b);
        a_name.cmp(&b_name)
    });

    result.extend(unmatched);
    result
}

/// Get the name of an item
fn get_item_name(item: &ComparisonTreeItem) -> &str {
    match item {
        ComparisonTreeItem::Field(node) => &node.metadata.logical_name,
        ComparisonTreeItem::Relationship(node) => &node.metadata.name,
        ComparisonTreeItem::Entity(node) => &node.name,
        ComparisonTreeItem::Container(node) => &node.id,
        _ => "",
    }
}

/// Get the target field name from an item's match info
fn get_item_match_target(item: &ComparisonTreeItem) -> Option<&str> {
    match item {
        ComparisonTreeItem::Field(node) => node.match_info.as_ref().map(|m| m.target_field.as_str()),
        ComparisonTreeItem::Relationship(node) => node.match_info.as_ref().map(|m| m.target_field.as_str()),
        ComparisonTreeItem::Entity(node) => node.match_info.as_ref().map(|m| m.target_field.as_str()),
        ComparisonTreeItem::Container(node) => node.match_info.as_ref().map(|m| m.target_field.as_str()),
        _ => None,
    }
}

pub fn render_manual_mappings_modal(state: &State, theme: &Theme) -> Element<Msg> {
    use crate::tui::modals::{ManualMappingsModal, ManualMappingItem};

    // Convert manual field mappings to list items
    let mapping_items: Vec<ManualMappingItem<Msg>> = state.field_mappings.iter().map(|(source, target)| {
        ManualMappingItem {
            source_field: source.clone(),
            target_field: target.clone(),
            on_delete: Msg::DeleteManualMappingFromModal,
        }
    }).collect();

    ManualMappingsModal::new()
        .mappings(mapping_items)
        .list_state(state.manual_mappings_list_state.clone())
        .on_list_navigate(Msg::ManualMappingsListNavigate)
        .on_list_select(Msg::ManualMappingsListSelect)
        .on_delete(Msg::DeleteManualMappingFromModal)
        .on_close(Msg::CloseManualMappingsModal)
        .build()
}
