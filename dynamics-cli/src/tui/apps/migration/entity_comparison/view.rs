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
pub fn render_main_layout(state: &mut State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
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
            &state.ignored_items,
        )
    } else {
        vec![]
    };

    // Filter out matched items if hide_matched is enabled
    if hide_matched {
        source_items = filter_matched_items(source_items);
    }

    // Apply search filter if there's a search query
    let search_query = state.search_input.value();
    if !search_query.is_empty() {
        source_items = filter_tree_items_by_search(source_items, &search_query);
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
            &state.ignored_items,
        )
    } else {
        vec![]
    };

    // Filter out matched items if hide_matched is enabled
    if hide_matched {
        target_items = filter_matched_items(target_items);
    }

    // Apply search filter if there's a search query
    if !search_query.is_empty() {
        target_items = filter_tree_items_by_search(target_items, &search_query);
    }

    // Apply SourceMatches sorting to target side if enabled
    if sort_mode == super::models::SortMode::SourceMatches {
        target_items = sort_target_by_source_order(&source_items, target_items);
    }

    // Calculate completion percentages
    let (source_completion, target_completion) = calculate_completion_percentages(state, active_tab);

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
    .title(format!("Source: {} ({}%)", source_entity_name, source_completion))
    .build();

    // Target panel with tree - renderer will call on_render with actual area.height
    let target_panel = Element::panel(
        Element::tree("target_tree", &target_items, target_tree_state, theme)
            .on_event(Msg::TargetTreeEvent)
            .on_select(Msg::TargetTreeNodeClicked)
            .on_render(Msg::TargetViewportHeight)
            .build()
    )
    .title(format!("Target: {} ({}%)", target_entity_name, target_completion))
    .build();

    // Build search box if visible (when focused or has content)
    let search_visible = state.search_is_focused || !state.search_input.value().is_empty();

    if search_visible {
        // Count filtered results
        let source_count = source_items.len();
        let target_count = target_items.len();

        // Create search input
        let search_input = Element::text_input(
            crate::tui::FocusId::new("entity-search-input"),
            &state.search_input.value(),
            &state.search_input.state,
        )
        .placeholder("Search fields... (/ to focus, Esc to clear)")
        .on_event(Msg::SearchInputEvent)
        .build();

        // Build search panel title with result counts
        let search_title = if !search_query.is_empty() {
            format!("Search - {} matches in source, {} matches in target", source_count, target_count)
        } else {
            "Search".to_string()
        };

        // Search panel
        let search_panel = Element::panel(search_input)
            .title(search_title)
            .build();

        // Main layout with search
        col![
            search_panel => Length(3),
            row![
                source_panel => Fill(1),
                target_panel => Fill(1),
            ] => Fill(1),
        ]
    } else {
        // Side-by-side layout without search
        row![
            source_panel => Fill(1),
            target_panel => Fill(1),
        ]
    }
}

/// Render the back confirmation modal
pub fn render_back_confirmation_modal() -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
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

