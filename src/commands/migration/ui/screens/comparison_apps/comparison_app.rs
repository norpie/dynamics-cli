use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem as RatatuiListItem},
};

use crate::{
    commands::migration::ui::{
        components::hierarchy_tree::HierarchyTree,
        screens::{
            comparison::data_models::ExamplesState,
            comparison_apps::{
                common::{AppResult, ComparisonApp as ComparisonAppTrait, FocusedSide},
                converters::{ComparisonData, DataConverter, UnifiedConverter},
            },
        },
    },
    dynamics::metadata::FieldInfo,
};

/// Information captured before a toggle action for mirroring
#[derive(Debug)]
struct MirrorToggleInfo {
    source_name: String,
    mapping_target: Option<String>,
    will_be_expanded: bool, // What the state will be AFTER the toggle
}

/// Generic comparison app that uses the strategy pattern with DataConverter
/// This replaces the old app-specific implementations with a unified approach
pub struct ComparisonApp {
    // Common state - same as HierarchicalComparisonApp
    source_tree: HierarchyTree,
    target_tree: HierarchyTree,
    source_entity: String,
    target_entity: String,
    source_fields: Vec<FieldInfo>,
    target_fields: Vec<FieldInfo>,

    // Total counts (before hide_matched filter) for accurate display
    source_total_count: usize,
    target_total_count: usize,

    // Strategy pattern - the converter handles app-specific data conversion
    converter: Box<dyn DataConverter>,
}

impl ComparisonApp {
    /// Create a new Fields comparison app
    pub fn new_fields_app(source_entity: String, target_entity: String) -> Self {
        Self::new(
            source_entity,
            target_entity,
            Box::new(UnifiedConverter::new_fields()),
        )
    }

    /// Create a new Views comparison app
    pub fn new_views_app(source_entity: String, target_entity: String) -> Self {
        Self::new(
            source_entity,
            target_entity,
            Box::new(UnifiedConverter::new_views()),
        )
    }

    /// Create a new Forms comparison app
    pub fn new_forms_app(source_entity: String, target_entity: String) -> Self {
        Self::new(
            source_entity,
            target_entity,
            Box::new(UnifiedConverter::new_forms()),
        )
    }

    /// Create a new Relationships comparison app
    pub fn new_relationships_app(source_entity: String, target_entity: String) -> Self {
        Self::new(
            source_entity,
            target_entity,
            Box::new(UnifiedConverter::new_relationships()),
        )
    }

    /// Generic constructor that accepts any DataConverter
    fn new(
        source_entity: String,
        target_entity: String,
        converter: Box<dyn DataConverter>,
    ) -> Self {
        Self {
            source_tree: HierarchyTree::new(),
            target_tree: HierarchyTree::new(),
            source_entity,
            target_entity,
            source_fields: Vec::new(),
            target_fields: Vec::new(),
            source_total_count: 0,
            target_total_count: 0,
            converter,
        }
    }

    /// Single unified update method that works for all app types
    pub fn update_comparison_data(&mut self, data: &ComparisonData) {
        // Store field data for rendering
        self.source_fields = data.source_fields.clone();
        self.target_fields = data.target_fields.clone();

        // Calculate total counts by temporarily disabling hide_matched
        let mut data_for_totals = data.clone();
        data_for_totals.hide_matched = false;
        let (source_total_nodes, target_total_nodes) =
            self.converter
                .convert_data(&data_for_totals, &self.source_fields, &self.target_fields);
        self.source_total_count = source_total_nodes.len();
        self.target_total_count = target_total_nodes.len();

        // Use the converter to handle app-specific data conversion with actual settings
        let (source_nodes, target_nodes) =
            self.converter
                .convert_data(data, &self.source_fields, &self.target_fields);

        // Set the tree data
        self.source_tree.set_nodes(source_nodes);
        self.target_tree.set_nodes(target_nodes);
    }

    /// Get the app name from the converter
    fn get_app_name(&self) -> &str {
        self.converter.get_app_name()
    }

