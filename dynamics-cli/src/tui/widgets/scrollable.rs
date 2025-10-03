/// Manages scrolling state for Scrollable elements
#[derive(Debug, Clone)]
pub struct ScrollableState {
    scroll_offset: usize,
    scroll_off: usize,  // Distance from edge before scrolling (vim scrolloff)
    content_height: Option<usize>,  // Cached content height
    viewport_height: Option<usize>, // Cached viewport height
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
            scroll_off: 3,
            content_height: None,
            viewport_height: None,
        }
    }

    /// Set the scroll-off distance (rows from edge before scrolling)
    pub fn with_scroll_off(mut self, scroll_off: usize) -> Self {
        self.scroll_off = scroll_off;
        self
    }

    /// Get current scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set scroll offset directly (will be clamped)
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset;
        self.clamp_scroll();
    }

    /// Update content and viewport dimensions
    pub fn update_dimensions(&mut self, content_height: usize, viewport_height: usize) {
        self.content_height = Some(content_height);
        self.viewport_height = Some(viewport_height);
        self.clamp_scroll();
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
        self.clamp_scroll();
    }

    /// Scroll to top
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        if let (Some(content), Some(viewport)) = (self.content_height, self.viewport_height) {
            self.scroll_offset = content.saturating_sub(viewport);
        }
    }

    /// Page up
    pub fn page_up(&mut self) {
        let page_size = self.viewport_height.unwrap_or(10);
        self.scroll_up(page_size);
    }

    /// Page down
    pub fn page_down(&mut self) {
        let page_size = self.viewport_height.unwrap_or(10);
        self.scroll_down(page_size);
    }

    /// Clamp scroll offset to valid range
    fn clamp_scroll(&mut self) {
        if let (Some(content), Some(viewport)) = (self.content_height, self.viewport_height) {
            let max_scroll = content.saturating_sub(viewport);
            self.scroll_offset = self.scroll_offset.min(max_scroll);
        }
    }

    /// Handle keyboard navigation (like ListState::handle_key)
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode, content_height: usize, viewport_height: usize) {
        // Update dimensions first
        self.update_dimensions(content_height, viewport_height);

        match key {
            crossterm::event::KeyCode::Up => self.scroll_up(1),
            crossterm::event::KeyCode::Down => self.scroll_down(1),
            crossterm::event::KeyCode::PageUp => self.page_up(),
            crossterm::event::KeyCode::PageDown => self.page_down(),
            crossterm::event::KeyCode::Home => self.scroll_to_top(),
            crossterm::event::KeyCode::End => self.scroll_to_bottom(),
            _ => {}
        }
    }
}