pub fn render_examples_modal(state: &State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
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

pub fn render_prefix_mappings_modal(state: &State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
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
/// Filter out ignored items from the tree
pub fn filter_ignored_items(
    items: Vec<super::tree_items::ComparisonTreeItem>,
    ignored_items: &std::collections::HashSet<String>,
    active_tab: ActiveTab,
    is_source: bool,
) -> Vec<super::tree_items::ComparisonTreeItem> {
    use super::tree_items::ComparisonTreeItem;

    let tab_prefix = match active_tab {
        ActiveTab::Fields => "fields",
        ActiveTab::Relationships => "relationships",
        ActiveTab::Views => "views",
        ActiveTab::Forms => "forms",
        ActiveTab::Entities => "entities",
    };
    let side_prefix = if is_source { "source" } else { "target" };

    items.into_iter().filter(|item| {
        let node_id = match item {
            ComparisonTreeItem::Field(node) => Some(&node.metadata.logical_name),
            ComparisonTreeItem::Relationship(node) => Some(&node.metadata.name),
            ComparisonTreeItem::View(node) => Some(&node.metadata.id),
            ComparisonTreeItem::Form(node) => Some(&node.metadata.id),
            ComparisonTreeItem::Entity(node) => Some(&node.name),
            _ => None,
        };

        if let Some(id) = node_id {
            let full_id = format!("{}:{}:{}", tab_prefix, side_prefix, id);
            !ignored_items.contains(&full_id)
        } else {
            true // Keep containers and other items
        }
    }).collect()
}

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

pub fn render_manual_mappings_modal(state: &State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
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

/// Render import results modal showing what was added/removed/couldn't be parsed
pub fn render_import_results_modal(state: &mut State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::element::LayoutConstraint::*;
    use crate::{col, spacer, button_row};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Stylize};
    use crate::tui::widgets::ListItem;

    let results = state.import_results.as_ref().unwrap();

    // Build list items - using String instead of Line to avoid lifetime issues
    #[derive(Clone)]
    struct ImportResultLine {
        text: String,
        style: Style,
    }

    impl ListItem for ImportResultLine {
        type Msg = Msg;

        fn to_element(&self, _is_selected: bool, _is_hovered: bool) -> Element<Self::Msg> {
            Element::styled_text(Line::from(Span::styled(self.text.clone(), self.style))).build()
        }
    }

    let mut list_items: Vec<ImportResultLine> = vec![];

    // Header line
    list_items.push(ImportResultLine {
        text: format!("Import Results: {}", results.filename),
        style: Style::default().fg(theme.accent_primary).bold(),
    });
    list_items.push(ImportResultLine {
        text: String::new(),
        style: Style::default(),
    });

    // Added mappings
    if !results.added.is_empty() {
        list_items.push(ImportResultLine {
            text: format!("✓ Added {} mappings", results.added.len()),
            style: Style::default().fg(theme.accent_success).bold(),
        });
        for (src, tgt) in &results.added {
            list_items.push(ImportResultLine {
                text: format!("  {} → {}", src, tgt),
                style: Style::default().fg(theme.text_primary),
            });
        }
        list_items.push(ImportResultLine {
            text: String::new(),
            style: Style::default(),
        });
    }

    // Updated mappings
    if !results.updated.is_empty() {
        list_items.push(ImportResultLine {
            text: format!("⟳ Updated {} mappings", results.updated.len()),
            style: Style::default().fg(theme.accent_warning).bold(),
        });
        for (src, tgt) in &results.updated {
            list_items.push(ImportResultLine {
                text: format!("  {} → {}", src, tgt),
                style: Style::default().fg(theme.text_primary),
            });
        }
        list_items.push(ImportResultLine {
            text: String::new(),
            style: Style::default(),
        });
    }

    // Removed mappings
    if !results.removed.is_empty() {
        list_items.push(ImportResultLine {
            text: format!("✗ Removed {} mappings", results.removed.len()),
            style: Style::default().fg(theme.accent_error).bold(),
        });
        for (src, tgt) in &results.removed {
            list_items.push(ImportResultLine {
                text: format!("  {} → {}", src, tgt),
                style: Style::default().fg(theme.text_secondary),
            });
        }
        list_items.push(ImportResultLine {
            text: String::new(),
            style: Style::default(),
        });
    }

    // Unparsed lines
    if !results.unparsed.is_empty() {
        list_items.push(ImportResultLine {
            text: format!("⚠ Couldn't parse {} lines", results.unparsed.len()),
            style: Style::default().fg(theme.accent_warning).bold(),
        });
        for line in &results.unparsed {
            let truncated = if line.len() > 60 {
                format!("{}...", &line[..57])
            } else {
                line.clone()
            };
            list_items.push(ImportResultLine {
                text: format!("  {}", truncated),
                style: Style::default().fg(theme.text_tertiary),
            });
        }
    }

    // If no changes at all, show a message
    if results.added.is_empty() && results.updated.is_empty() && results.removed.is_empty() && results.unparsed.is_empty() {
        list_items.push(ImportResultLine {
            text: "No changes detected".to_string(),
            style: Style::default().fg(theme.text_tertiary).italic(),
        });
    }

    // List
    log::debug!("Import results modal: {} list items", list_items.len());
    let list = Element::list("import-results-list", &list_items, &state.import_results_list, theme)
        .on_select(Msg::ImportResultsSelect)
        .on_navigate(Msg::ImportResultsNavigate)
        .on_render(Msg::ImportResultsSetViewportHeight)
        .build();

    let list_panel = Element::panel(list)
        .title("Results")
        .build();

    // Buttons
    let buttons = button_row![
        ("import-results-clear", "Clear Imports (c)", Msg::ClearImportedMappings),
        ("import-results-close", "Close (Esc)", Msg::CloseImportResultsModal),
    ];

    // Layout
    let content = col![
        list_panel => Fill(1),
        spacer!() => Length(1),
        buttons => Length(3),
    ];

    Element::panel(Element::container(content).padding(2).build())
        .title("Import Results")
        .width(90)
        .height(35)
        .build()
}

/// Render the C# mapping import modal with file browser
pub fn render_import_modal(state: &mut State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::element::LayoutConstraint::*;
    use crate::{spacer, button_row};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Stylize};

    // File browser
    let browser = Element::file_browser("import-file-browser", &state.import_file_browser, theme)
        .on_file_selected(Msg::ImportFileSelected)
        .on_navigate(Msg::ImportNavigate)
        .on_render(Msg::ImportSetViewportHeight)
        .build();

    let browser_panel = Element::panel(browser)
        .title(format!("Select C# Mapping File - {}", state.import_file_browser.current_path().display()))
        .build();

    // Help text
    let help_text = Element::styled_text(Line::from(vec![
        Span::styled("Select a .cs file to import field mappings. ", Style::default().fg(theme.text_tertiary)),
        Span::styled("Navigate with ", Style::default().fg(theme.text_tertiary)),
        Span::styled("↑/↓", Style::default().fg(theme.accent_primary).bold()),
        Span::styled(", press ", Style::default().fg(theme.text_tertiary)),
        Span::styled("Enter", Style::default().fg(theme.accent_primary).bold()),
        Span::styled(" to select.", Style::default().fg(theme.text_tertiary)),
    ])).build();

    // Info about current import
    let import_info = if let Some(ref file) = state.import_source_file {
        Element::styled_text(Line::from(vec![
            Span::styled("Currently imported: ", Style::default().fg(theme.text_secondary)),
            Span::styled(file.clone(), Style::default().fg(theme.accent_success).bold()),
            Span::styled(format!(" ({} mappings)", state.imported_mappings.len()), Style::default().fg(theme.text_tertiary)),
        ])).build()
    } else {
        Element::styled_text(Line::from(vec![
            Span::styled("No mappings currently imported", Style::default().fg(theme.text_tertiary).italic()),
        ])).build()
    };

    // Buttons
    let buttons = button_row![
        ("import-close", "Close (Esc)", Msg::CloseImportModal),
    ];

    // Layout
    let content = col![
        help_text => Length(1),
        spacer!() => Length(1),
        import_info => Length(1),
        spacer!() => Length(1),
        browser_panel => Fill(1),
        spacer!() => Length(1),
        buttons => Length(3),
    ];

    Element::panel(Element::container(content).padding(2).build())
        .title("Import C# Field Mappings")
        .width(90)
        .height(35)
        .build()
}

/// Render the ignore manager modal
pub fn render_ignore_modal(state: &mut State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::element::LayoutConstraint::*;
    use crate::{col, spacer, button_row};
    use ratatui::text::{Line, Span};
    use ratatui::style::{Style, Stylize};
    use crate::tui::widgets::ListItem;

    // Convert HashSet to Vec for consistent ordering
    let ignored_items: Vec<String> = state.ignored_items.iter().cloned().collect();

    // Build list items
    #[derive(Clone)]
    struct IgnoredItemLine {
        text: String,
        style: Style,
    }

    impl ListItem for IgnoredItemLine {
        type Msg = Msg;

        fn to_element(&self, is_selected: bool, _is_hovered: bool) -> Element<Self::Msg> {
            let style = if is_selected {
                self.style.bg(crate::global_runtime_config().theme.bg_surface)
            } else {
                self.style
            };
            Element::styled_text(Line::from(Span::styled(self.text.clone(), style))).build()
        }
    }

    let list_items: Vec<IgnoredItemLine> = ignored_items.iter()
        .map(|item| {
            // Parse item ID: "tab:side:node_id"
            let parts: Vec<&str> = item.split(':').collect();
            let display = if parts.len() == 3 {
                format!("[{}/{}] {}", parts[0], parts[1], parts[2])
            } else {
                item.clone()
            };

            IgnoredItemLine {
                text: display,
                style: Style::default().fg(theme.text_primary),
            }
        })
        .collect();

    // Help text
    let help_text = if ignored_items.is_empty() {
        Element::styled_text(Line::from(vec![
            Span::styled("No ignored items. ", Style::default().fg(theme.text_secondary)),
            Span::styled("Press ", Style::default().fg(theme.text_secondary)),
            Span::styled("Esc", Style::default().fg(theme.accent_primary).bold()),
            Span::styled(" to close.", Style::default().fg(theme.text_secondary)),
        ])).build()
    } else {
        Element::styled_text(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(theme.accent_primary).bold()),
            Span::styled(" Navigate  ", Style::default().fg(theme.text_secondary)),
            Span::styled("d", Style::default().fg(theme.accent_primary).bold()),
            Span::styled(" Delete  ", Style::default().fg(theme.text_secondary)),
            Span::styled("C", Style::default().fg(theme.accent_primary).bold()),
            Span::styled(" Clear All  ", Style::default().fg(theme.text_secondary)),
            Span::styled("Esc", Style::default().fg(theme.accent_primary).bold()),
            Span::styled(" Close", Style::default().fg(theme.text_secondary)),
        ])).build()
    };

    // List panel
    let list_panel = Element::list(
        "ignore-list",
        &list_items,
        &state.ignore_list_state,
        theme,
    )
    .on_render(|height| Msg::IgnoreSetViewportHeight(height))
    .build();

    // Buttons
    let buttons = button_row![
        ("ignore-delete", "Delete (d)", Msg::DeleteIgnoredItem),
        ("ignore-clear", "Clear All (C)", Msg::ClearAllIgnored),
        ("ignore-close", "Close (Esc)", Msg::CloseIgnoreModal),
    ];

    // Count info
    let count_info = Element::styled_text(Line::from(vec![
        Span::styled("Total ignored items: ", Style::default().fg(theme.text_secondary)),
        Span::styled(ignored_items.len().to_string(), Style::default().fg(theme.accent_primary).bold()),
    ])).build();

    // Layout
    let content = col![
        help_text => Length(1),
        spacer!() => Length(1),
        count_info => Length(1),
        spacer!() => Length(1),
        list_panel => Fill(1),
        spacer!() => Length(1),
        buttons => Length(3),
    ];

    Element::panel(Element::container(content).padding(2).build())
        .title("Ignore Manager")
        .width(80)
        .height(30)
        .build()
}