    /// Common render method for all app types
    fn render_common(
        &mut self,
        f: &mut Frame,
        source_area: Rect,
        target_area: Rect,
        source_focused: bool,
        target_focused: bool,
        app_name: &str,
        examples_state: &ExamplesState,
    ) {
        // Render source tree
        let source_items = self.source_tree.get_flattened_items();
        let source_list_items: Vec<RatatuiListItem> = source_items
            .iter()
            .map(|(node, level)| {
                let line = HierarchyTree::render_node_line_with_field_data(
                    node,
                    *level,
                    &self.source_fields,
                    examples_state,
                    true, // is_source = true
                );
                RatatuiListItem::new(line)
            })
            .collect();

        let source_border_color = if source_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let source_selected = self
            .source_tree
            .list_state
            .selected()
            .map(|i| i + 1)
            .unwrap_or(0);
        let source_shown = source_list_items.len();

        let source_list = List::new(source_list_items)
            .block(
                Block::default()
                    .title(format!(
                        "Source {} ({}) {}:{}/{}",
                        app_name,
                        self.source_entity,
                        source_selected,
                        source_shown,
                        self.source_total_count
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(source_border_color)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(source_list, source_area, &mut self.source_tree.list_state);

        // Render target tree
        let target_items = self.target_tree.get_flattened_items();
        let target_list_items: Vec<RatatuiListItem> = target_items
            .iter()
            .map(|(node, level)| {
                let line = HierarchyTree::render_node_line_with_field_data(
                    node,
                    *level,
                    &self.target_fields,
                    examples_state,
                    false, // is_source = false
                );
                RatatuiListItem::new(line)
            })
            .collect();

        let target_border_color = if target_focused {
            Color::Cyan
        } else {
            Color::White
        };

        let target_selected = self
            .target_tree
            .list_state
            .selected()
            .map(|i| i + 1)
            .unwrap_or(0);
        let target_shown = target_list_items.len();

        let target_list = List::new(target_list_items)
            .block(
                Block::default()
                    .title(format!(
                        "Target {} ({}) {}:{}/{}",
                        app_name,
                        self.target_entity,
                        target_selected,
                        target_shown,
                        self.target_total_count
                    ))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(target_border_color)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(target_list, target_area, &mut self.target_tree.list_state);
    }

    /// Common navigation handler for all app types
    pub fn handle_navigation_common(&mut self, up: bool, focused_side: FocusedSide) {
        if up {
            match focused_side {
                FocusedSide::Source => self.source_tree.previous(),
                FocusedSide::Target => self.target_tree.previous(),
            }
        } else {
            match focused_side {
                FocusedSide::Source => self.source_tree.next(),
                FocusedSide::Target => self.target_tree.next(),
            }
        }

        // Mirror navigation to target if we're navigating on source
        self.mirror_navigation_if_source_focused(&focused_side);
    }

    /// List navigation method for backward compatibility
    pub fn handle_list_navigation(&mut self, up: bool, focused_side: FocusedSide) {
        self.handle_navigation_common(up, focused_side);
    }

    /// Tree action handler for backward compatibility
    pub fn handle_tree_action(&mut self, focused_side: FocusedSide) -> AppResult {
        // Capture source state BEFORE toggle for mirroring
        let source_mirror_info = if focused_side == FocusedSide::Source {
            self.get_source_mirror_info_before_toggle()
        } else {
            None
        };

        match focused_side {
            FocusedSide::Source => {
                self.source_tree.toggle_selected();
            }
            FocusedSide::Target => {
                self.target_tree.toggle_selected();
            }
        }

        // Mirror expand/collapse to target if we're acting on source
        if let Some(mirror_info) = source_mirror_info {
            self.apply_mirror_toggle(&mirror_info);
        }

        AppResult::Continue
    }

    /// Mirror navigation from source to target (exact mappings only)
    fn mirror_navigation_if_source_focused(&mut self, focused_side: &FocusedSide) {
        // Only mirror when focused on source
        if *focused_side != FocusedSide::Source {
            return;
        }

        // Only mirror if there's an explicit mapping (no fallback to name matching)
        if let Some(source_mapping_target) = self.source_tree.get_selected_mapping_target() {
            self.target_tree.set_selected_by_name(&source_mapping_target);
        }
        // No fallback - if no mapping exists, don't mirror
    }

    /// Capture mirror information before toggle action
    fn get_source_mirror_info_before_toggle(&mut self) -> Option<MirrorToggleInfo> {
        // Get the currently selected source node info
        let mapping_target = self.source_tree.get_selected_mapping_target();
        let source_name = self.source_tree.get_selected_node_name()?;

        // Get the current expansion state of the selected node by checking if it's expandable and expanded
        let current_expanded = self.source_tree.get_selected_node_expanded_state()?;

        Some(MirrorToggleInfo {
            source_name,
            mapping_target,
            will_be_expanded: !current_expanded, // Will be opposite after toggle
        })
    }

    /// Apply the mirror toggle to target based on pre-toggle information
    fn apply_mirror_toggle(&mut self, mirror_info: &MirrorToggleInfo) {
        // Only mirror if there's an explicit mapping (no fallback to name matching)
        if let Some(target_name) = &mirror_info.mapping_target {
            // Find and set the target node by name, then apply the expansion state
            if self.target_tree.set_selected_by_name(target_name) {
                // Now directly set the expansion state for the found target node
                self.target_tree.set_selected_node_expanded_state(mirror_info.will_be_expanded);
            }
        }
        // No fallback - if no mapping exists, don't mirror
    }
}

// Implement ComparisonApp trait for the new generic app
impl ComparisonAppTrait for ComparisonApp {
    fn render(
        &mut self,
        f: &mut Frame,
        source_area: Rect,
        target_area: Rect,
        source_focused: bool,
        target_focused: bool,
        examples_state: &ExamplesState,
    ) {
        let app_name = self.get_app_name().to_string();
        self.render_common(
            f,
            source_area,
            target_area,
            source_focused,
            target_focused,
            &app_name,
            examples_state,
        );
    }

    fn handle_mouse(
        &mut self,
        mouse: MouseEvent,
        source_area: Rect,
        target_area: Rect,
    ) -> AppResult {
        // Determine which area the mouse event occurred in
        let mouse_in_source = mouse.column >= source_area.x
            && mouse.column < source_area.x + source_area.width
            && mouse.row >= source_area.y
            && mouse.row < source_area.y + source_area.height;

        let mouse_in_target = mouse.column >= target_area.x
            && mouse.column < target_area.x + target_area.width
            && mouse.row >= target_area.y
            && mouse.row < target_area.y + target_area.height;

        // Handle the event in the appropriate tree
        if mouse_in_source {
            self.source_tree.handle_mouse_event(&mouse, source_area);
            AppResult::FocusChanged(FocusedSide::Source)
        } else if mouse_in_target {
            self.target_tree.handle_mouse_event(&mouse, target_area);
            AppResult::FocusChanged(FocusedSide::Target)
        } else {
            AppResult::Continue
        }
    }

    fn handle_keyboard(&mut self, key: KeyCode) -> AppResult {
        // Most keyboard events are handled by the parent screen
        // This could be used for app-specific shortcuts in the future
        let _ = key;
        AppResult::Continue
    }

    fn get_source_count(&self) -> usize {
        self.source_tree.get_visible_count()
    }

    fn get_target_count(&self) -> usize {
        self.target_tree.get_visible_count()
    }

    fn get_title(&self) -> String {
        format!("{} Comparison", self.get_app_name())
    }

    fn update_data(&mut self, data: &super::common::ComparisonData) {
        // Convert from common::ComparisonData to converters::ComparisonData
        let converter_data = ComparisonData {
            source_fields: data.source_fields.clone(),
            target_fields: data.target_fields.clone(),
            source_views: data.source_views.clone(),
            target_views: data.target_views.clone(),
            source_forms: data.source_forms.clone(),
            target_forms: data.target_forms.clone(),
            field_mappings: data.field_mappings.clone(),
            prefix_mappings: data.prefix_mappings.clone(),
            hide_matched: data.hide_matched,
        };
        self.update_comparison_data(&converter_data);
    }

    fn handle_list_navigation(&mut self, up: bool, focused_side: FocusedSide) {
        self.handle_navigation_common(up, focused_side);
    }

    fn handle_tree_action(&mut self, focused_side: FocusedSide) -> AppResult {
        self.handle_tree_action(focused_side)
    }

    fn apply_sorting(&mut self, sort_mode: &crate::commands::migration::ui::components::hierarchy_tree::SortMode) {
        // Apply sorting to both source and target trees
        self.source_tree.sort_nodes(sort_mode);
        self.target_tree.sort_nodes(sort_mode);
    }
}
