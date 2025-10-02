use crate::tui::{Element, LayoutConstraint};

/// Builder for row layouts
pub struct RowBuilder<Msg> {
    pub(crate) items: Vec<(LayoutConstraint, Element<Msg>)>,
    pub(crate) spacing: u16,
}

impl<Msg> RowBuilder<Msg> {
    /// Create a new row builder (for new API with explicit constraints)
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            spacing: 1,
        }
    }

    /// Add a child with an explicit layout constraint
    pub fn add(mut self, child: Element<Msg>, constraint: LayoutConstraint) -> Self {
        self.items.push((constraint, child));
        self
    }

    pub fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Row {
            items: self.items,
            spacing: self.spacing,
        }
    }
}
