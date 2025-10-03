use super::data_models::{ActiveTab, ComparisonData, FocusedSide, SharedState};
use crate::commands::migration::ui::screens::{
    ScreenResult,
    comparison_apps::common::{
        ComparisonApp, ComparisonData as CommonComparisonData, FocusedSide as CommonFocusedSide,
    },
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// Handles all event processing for the unified comparison screen
pub struct EventHandlers;

impl EventHandlers {
    /// Handle mouse events including tab switching, scrolling, and clicking
    pub fn handle_mouse_event<FA, RA, VA, FoA>(
        mouse: MouseEvent,
        active_tab: &mut ActiveTab,
        focused_side: &mut FocusedSide,
        tab_area: Option<Rect>,
        source_area: Option<Rect>,
        target_area: Option<Rect>,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) -> ScreenResult
    where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        // Check for tab area clicks first
        if let Some(tab_area) = tab_area {
            let mouse_over_tabs = mouse.column >= tab_area.x
                && mouse.column < tab_area.x + tab_area.width
                && mouse.row >= tab_area.y
                && mouse.row < tab_area.y + tab_area.height;

            if mouse_over_tabs && let MouseEventKind::Down(MouseButton::Left) = mouse.kind {
                // Calculate which tab was clicked based on sequential layout with separators
                // Format: " [1] Fields │ [2] Relationships │ [3] Views │ [4] Forms "
                let tab_titles = Self::get_tab_titles();
                let relative_x = mouse.column - tab_area.x;

                // Start after left border and initial space
                let mut current_pos = 1u16; // Account for left border

                for (index, title) in tab_titles.iter().enumerate() {
                    let tab_length = title.len() as u16;

                    // Check if click is within this tab's text area
                    if relative_x >= current_pos && relative_x < current_pos + tab_length {
                        Self::switch_tab(active_tab, index + 1); // switch_tab expects 1-indexed
                        return ScreenResult::Continue;
                    }

                    // Move to next tab: tab_length + separator (" │ " = 3 chars)
                    current_pos += tab_length;
                    if index < tab_titles.len() - 1 {
                        current_pos += 3; // Add separator width
                    }
                }
            }
        }

        // Use cached areas from the last render
        let (source_area, target_area) = match (source_area, target_area) {
            (Some(source), Some(target)) => (source, target),
            _ => return ScreenResult::Continue, // No areas cached yet
        };

        // Determine which side the mouse is over
        let mouse_over_source = mouse.column >= source_area.x
            && mouse.column < source_area.x + source_area.width
            && mouse.row >= source_area.y
            && mouse.row < source_area.y + source_area.height;

        let mouse_over_target = mouse.column >= target_area.x
            && mouse.column < target_area.x + target_area.width
            && mouse.row >= target_area.y
            && mouse.row < target_area.y + target_area.height;

        // Update focus based on which side mouse is interacting with
        let mut focus_changed = false;
        if mouse_over_source && !matches!(*focused_side, FocusedSide::Source) {
            *focused_side = FocusedSide::Source;
            focus_changed = true;
        } else if mouse_over_target && !matches!(*focused_side, FocusedSide::Target) {
            *focused_side = FocusedSide::Target;
            focus_changed = true;
        }

        // Handle mouse interactions with the lists
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                if mouse_over_source || mouse_over_target {
                    Self::handle_mouse_scroll(
                        mouse_over_source,
                        true,
                        active_tab,
                        focused_side,
                        fields_app,
                        relationships_app,
                        views_app,
                        forms_app,
                    );
                    return ScreenResult::Continue;
                }
            }
            MouseEventKind::ScrollDown => {
                if mouse_over_source || mouse_over_target {
                    Self::handle_mouse_scroll(
                        mouse_over_source,
                        false,
                        active_tab,
                        focused_side,
                        fields_app,
                        relationships_app,
                        views_app,
                        forms_app,
                    );
                    return ScreenResult::Continue;
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if mouse_over_source || mouse_over_target {
                    Self::handle_mouse_click(
                        mouse_over_source,
                        mouse,
                        if mouse_over_source {
                            source_area
                        } else {
                            target_area
                        },
                        active_tab,
                        fields_app,
                        relationships_app,
                    );
                    return ScreenResult::Continue;
                }
            }
            _ => {}
        }

        // Return appropriate result based on focus change
        if focus_changed {
            ScreenResult::Continue
        } else {
            ScreenResult::Continue
        }
    }

    /// Handle mouse scroll events
    fn handle_mouse_scroll<FA, RA, VA, FoA>(
        is_source: bool,
        up: bool,
        active_tab: &ActiveTab,
        focused_side: &mut FocusedSide,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        // Set focus to the side being scrolled
        let target_side = if is_source {
            FocusedSide::Source
        } else {
            FocusedSide::Target
        };
        *focused_side = target_side;

        // All apps now use hierarchical navigation - delegate to appropriate app
        match *active_tab {
            ActiveTab::Fields => {
                let common_focused_side = Self::convert_focused_side(target_side);
                fields_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Relationships => {
                let common_focused_side = Self::convert_focused_side(target_side);
                relationships_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Views => {
                let common_focused_side = Self::convert_focused_side(target_side);
                views_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Forms => {
                let common_focused_side = Self::convert_focused_side(target_side);
                forms_app.handle_list_navigation(up, common_focused_side);
            }
        }
    }

    /// Handle mouse click events
    fn handle_mouse_click<FA, RA>(
        is_source: bool,
        mouse: MouseEvent,
        area: Rect,
        active_tab: &ActiveTab,
        fields_app: &mut FA,
        relationships_app: &mut RA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
    {
        match *active_tab {
            ActiveTab::Fields => {
                // Delegate mouse events to fields app
                let source_area = if is_source { area } else { Rect::default() };
                let target_area = if !is_source { area } else { Rect::default() };
                let _ = fields_app.handle_mouse(mouse, source_area, target_area);
            }
            ActiveTab::Relationships => {
                // Delegate mouse events to relationships app
                let source_area = if is_source { area } else { Rect::default() };
                let target_area = if !is_source { area } else { Rect::default() };
                let _ = relationships_app.handle_mouse(mouse, source_area, target_area);
            }
            // Views and Forms would need additional state management for mouse clicks
            _ => {
                // Legacy click handling for Views and Forms could be added here if needed
            }
        }
    }

    /// Handle action keys (Enter/Space)
    pub fn handle_action_key<FA, RA, VA, FoA>(
        active_tab: &ActiveTab,
        focused_side: &FocusedSide,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        match *active_tab {
            ActiveTab::Fields => {
                // Handle expand/collapse for fields tree (now with mirroring!)
                let common_focused_side = Self::convert_focused_side(*focused_side);
                fields_app.handle_tree_action(common_focused_side);
            }
            ActiveTab::Relationships => {
                // Handle expand/collapse for relationships tree
                let common_focused_side = Self::convert_focused_side(*focused_side);
                relationships_app.handle_tree_action(common_focused_side);
            }
            ActiveTab::Views => {
                // Handle expand/collapse for views tree
                let common_focused_side = Self::convert_focused_side(*focused_side);
                views_app.handle_tree_action(common_focused_side);
            }
            ActiveTab::Forms => {
                // Handle expand/collapse for forms tree
                let common_focused_side = Self::convert_focused_side(*focused_side);
                forms_app.handle_tree_action(common_focused_side);
            }
        }
    }

    /// Switch focused side
    pub fn switch_side(focused_side: &mut FocusedSide) {
        *focused_side = match *focused_side {
            FocusedSide::Source => FocusedSide::Target,
            FocusedSide::Target => FocusedSide::Source,
        };
    }

    /// Toggle hide matched setting and refresh apps
    pub fn toggle_hide_matched<FA, RA, VA, FoA>(
        shared_state: &mut SharedState,
        comparison_data: &Option<ComparisonData>,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        // Toggle hide_matched setting and refresh all apps with updated data
        shared_state.hide_matched = !shared_state.hide_matched;
        log::debug!("Toggle hide_matched: {}", shared_state.hide_matched);

        // Refresh all apps with updated data
        Self::refresh_all_apps(
            shared_state,
            comparison_data,
            fields_app,
            relationships_app,
            views_app,
            forms_app,
        );
    }

    /// Refresh all apps with current data
    pub fn refresh_all_apps<FA, RA, VA, FoA>(
        shared_state: &SharedState,
        comparison_data: &Option<ComparisonData>,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        if let Some(data) = comparison_data {
            // Create updated comparison data with current settings
            let common_data = CommonComparisonData {
                source_fields: data.source_fields.clone(),
                target_fields: data.target_fields.clone(),
                source_views: data.source_views.clone(),
                target_views: data.target_views.clone(),
                source_forms: data.source_forms.clone(),
                target_forms: data.target_forms.clone(),
                source_entity: data.source_entity.clone(),
                target_entity: data.target_entity.clone(),
                field_mappings: shared_state.field_mappings.clone(),
                prefix_mappings: shared_state.prefix_mappings.clone(),
                hide_matched: shared_state.hide_matched,
            };

            // Update all apps with refreshed data
            fields_app.update_data(&common_data);
            relationships_app.update_data(&common_data);
            views_app.update_data(&common_data);
            forms_app.update_data(&common_data);
        }
    }

    /// Handle list navigation
    pub fn handle_list_navigation<FA, RA, VA, FoA>(
        up: bool,
        active_tab: &ActiveTab,
        focused_side: &FocusedSide,
        fields_app: &mut FA,
        relationships_app: &mut RA,
        views_app: &mut VA,
        forms_app: &mut FoA,
    ) where
        FA: ComparisonApp + ?Sized,
        RA: ComparisonApp + ?Sized,
        VA: ComparisonApp + ?Sized,
        FoA: ComparisonApp + ?Sized,
    {
        // Handle navigation for the focused list - delegate to appropriate app
        match *active_tab {
            ActiveTab::Fields => {
                // Fields use hierarchical navigation - delegate to the fields app
                let common_focused_side = Self::convert_focused_side(*focused_side);
                fields_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Relationships => {
                // Relationships use hierarchical navigation - delegate to the relationships app
                let common_focused_side = Self::convert_focused_side(*focused_side);
                relationships_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Views => {
                // Views use hierarchical navigation - delegate to the views app
                let common_focused_side = Self::convert_focused_side(*focused_side);
                views_app.handle_list_navigation(up, common_focused_side);
            }
            ActiveTab::Forms => {
                // Forms use hierarchical navigation - delegate to the forms app
                let common_focused_side = Self::convert_focused_side(*focused_side);
                forms_app.handle_list_navigation(up, common_focused_side);
            }
        }
    }

    /// Switch to a specific tab
    pub fn switch_tab(active_tab: &mut ActiveTab, tab_number: usize) {
        *active_tab = match tab_number {
            1 => ActiveTab::Fields,
            2 => ActiveTab::Relationships,
            3 => ActiveTab::Views,
            4 => ActiveTab::Forms,
            _ => return, // Invalid tab number
        };
    }

    /// Get tab titles for UI
    pub fn get_tab_titles() -> Vec<&'static str> {
        vec!["[1] Fields", "[2] Relationships", "[3] Views", "[4] Forms"]
    }

    /// Get active tab index
    pub fn get_active_tab_index(active_tab: &ActiveTab) -> usize {
        match *active_tab {
            ActiveTab::Fields => 0,
            ActiveTab::Relationships => 1,
            ActiveTab::Views => 2,
            ActiveTab::Forms => 3,
        }
    }

    /// Convert local FocusedSide to common FocusedSide
    fn convert_focused_side(focused_side: FocusedSide) -> CommonFocusedSide {
        match focused_side {
            FocusedSide::Source => CommonFocusedSide::Source,
            FocusedSide::Target => CommonFocusedSide::Target,
        }
    }
}
