use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};

/// Trait for items that can be displayed in a list
pub trait ListItem {
    type Msg: Clone;

    /// Render this item as an Element
    fn to_element(&self, theme: &Theme, is_selected: bool, is_hovered: bool) -> Element<Self::Msg>;

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
        }
    }

    /// Create a new ListState with first item selected
    pub fn with_selection() -> Self {
        Self {
            selected: Some(0),
            scroll_offset: 0,
            scroll_off: 3,
            wrap_around: true,
        }
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
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Handle navigation key, returns true if handled
    pub fn handle_key(&mut self, key: KeyCode, item_count: usize, visible_height: usize) -> bool {
        if item_count == 0 {
            return false;
        }

        match key {
            KeyCode::Up => {
                self.move_up(item_count);
                true
            }
            KeyCode::Down => {
                self.move_down(item_count);
                true
            }
            KeyCode::PageUp => {
                self.page_up(visible_height);
                true
            }
            KeyCode::PageDown => {
                self.page_down(item_count, visible_height);
                true
            }
            KeyCode::Home => {
                self.select_first();
                true
            }
            KeyCode::End => {
                self.select_last(item_count);
                true
            }
            _ => false,
        }
    }

    fn move_up(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(sel) = self.selected {
            if sel > 0 {
                self.selected = Some(sel - 1);
                // Adjust scroll if needed (scrolloff logic)
                if (sel as isize - self.scroll_offset as isize) <= self.scroll_off as isize {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
            } else if self.wrap_around {
                // At top, wrap to bottom
                self.selected = Some(item_count - 1);
            }
        } else {
            // No selection, select first
            self.selected = Some(0);
        }
    }

    fn move_down(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }

        if let Some(sel) = self.selected {
            if sel < item_count - 1 {
                self.selected = Some(sel + 1);
                // Adjust scroll if needed (scrolloff logic)
                // We need visible_height for this, but we'll handle it in the renderer
                // For now, just update selection
            } else if self.wrap_around {
                // At bottom, wrap to top
                self.selected = Some(0);
                self.scroll_offset = 0;
            }
        } else {
            // No selection, select first
            self.selected = Some(0);
        }
    }

    fn page_up(&mut self, visible_height: usize) {
        if let Some(sel) = self.selected {
            let new_sel = sel.saturating_sub(visible_height);
            self.selected = Some(new_sel);
            self.scroll_offset = self.scroll_offset.saturating_sub(visible_height);
        } else {
            self.selected = Some(0);
        }
    }

    fn page_down(&mut self, item_count: usize, visible_height: usize) {
        if let Some(sel) = self.selected {
            let new_sel = (sel + visible_height).min(item_count - 1);
            self.selected = Some(new_sel);
            self.scroll_offset = (self.scroll_offset + visible_height).min(item_count.saturating_sub(visible_height));
        } else if item_count > 0 {
            self.selected = Some(0);
        }
    }

    fn select_first(&mut self) {
        self.selected = Some(0);
        self.scroll_offset = 0;
    }

    fn select_last(&mut self, item_count: usize) {
        if item_count > 0 {
            self.selected = Some(item_count - 1);
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
}
