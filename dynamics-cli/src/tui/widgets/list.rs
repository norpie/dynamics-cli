use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};

/// Trait for items that can be displayed in a list
pub trait ListItem {
    type Msg: Clone;

    /// Render this item as an Element
    fn to_element(&self, is_selected: bool, is_hovered: bool) -> Element<Self::Msg>;

    /// Optional: height in lines (default 1)
    fn height(&self) -> u16 {
        1
    }
}

/// Manages list selection and scrolling state
#[derive(Debug, Clone)]
pub struct ListState {
    selected: Option<usize>,
    scroll_offset: usize,
    scroll_off: usize, // Rows from edge before scrolling (like vim scrolloff)
    wrap_around: bool, // Wrap to bottom/top when reaching edges
    viewport_height: Option<usize>, // Last known viewport height from renderer
}

impl Default for ListState {
    fn default() -> Self {
        Self::new()
    }
}

impl ListState {
    /// Create a new ListState with no selection
    pub fn new() -> Self {
        Self {
            selected: None,
            scroll_offset: 0,
            scroll_off: 3,
            wrap_around: true,
            viewport_height: None,
        }
    }

    /// Create a new ListState with first item selected
    pub fn with_selection() -> Self {
        Self {
            selected: Some(0),
            scroll_offset: 0,
            scroll_off: 3,
            wrap_around: true,
            viewport_height: None,
        }
    }

    /// Set the viewport height (called by renderer with actual area height)
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = Some(height);
    }

    /// Set the scroll-off distance (rows from edge before scrolling)
    pub fn with_scroll_off(mut self, scroll_off: usize) -> Self {
        self.scroll_off = scroll_off;
        self
    }

    /// Enable or disable wrap-around navigation
    pub fn with_wrap_around(mut self, wrap_around: bool) -> Self {
        self.wrap_around = wrap_around;
        self
    }

    /// Get currently selected index
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set selected index (useful for initialization)
    /// Note: This does NOT adjust scroll. Use select_and_scroll() if you need
    /// to ensure the selected item is visible.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Set selected index and adjust scroll to ensure it's visible
    /// This should be used when programmatically changing selection.
    pub fn select_and_scroll(&mut self, index: Option<usize>, item_count: usize) {
        self.selected = index;
        if let Some(height) = self.viewport_height {
            self.update_scroll(height, item_count);
        }
    }

    /// Handle navigation key, returns true if handled
    /// Uses stored viewport_height if available, otherwise falls back to provided visible_height
    pub fn handle_key(&mut self, key: KeyCode, item_count: usize, visible_height: usize) -> bool {
        if item_count == 0 {
            return false;
        }

        // Use stored viewport_height if available, otherwise use provided value
        let height = self.viewport_height.unwrap_or(visible_height);

        match key {
            KeyCode::Up => {
                self.move_up(item_count, height);
                true
            }
            KeyCode::Down => {
                self.move_down(item_count, height);
                true
            }
            KeyCode::PageUp => {
                self.page_up(height, item_count);
                true
            }
            KeyCode::PageDown => {
                self.page_down(item_count, height);
                true
            }
            KeyCode::Home => {
                self.select_first(height, item_count);
                true
            }
            KeyCode::End => {
                self.select_last(item_count, height);
                true
            }
            _ => false,
        }
    }

    fn move_up(&mut self, item_count: usize, visible_height: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(sel) = self.selected {
            if sel > 0 {
                self.selected = Some(sel - 1);
            } else if self.wrap_around {
                // At top, wrap to bottom
                self.selected = Some(item_count - 1);
            }
        } else {
            // No selection, select first
            self.selected = Some(0);
        }

        // Ensure the new selection is visible
        self.update_scroll(visible_height, item_count);
    }

    fn move_down(&mut self, item_count: usize, visible_height: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(sel) = self.selected {
            if sel < item_count - 1 {
                self.selected = Some(sel + 1);
            } else if self.wrap_around {
                // At bottom, wrap to top
                self.selected = Some(0);
            }
        } else {
            // No selection, select first
            self.selected = Some(0);
        }

        // Ensure the new selection is visible
        self.update_scroll(visible_height, item_count);
    }

    fn page_up(&mut self, visible_height: usize, item_count: usize) {
        if let Some(sel) = self.selected {
            let new_sel = sel.saturating_sub(visible_height);
            self.selected = Some(new_sel);
        } else {
            self.selected = Some(0);
        }

        // Ensure the new selection is visible
        self.update_scroll(visible_height, item_count);
    }

    fn page_down(&mut self, item_count: usize, visible_height: usize) {
        if let Some(sel) = self.selected {
            let new_sel = (sel + visible_height).min(item_count - 1);
            self.selected = Some(new_sel);
        } else if item_count > 0 {
            self.selected = Some(0);
        }

        // Ensure the new selection is visible
        self.update_scroll(visible_height, item_count);
    }

    fn select_first(&mut self, visible_height: usize, item_count: usize) {
        self.selected = Some(0);
        // Ensure the selection is visible
        self.update_scroll(visible_height, item_count);
    }

    fn select_last(&mut self, item_count: usize, visible_height: usize) {
        if item_count > 0 {
            self.selected = Some(item_count - 1);
            // Ensure the selection is visible
            self.update_scroll(visible_height, item_count);
        }
    }

    /// Update scroll offset based on selection and visible height
    /// Called during rendering to ensure scrolloff is maintained
    pub fn update_scroll(&mut self, visible_height: usize, item_count: usize) {
        if let Some(sel) = self.selected {
            // Calculate ideal scroll range to keep selection visible with scrolloff
            let min_scroll = sel.saturating_sub(visible_height.saturating_sub(self.scroll_off + 1));
            let max_scroll = sel.saturating_sub(self.scroll_off);

            if self.scroll_offset < min_scroll {
                self.scroll_offset = min_scroll;
            } else if self.scroll_offset > max_scroll {
                self.scroll_offset = max_scroll;
            }

            // Clamp to valid range
            let max_offset = item_count.saturating_sub(visible_height);
            self.scroll_offset = self.scroll_offset.min(max_offset);
        }
    }

    /// Handle list event (unified event pattern)
    /// Returns Some(selected_index) on Select event, None otherwise
    pub fn handle_event(&mut self, event: crate::tui::widgets::events::ListEvent, item_count: usize, visible_height: usize) -> Option<usize> {
        use crate::tui::widgets::events::ListEvent;

        match event {
            ListEvent::Navigate(key) => {
                self.handle_key(key, item_count, visible_height);
                None
            }
            ListEvent::Select => self.selected,
        }
    }
}
