use crate::tui::Element;
use crate::tui::element::FocusId;

/// Builder for scrollable elements
pub struct ScrollableBuilder<Msg> {
    pub(crate) id: FocusId,
    pub(crate) child: Box<Element<Msg>>,
    pub(crate) scroll_offset: usize,
    pub(crate) content_height: Option<usize>,
    pub(crate) on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    pub(crate) on_render: Option<fn(usize, usize) -> Msg>,
    pub(crate) on_focus: Option<Msg>,
    pub(crate) on_blur: Option<Msg>,
}

impl<Msg> ScrollableBuilder<Msg> {
    /// Set explicit content height (optional, auto-detected for Column)
    pub fn content_height(mut self, height: usize) -> Self {
        self.content_height = Some(height);
        self
    }

    /// Set callback for navigation keys (Up/Down/PageUp/PageDown/Home/End)
    pub fn on_navigate(mut self, msg: fn(crossterm::event::KeyCode) -> Msg) -> Self {
        self.on_navigate = Some(msg);
        self
    }

    /// Set callback for render events (called each frame with viewport_height, content_height)
    pub fn on_render(mut self, msg: fn(usize, usize) -> Msg) -> Self {
        self.on_render = Some(msg);
        self
    }

    pub fn on_focus(mut self, msg: Msg) -> Self {
        self.on_focus = Some(msg);
        self
    }

    pub fn on_blur(mut self, msg: Msg) -> Self {
        self.on_blur = Some(msg);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Scrollable {
            id: self.id,
            child: self.child,
            scroll_offset: self.scroll_offset,
            content_height: self.content_height,
            on_navigate: self.on_navigate,
            on_render: self.on_render,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}
