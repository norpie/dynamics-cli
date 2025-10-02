use crate::tui::Element;
use crate::tui::element::FocusId;

/// Builder for text input elements
pub struct TextInputBuilder<Msg> {
    pub(crate) id: FocusId,
    pub(crate) value: String,
    pub(crate) cursor_pos: usize,
    pub(crate) scroll_offset: usize,
    pub(crate) placeholder: Option<String>,
    pub(crate) max_length: Option<usize>,
    pub(crate) on_change: Option<fn(crossterm::event::KeyCode) -> Msg>,
    pub(crate) on_submit: Option<Msg>,
    pub(crate) on_focus: Option<Msg>,
    pub(crate) on_blur: Option<Msg>,
}

impl<Msg> TextInputBuilder<Msg> {
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    pub fn max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    pub fn on_change(mut self, msg: fn(crossterm::event::KeyCode) -> Msg) -> Self {
        self.on_change = Some(msg);
        self
    }

    pub fn on_submit(mut self, msg: Msg) -> Self {
        self.on_submit = Some(msg);
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
        Element::TextInput {
            id: self.id,
            value: self.value,
            cursor_pos: self.cursor_pos,
            scroll_offset: self.scroll_offset,
            placeholder: self.placeholder,
            max_length: self.max_length,
            on_change: self.on_change,
            on_submit: self.on_submit,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}
