use crate::tui::Element;
use crate::tui::element::FocusId;
use crate::tui::widgets::SelectEvent;

/// Builder for select/dropdown elements
pub struct SelectBuilder<Msg> {
    pub(crate) id: FocusId,
    pub(crate) options: Vec<String>,
    pub(crate) selected: usize,
    pub(crate) is_open: bool,
    pub(crate) highlight: usize,
    pub(crate) on_select: Option<fn(usize) -> Msg>,
    pub(crate) on_toggle: Option<Msg>,
    pub(crate) on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    pub(crate) on_event: Option<fn(SelectEvent) -> Msg>,
    pub(crate) on_focus: Option<Msg>,
    pub(crate) on_blur: Option<Msg>,
}

impl<Msg> SelectBuilder<Msg> {
    /// Set callback when option is selected
    pub fn on_select(mut self, msg: fn(usize) -> Msg) -> Self {
        self.on_select = Some(msg);
        self
    }

    /// Set callback when dropdown is toggled
    pub fn on_toggle(mut self, msg: Msg) -> Self {
        self.on_toggle = Some(msg);
        self
    }

    /// Set callback for keyboard navigation when dropdown is open
    pub fn on_navigate(mut self, msg: fn(crossterm::event::KeyCode) -> Msg) -> Self {
        self.on_navigate = Some(msg);
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

    /// Set unified event callback (new event pattern)
    /// This replaces on_select, on_toggle, and on_navigate
    pub fn on_event(mut self, msg: fn(SelectEvent) -> Msg) -> Self {
        self.on_event = Some(msg);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Select {
            id: self.id,
            options: self.options,
            selected: self.selected,
            is_open: self.is_open,
            highlight: self.highlight,
            on_select: self.on_select,
            on_toggle: self.on_toggle,
            on_navigate: self.on_navigate,
            on_event: self.on_event,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}
