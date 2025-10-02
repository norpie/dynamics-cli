use crate::tui::Element;
use crate::tui::element::FocusId;
use crate::tui::widgets::TreeEvent;

/// Builder for tree elements
pub struct TreeBuilder<Msg> {
    pub(crate) id: FocusId,
    pub(crate) items: Vec<Element<Msg>>,
    pub(crate) node_ids: Vec<String>,
    pub(crate) selected: Option<String>,
    pub(crate) scroll_offset: usize,
    pub(crate) on_select: Option<fn(String) -> Msg>,
    pub(crate) on_toggle: Option<fn(String) -> Msg>,
    pub(crate) on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    pub(crate) on_event: Option<fn(TreeEvent) -> Msg>,
    pub(crate) on_focus: Option<Msg>,
    pub(crate) on_blur: Option<Msg>,
}

impl<Msg> TreeBuilder<Msg> {
    pub fn on_select(mut self, msg: fn(String) -> Msg) -> Self {
        self.on_select = Some(msg);
        self
    }

    pub fn on_toggle(mut self, msg: fn(String) -> Msg) -> Self {
        self.on_toggle = Some(msg);
        self
    }

    pub fn on_navigate(mut self, msg: fn(crossterm::event::KeyCode) -> Msg) -> Self {
        self.on_navigate = Some(msg);
        self
    }

    pub fn on_event(mut self, msg: fn(TreeEvent) -> Msg) -> Self {
        self.on_event = Some(msg);
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
        Element::Tree {
            id: self.id,
            items: self.items,
            node_ids: self.node_ids,
            selected: self.selected,
            scroll_offset: self.scroll_offset,
            on_select: self.on_select,
            on_toggle: self.on_toggle,
            on_navigate: self.on_navigate,
            on_event: self.on_event,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}
