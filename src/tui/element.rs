use ratatui::style::Style;

/// Layout constraints for sizing elements within containers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutConstraint {
    /// Fixed size (exact number of lines/columns)
    Length(u16),
    /// Minimum size (at least this many lines/columns)
    Min(u16),
    /// Proportional fill (weight for distributing remaining space)
    Fill(u16),
}

/// Declarative UI elements that compose to form the view
pub enum Element<Msg> {
    /// Empty element that renders nothing
    None,

    /// Static text
    Text { content: String, style: Option<Style> },

    /// Interactive button
    Button {
        label: String,
        on_press: Option<Msg>,
        on_hover: Option<Msg>,
        on_hover_exit: Option<Msg>,
        style: Option<Style>,
    },

    /// Vertical layout container
    Column {
        items: Vec<(LayoutConstraint, Element<Msg>)>,
        spacing: u16,
    },

    /// Horizontal layout container
    Row {
        items: Vec<(LayoutConstraint, Element<Msg>)>,
        spacing: u16,
    },

    /// Container with padding/margins
    Container {
        child: Box<Element<Msg>>,
        padding: u16,
    },
}

impl<Msg> Element<Msg> {
    /// Create a text element
    pub fn text(content: impl Into<String>) -> Self {
        Element::Text {
            content: content.into(),
            style: None,
        }
    }

    /// Create a button element
    pub fn button(label: impl Into<String>) -> ButtonBuilder<Msg> {
        ButtonBuilder {
            label: label.into(),
            on_press: None,
            on_hover: None,
            on_hover_exit: None,
            style: None,
        }
    }

    /// Create a column layout (old API - backward compatible)
    pub fn column(children: Vec<Element<Msg>>) -> ColumnBuilder<Msg> {
        // Convert children to items with default constraints
        let items = children
            .into_iter()
            .map(|child| (child.default_constraint(), child))
            .collect();

        ColumnBuilder {
            items,
            spacing: 1,
        }
    }

    /// Create a row layout (old API - backward compatible)
    pub fn row(children: Vec<Element<Msg>>) -> RowBuilder<Msg> {
        // Convert children to items with default constraints
        let items = children
            .into_iter()
            .map(|child| (child.default_constraint(), child))
            .collect();

        RowBuilder {
            items,
            spacing: 1,
        }
    }

    /// Wrap element in a container
    pub fn container(child: Element<Msg>) -> ContainerBuilder<Msg> {
        ContainerBuilder {
            child: Box::new(child),
            padding: 1,
        }
    }

    /// Get the default layout constraint for this element type
    pub fn default_constraint(&self) -> LayoutConstraint {
        match self {
            Element::None => LayoutConstraint::Length(0),
            Element::Text { .. } => LayoutConstraint::Length(1),
            Element::Button { .. } => LayoutConstraint::Length(3),
            Element::Column { .. } => LayoutConstraint::Fill(1),
            Element::Row { .. } => LayoutConstraint::Fill(1),
            Element::Container { .. } => LayoutConstraint::Fill(1),
        }
    }
}

/// Builder for button elements
pub struct ButtonBuilder<Msg> {
    label: String,
    on_press: Option<Msg>,
    on_hover: Option<Msg>,
    on_hover_exit: Option<Msg>,
    style: Option<Style>,
}

impl<Msg> ButtonBuilder<Msg> {
    pub fn on_press(mut self, msg: Msg) -> Self {
        self.on_press = Some(msg);
        self
    }

    pub fn on_hover(mut self, msg: Msg) -> Self {
        self.on_hover = Some(msg);
        self
    }

    pub fn on_hover_exit(mut self, msg: Msg) -> Self {
        self.on_hover_exit = Some(msg);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Button {
            label: self.label,
            on_press: self.on_press,
            on_hover: self.on_hover,
            on_hover_exit: self.on_hover_exit,
            style: self.style,
        }
    }
}

/// Builder for column layouts
pub struct ColumnBuilder<Msg> {
    items: Vec<(LayoutConstraint, Element<Msg>)>,
    spacing: u16,
}

impl<Msg> ColumnBuilder<Msg> {
    /// Create a new column builder (for new API with explicit constraints)
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
        Element::Column {
            items: self.items,
            spacing: self.spacing,
        }
    }
}

/// Builder for row layouts
pub struct RowBuilder<Msg> {
    items: Vec<(LayoutConstraint, Element<Msg>)>,
    spacing: u16,
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

/// Builder for containers
pub struct ContainerBuilder<Msg> {
    child: Box<Element<Msg>>,
    padding: u16,
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

impl<Msg> Default for Element<Msg> {
    fn default() -> Self {
        Element::None
    }
}