use ratatui::style::Style;

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
        children: Vec<Element<Msg>>,
        spacing: u16,
    },

    /// Horizontal layout container
    Row {
        children: Vec<Element<Msg>>,
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

    /// Create a column layout
    pub fn column(children: Vec<Element<Msg>>) -> ColumnBuilder<Msg> {
        ColumnBuilder {
            children,
            spacing: 1,
        }
    }

    /// Create a row layout
    pub fn row(children: Vec<Element<Msg>>) -> RowBuilder<Msg> {
        RowBuilder {
            children,
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
    children: Vec<Element<Msg>>,
    spacing: u16,
}

impl<Msg> ColumnBuilder<Msg> {
    pub fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Column {
            children: self.children,
            spacing: self.spacing,
        }
    }
}

/// Builder for row layouts
pub struct RowBuilder<Msg> {
    children: Vec<Element<Msg>>,
    spacing: u16,
}

impl<Msg> RowBuilder<Msg> {
    pub fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Row {
            children: self.children,
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