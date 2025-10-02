use crate::tui::Element;

/// Builder for panels
pub struct PanelBuilder<Msg> {
    pub(crate) child: Box<Element<Msg>>,
    pub(crate) title: Option<String>,
    pub(crate) width: Option<u16>,
    pub(crate) height: Option<u16>,
}

impl<Msg> PanelBuilder<Msg> {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Panel {
            child: self.child,
            title: self.title,
            width: self.width,
            height: self.height,
        }
    }
}
