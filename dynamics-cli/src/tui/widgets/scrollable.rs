/// Manages scrolling state for Scrollable elements
#[derive(Debug, Clone)]
pub struct ScrollableState {
    scroll_offset: usize,
    viewport_height: Option<usize>, // Last known viewport height from renderer
    content_height: Option<usize>,  // Last known content height from renderer
}

impl Default for ScrollableState {
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollableState {
    /// Create a new ScrollableState
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            viewport_height: None,
            content_height: None,
        }
    }

    /// Set the viewport height (called by renderer with actual area height)
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = Some(height);
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Get viewport height
    pub fn viewport_height(&self) -> Option<usize> {
        self.viewport_height
    }

    /// Get content height
    pub fn content_height(&self) -> Option<usize> {
        self.content_height
    }

    /// Handle keyboard navigation
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode, content_height: usize, visible_height: usize) {
        if content_height == 0 {
            return;
        }

        // Use stored viewport_height if available, otherwise use provided value
        let height = self.viewport_height.unwrap_or(visible_height);
        let max_scroll = content_height.saturating_sub(height);

        match key {
            crossterm::event::KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            crossterm::event::KeyCode::Down => {
                self.scroll_offset = (self.scroll_offset + 1).min(max_scroll);
            }
            crossterm::event::KeyCode::PageUp => {
                self.scroll_offset = self.scroll_offset.saturating_sub(height);
            }
            crossterm::event::KeyCode::PageDown => {
                self.scroll_offset = (self.scroll_offset + height).min(max_scroll);
            }
            crossterm::event::KeyCode::Home => {
                self.scroll_offset = 0;
            }
            crossterm::event::KeyCode::End => {
                self.scroll_offset = max_scroll;
            }
            _ => {}
        }
    }

    /// Update scroll offset to stay within bounds
    /// Called during rendering with actual dimensions
    pub fn update_scroll(&mut self, visible_height: usize, content_height: usize) {
        self.content_height = Some(content_height);

        if content_height == 0 {
            self.scroll_offset = 0;
            return;
        }

        // Don't scroll if all content fits on screen
        if content_height <= visible_height {
            self.scroll_offset = 0;
            return;
        }

        // Clamp scroll to valid range
        let max_offset = content_height.saturating_sub(visible_height);
        self.scroll_offset = self.scroll_offset.min(max_offset);
    }
}