/// Filter tree items based on fuzzy search query
/// Searches both logical names and display names
fn filter_tree_items_by_search(
    items: Vec<super::tree_items::ComparisonTreeItem>,
    query: &str,
) -> Vec<super::tree_items::ComparisonTreeItem> {
    use fuzzy_matcher::skim::SkimMatcherV2;
    use fuzzy_matcher::FuzzyMatcher;
    use super::tree_items::ComparisonTreeItem;

    if query.is_empty() {
        return items;
    }

    let matcher = SkimMatcherV2::default();

    items.into_iter().filter_map(|item| {
        match &item {
            ComparisonTreeItem::Field(node) => {
                // Search both logical name and display name
                let logical_match = matcher.fuzzy_match(&node.metadata.logical_name, query);
                let display_match = matcher.fuzzy_match(&node.display_name, query);

                if logical_match.is_some() || display_match.is_some() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Relationship(node) => {
                // Search relationship name and related entity
                let name_match = matcher.fuzzy_match(&node.metadata.name, query);
                let entity_match = matcher.fuzzy_match(&node.metadata.related_entity, query);

                if name_match.is_some() || entity_match.is_some() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Entity(node) => {
                // Search entity name
                if matcher.fuzzy_match(&node.name, query).is_some() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Container(node) => {
                // Recursively filter container children
                let filtered_children = filter_tree_items_by_search(node.children.clone(), query);

                // Keep container if it has matching children OR if container label matches
                let label_match = matcher.fuzzy_match(&node.label, query);

                if !filtered_children.is_empty() || label_match.is_some() {
                    Some(ComparisonTreeItem::Container(super::tree_items::ContainerNode {
                        id: node.id.clone(),
                        label: node.label.clone(),
                        children: filtered_children,
                        container_match_type: node.container_match_type.clone(),
                        match_info: node.match_info.clone(),
                    }))
                } else {
                    None
                }
            }
            ComparisonTreeItem::View(node) => {
                // Search view name
                if matcher.fuzzy_match(&node.metadata.name, query).is_some() {
                    Some(item)
                } else {
                    None
                }
            }
            ComparisonTreeItem::Form(node) => {
                // Search form name
                if matcher.fuzzy_match(&node.metadata.name, query).is_some() {
                    Some(item)
                } else {
                    None
                }
            }
        }
    }).collect()
}

/// Calculate mapping completion percentages for source and target sides
/// Returns (source_completion_percent, target_completion_percent)
fn calculate_completion_percentages(state: &State, active_tab: ActiveTab) -> (usize, usize) {
    match active_tab {
        ActiveTab::Fields => {
            // Get total counts from metadata
            let source_total = if let Resource::Success(ref metadata) = state.source_metadata {
                metadata.fields.len()
            } else {
                0
            };
            let target_total = if let Resource::Success(ref metadata) = state.target_metadata {
                metadata.fields.len()
            } else {
                0
            };

            // Count mapped items
            let source_mapped = state.field_matches.len();

            // Count unique target fields that have been mapped to
            let target_mapped = state.field_matches
                .values()
                .map(|m| &m.target_field)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Calculate percentages
            let source_pct = if source_total > 0 {
                (source_mapped as f64 / source_total as f64 * 100.0) as usize
            } else {
                0
            };

            let target_pct = if target_total > 0 {
                (target_mapped as f64 / target_total as f64 * 100.0) as usize
            } else {
                0
            };

            (source_pct, target_pct)
        }
        ActiveTab::Relationships => {
            // Get total counts from metadata
            let source_total = if let Resource::Success(ref metadata) = state.source_metadata {
                metadata.relationships.len()
            } else {
                0
            };
            let target_total = if let Resource::Success(ref metadata) = state.target_metadata {
                metadata.relationships.len()
            } else {
                0
            };

            // Count mapped items
            let source_mapped = state.relationship_matches.len();

            // Count unique target relationships that have been mapped to
            let target_mapped = state.relationship_matches
                .values()
                .map(|m| &m.target_field)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Calculate percentages
            let source_pct = if source_total > 0 {
                (source_mapped as f64 / source_total as f64 * 100.0) as usize
            } else {
                0
            };

            let target_pct = if target_total > 0 {
                (target_mapped as f64 / target_total as f64 * 100.0) as usize
            } else {
                0
            };

            (source_pct, target_pct)
        }
        ActiveTab::Entities => {
            // Get total counts from entity lists
            let source_total = state.source_entities.len();
            let target_total = state.target_entities.len();

            // Count mapped items
            let source_mapped = state.entity_matches.len();

            // Count unique target entities that have been mapped to
            let target_mapped = state.entity_matches
                .values()
                .map(|m| &m.target_field)
                .collect::<std::collections::HashSet<_>>()
                .len();

            // Calculate percentages
            let source_pct = if source_total > 0 {
                (source_mapped as f64 / source_total as f64 * 100.0) as usize
            } else {
                0
            };

            let target_pct = if target_total > 0 {
                (target_mapped as f64 / target_total as f64 * 100.0) as usize
            } else {
                0
            };

            (source_pct, target_pct)
        }
        ActiveTab::Views | ActiveTab::Forms => {
            // Views and Forms don't have mappings/matches
            (0, 0)
        }
    }
}
