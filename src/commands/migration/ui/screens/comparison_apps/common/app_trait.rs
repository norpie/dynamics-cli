use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{Frame, layout::Rect};

use crate::{
    commands::migration::ui::screens::{
        comparison_apps::common::FocusedSide,
        comparison::data_models::ExamplesState,
    },
};

/// Result returned by app operations
#[derive(Debug, Clone)]
pub enum AppResult {
    Continue,
    FocusChanged(FocusedSide),
    Exit,
    Back,
}

/// Common interface for comparison apps (fields, views, forms)
pub trait ComparisonApp: Send {
    /// Render the app in the given source and target areas
    fn render(
        &mut self,
        f: &mut Frame,
        source_area: Rect,
        target_area: Rect,
        source_focused: bool,
        target_focused: bool,
        examples_state: &ExamplesState,
    );

    /// Handle mouse events within the app areas
    fn handle_mouse(
        &mut self,
        mouse: MouseEvent,
        source_area: Rect,
        target_area: Rect,
    ) -> AppResult;

    /// Handle keyboard navigation
    fn handle_keyboard(&mut self, key: KeyCode) -> AppResult;

    /// Get the number of source items
    fn get_source_count(&self) -> usize;

    /// Get the number of target items
    fn get_target_count(&self) -> usize;

    /// Get the display title for this app
    fn get_title(&self) -> String;

    /// Called when the app becomes active
    fn on_enter(&mut self) {}

    /// Called when the app becomes inactive
    fn on_exit(&mut self) {}

    /// Update the app's data
    fn update_data(&mut self, data: &ComparisonData);

    /// Handle list navigation (up/down)
    fn handle_list_navigation(&mut self, up: bool, focused_side: FocusedSide) {
        // Default implementation - apps can override
        let _ = (up, focused_side);
    }

    /// Handle tree action (expand/collapse)
    fn handle_tree_action(&mut self, focused_side: FocusedSide) -> AppResult {
        // Default implementation - apps can override
        let _ = focused_side;
        AppResult::Continue
    }

    /// Apply sorting to the app's data
    fn apply_sorting(&mut self, sort_mode: &crate::commands::migration::ui::components::hierarchy_tree::SortMode) {
        // Default implementation - apps can override
        let _ = sort_mode;
    }
}

/// Data shared between all comparison apps
#[derive(Debug, Clone)]
pub struct ComparisonData {
    // Core entity metadata
    pub source_fields: Vec<crate::dynamics::metadata::FieldInfo>,
    pub target_fields: Vec<crate::dynamics::metadata::FieldInfo>,
    pub source_views: Vec<crate::dynamics::metadata::ViewInfo>,
    pub target_views: Vec<crate::dynamics::metadata::ViewInfo>,
    pub source_forms: Vec<crate::dynamics::metadata::FormInfo>,
    pub target_forms: Vec<crate::dynamics::metadata::FormInfo>,

    // Entity names for titles
    pub source_entity: String,
    pub target_entity: String,

    // Field mappings from SharedState
    pub field_mappings: std::collections::HashMap<String, String>,
    pub prefix_mappings: std::collections::HashMap<String, String>,
    pub hide_matched: bool,
}
