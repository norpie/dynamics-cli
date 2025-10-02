use crate::tui::Element;

/// Builder for containers
pub struct ContainerBuilder<Msg> {
    pub(crate) child: Box<Element<Msg>>,
    pub(crate) padding: u16,
}

impl<Msg> ContainerBuilder<Msg> {
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Container {
            child: self.child,
            padding: self.padding,
        }
    }
}
