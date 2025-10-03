use crate::tui::{Element, LayoutConstraint};

/// Builder for column layouts
pub struct ColumnBuilder<Msg> {
    pub(crate) items: Vec<(LayoutConstraint, Element<Msg>)>,
    pub(crate) spacing: u16,
}

impl<Msg> ColumnBuilder<Msg> {
    /// Create a new column builder (for new API with explicit constraints)
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            spacing: 1,
        }
    }

    /// Create a column builder from existing items (for macros)
    pub fn from_items(items: Vec<(LayoutConstraint, Element<Msg>)>) -> Self {
        Self { items, spacing: 1 }
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
        Element::Column {
            items: self.items,
            spacing: self.spacing,
        }
    }
}
