use ratatui::layout::Rect;

/// Stores interaction handlers for UI elements
/// Maps (Rect, InteractionType) -> Message
pub struct InteractionRegistry<Msg> {
    click_handlers: Vec<(Rect, Msg)>,
    hover_handlers: Vec<(Rect, Msg)>,
    hover_exit_handlers: Vec<(Rect, Msg)>,
}

impl<Msg: Clone> InteractionRegistry<Msg> {
    pub fn new() -> Self {
        Self {
            click_handlers: Vec::new(),
            hover_handlers: Vec::new(),
            hover_exit_handlers: Vec::new(),
        }
    }

    pub fn register_click(&mut self, rect: Rect, msg: Msg) {
        self.click_handlers.push((rect, msg));
    }

    pub fn register_hover(&mut self, rect: Rect, msg: Msg) {
        self.hover_handlers.push((rect, msg));
    }

    pub fn register_hover_exit(&mut self, rect: Rect, msg: Msg) {
        self.hover_exit_handlers.push((rect, msg));
    }

    pub fn find_click(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.click_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.hover_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover_exit(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.hover_exit_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.click_handlers.clear();
        self.hover_handlers.clear();
        self.hover_exit_handlers.clear();
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}
