use ratatui::style::Style;
use ratatui::text::Line;

/// Stable identifier for focusable UI elements
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FocusId(pub &'static str);

impl FocusId {
    /// Create a new FocusId with a static string identifier
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }
}

/// Alignment options for positioned elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// A layer in a stack of UI elements
pub struct Layer<Msg> {
    pub element: Element<Msg>,
    pub alignment: Alignment,
    pub dim_below: bool,
}

impl<Msg> Layer<Msg> {
    pub fn new(element: Element<Msg>) -> Self {
        Self {
            element,
            alignment: Alignment::TopLeft,
            dim_below: false,
        }
    }

    pub fn center(mut self) -> Self {
        self.alignment = Alignment::Center;
        self
    }

    pub fn align(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn dim(mut self, should_dim: bool) -> Self {
        self.dim_below = should_dim;
        self
    }
}

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

    /// Styled text with multiple spans
    StyledText {
        line: Line<'static>,
        background: Option<Style>,
    },

    /// Interactive button
    Button {
        id: FocusId,
        label: String,
        on_press: Option<Msg>,
        on_hover: Option<Msg>,
        on_hover_exit: Option<Msg>,
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
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

    /// Panel with border
    Panel {
        child: Box<Element<Msg>>,
        title: Option<String>,
        width: Option<u16>,
        height: Option<u16>,
    },

    /// Stack of layered elements (for modals, overlays)
    Stack {
        layers: Vec<Layer<Msg>>,
    },

    /// Scrollable list of items
    List {
        id: FocusId,
        items: Vec<Element<Msg>>,
        selected: Option<usize>,
        scroll_offset: usize,
        on_select: Option<fn(usize) -> Msg>,
        on_activate: Option<fn(usize) -> Msg>,
        on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
    },

    /// Single-line text input
    TextInput {
        id: FocusId,
        value: String,
        cursor_pos: usize,
        scroll_offset: usize,
        placeholder: Option<String>,
        max_length: Option<usize>,
        on_change: Option<fn(crossterm::event::KeyCode) -> Msg>,
        on_submit: Option<Msg>,
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
    },

    /// Hierarchical tree with expand/collapse
    Tree {
        id: FocusId,
        items: Vec<Element<Msg>>,       // Flattened nodes
        node_ids: Vec<String>,           // Parallel array of node IDs
        selected: Option<String>,        // Selected node ID (not index!)
        scroll_offset: usize,
        on_select: Option<fn(String) -> Msg>,     // ID-based callbacks
        on_toggle: Option<fn(String) -> Msg>,     // Expand/collapse callback
        on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
    },

    /// Scrollable wrapper for any element
    Scrollable {
        id: FocusId,
        child: Box<Element<Msg>>,
        scroll_offset: usize,
        content_height: Option<usize>,   // If None, auto-detect from Column
        on_scroll: Option<fn(usize) -> Msg>,
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
    },

    /// Select/Dropdown widget
    Select {
        id: FocusId,
        options: Vec<String>,               // Display labels for options
        selected: usize,                    // Selected index
        is_open: bool,                      // Dropdown open?
        highlight: usize,                   // Highlighted option (when open)
        on_select: Option<fn(usize) -> Msg>,  // Called when option selected
        on_toggle: Option<Msg>,             // Called when dropdown toggled
        on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,  // Called for keyboard navigation when open
        on_focus: Option<Msg>,
        on_blur: Option<Msg>,
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

    /// Create a styled text element with optional background fill
    pub fn styled_text(line: Line<'static>) -> StyledTextBuilder<Msg> {
        StyledTextBuilder {
            line,
            background: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a button element
    pub fn button(id: FocusId, label: impl Into<String>) -> ButtonBuilder<Msg> {
        ButtonBuilder {
            id,
            label: label.into(),
            on_press: None,
            on_hover: None,
            on_hover_exit: None,
            on_focus: None,
            on_blur: None,
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

    /// Wrap element in a panel with border
    pub fn panel(child: Element<Msg>) -> PanelBuilder<Msg> {
        PanelBuilder {
            child: Box::new(child),
            title: None,
            width: None,
            height: None,
        }
    }

    /// Create a stack of layers
    pub fn stack(layers: Vec<Layer<Msg>>) -> Self {
        Element::Stack { layers }
    }

    /// Create a confirmation modal overlay
    pub fn modal_confirm(
        background: Element<Msg>,
        title: impl Into<String>,
        message: impl Into<String>,
        cancel_id: FocusId,
        on_cancel: Msg,
        confirm_id: FocusId,
        on_confirm: Msg,
    ) -> Self {
        use crate::tui::element::RowBuilder;

        let button_row = RowBuilder::new()
            .add(
                Element::button(cancel_id, "Cancel").on_press(on_cancel).build(),
                LayoutConstraint::Fill(1),
            )
            .add(
                Element::text("  "),
                LayoutConstraint::Length(2),
            )
            .add(
                Element::button(confirm_id, "Confirm").on_press(on_confirm).build(),
                LayoutConstraint::Fill(1),
            )
            .spacing(0)
            .build();

        let modal_content = ColumnBuilder::new()
            .add(
                Element::text(title.into()),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(message.into()),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                button_row,
                LayoutConstraint::Length(3),
            )
            .spacing(0)
            .build();

        Element::stack(vec![
            Layer::new(background),
            Layer::new(
                Element::panel(
                    Element::container(modal_content)
                        .padding(1)
                        .build()
                )
                .title("Confirmation")
                .width(60)
                .height(15)
                .build()
            ).center(),
        ])
    }

    /// Get the default layout constraint for this element type
    pub fn default_constraint(&self) -> LayoutConstraint {
        match self {
            Element::None => LayoutConstraint::Length(0),
            Element::Text { .. } => LayoutConstraint::Length(1),
            Element::StyledText { .. } => LayoutConstraint::Length(1),
            Element::Button { .. } => LayoutConstraint::Length(3),
            Element::Column { .. } => LayoutConstraint::Fill(1),
            Element::Row { .. } => LayoutConstraint::Fill(1),
            Element::Container { .. } => LayoutConstraint::Fill(1),
            Element::Panel { child, height, .. } => {
                // If explicit height is set, use it
                if let Some(h) = height {
                    LayoutConstraint::Length(*h)
                } else {
                    // Panel sizes to child + 2 lines for borders (top + bottom)
                    match child.default_constraint() {
                        LayoutConstraint::Length(n) => LayoutConstraint::Length(n + 2),
                        LayoutConstraint::Min(n) => LayoutConstraint::Min(n + 2),
                        LayoutConstraint::Fill(w) => LayoutConstraint::Fill(w),
                    }
                }
            }
            Element::Stack { .. } => LayoutConstraint::Fill(1),
            Element::List { .. } => LayoutConstraint::Fill(1),
            Element::TextInput { .. } => LayoutConstraint::Length(1),
            Element::Tree { .. } => LayoutConstraint::Fill(1),
            Element::Scrollable { .. } => LayoutConstraint::Fill(1),
            Element::Select { .. } => LayoutConstraint::Length(3),  // Closed: 3 lines (border + content + border)
        }
    }

    /// Create a text input element
    pub fn text_input(
        id: FocusId,
        value: &str,
        state: &crate::tui::widgets::TextInputState,
    ) -> TextInputBuilder<Msg> {
        TextInputBuilder {
            id,
            value: value.to_string(),
            cursor_pos: state.cursor_pos(),
            scroll_offset: state.scroll_offset(),
            placeholder: None,
            max_length: None,
            on_change: None,
            on_submit: None,
            on_focus: None,
            on_blur: None,
        }
    }

    /// Create a list element from items
    pub fn list<T>(
        id: FocusId,
        items: &[T],
        state: &crate::tui::widgets::ListState,
        theme: &crate::tui::Theme,
    ) -> ListBuilder<Msg>
    where
        T: crate::tui::widgets::ListItem<Msg = Msg>,
    {
        let elements = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = state.selected() == Some(i);
                item.to_element(theme, is_selected, false)
            })
            .collect();

        ListBuilder {
            id,
            items: elements,
            selected: state.selected(),
            scroll_offset: state.scroll_offset(),
            on_select: None,
            on_activate: None,
            on_navigate: None,
            on_focus: None,
            on_blur: None,
        }
    }

    /// Create a tree element from TreeItem-implementing items
    pub fn tree<T>(
        id: FocusId,
        root_items: &[T],
        state: &mut crate::tui::widgets::TreeState,
        theme: &crate::tui::Theme,
    ) -> TreeBuilder<Msg>
    where
        T: crate::tui::widgets::TreeItem<Msg = Msg>,
    {
        // Flatten tree based on expansion state
        let flattened = crate::tui::widgets::tree::flatten_tree(root_items, state, theme);

        // Extract elements and node IDs (parallel arrays) by consuming the vec
        let (elements, node_ids): (Vec<Element<Msg>>, Vec<String>) = flattened
            .into_iter()
            .map(|node| (node.element, node.id))
            .unzip();

        TreeBuilder {
            id,
            items: elements,
            node_ids,
            selected: state.selected().map(String::from),
            scroll_offset: state.scroll_offset(),
            on_select: None,
            on_toggle: None,
            on_navigate: None,
            on_focus: None,
            on_blur: None,
        }
    }

    /// Create a scrollable wrapper around any element
    pub fn scrollable(
        id: FocusId,
        child: Element<Msg>,
        state: &crate::tui::widgets::ScrollableState,
    ) -> ScrollableBuilder<Msg> {
        ScrollableBuilder {
            id,
            child: Box::new(child),
            scroll_offset: state.scroll_offset(),
            content_height: None,
            on_scroll: None,
            on_focus: None,
            on_blur: None,
        }
    }

    /// Create a select/dropdown element
    pub fn select(
        id: FocusId,
        options: Vec<String>,
        state: &mut crate::tui::widgets::SelectState,
    ) -> SelectBuilder<Msg> {
        // Update state with current option count
        state.update_option_count(options.len());

        SelectBuilder {
            id,
            options,
            selected: state.selected(),
            is_open: state.is_open(),
            highlight: state.highlighted(),
            on_select: None,
            on_toggle: None,
            on_navigate: None,
            on_focus: None,
            on_blur: None,
        }
    }
}

/// Builder for styled text elements
pub struct StyledTextBuilder<Msg> {
    line: Line<'static>,
    background: Option<Style>,
    _phantom: std::marker::PhantomData<Msg>,
}

impl<Msg> StyledTextBuilder<Msg> {
    pub fn background(mut self, style: Style) -> Self {
        self.background = Some(style);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::StyledText {
            line: self.line,
            background: self.background,
        }
    }
}

/// Builder for button elements
pub struct ButtonBuilder<Msg> {
    id: FocusId,
    label: String,
    on_press: Option<Msg>,
    on_hover: Option<Msg>,
    on_hover_exit: Option<Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
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

    pub fn on_focus(mut self, msg: Msg) -> Self {
        self.on_focus = Some(msg);
        self
    }

    pub fn on_blur(mut self, msg: Msg) -> Self {
        self.on_blur = Some(msg);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn build(self) -> Element<Msg> {
        Element::Button {
            id: self.id,
            label: self.label,
            on_press: self.on_press,
            on_hover: self.on_hover,
            on_hover_exit: self.on_hover_exit,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
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

/// Builder for panels
pub struct PanelBuilder<Msg> {
    child: Box<Element<Msg>>,
    title: Option<String>,
    width: Option<u16>,
    height: Option<u16>,
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

/// Builder for list elements
pub struct ListBuilder<Msg> {
    id: FocusId,
    items: Vec<Element<Msg>>,
    selected: Option<usize>,
    scroll_offset: usize,
    on_select: Option<fn(usize) -> Msg>,
    on_activate: Option<fn(usize) -> Msg>,
    on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
}

impl<Msg> ListBuilder<Msg> {
    pub fn on_select(mut self, msg: fn(usize) -> Msg) -> Self {
        self.on_select = Some(msg);
        self
    }

    pub fn on_activate(mut self, msg: fn(usize) -> Msg) -> Self {
        self.on_activate = Some(msg);
        self
    }

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

    pub fn build(self) -> Element<Msg> {
        Element::List {
            id: self.id,
            items: self.items,
            selected: self.selected,
            scroll_offset: self.scroll_offset,
            on_select: self.on_select,
            on_activate: self.on_activate,
            on_navigate: self.on_navigate,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}

/// Builder for text input elements
pub struct TextInputBuilder<Msg> {
    id: FocusId,
    value: String,
    cursor_pos: usize,
    scroll_offset: usize,
    placeholder: Option<String>,
    max_length: Option<usize>,
    on_change: Option<fn(crossterm::event::KeyCode) -> Msg>,
    on_submit: Option<Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
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

/// Builder for tree elements
pub struct TreeBuilder<Msg> {
    id: FocusId,
    items: Vec<Element<Msg>>,
    node_ids: Vec<String>,
    selected: Option<String>,
    scroll_offset: usize,
    on_select: Option<fn(String) -> Msg>,
    on_toggle: Option<fn(String) -> Msg>,
    on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
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
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}

/// Builder for scrollable elements
pub struct ScrollableBuilder<Msg> {
    id: FocusId,
    child: Box<Element<Msg>>,
    scroll_offset: usize,
    content_height: Option<usize>,
    on_scroll: Option<fn(usize) -> Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
}

impl<Msg> ScrollableBuilder<Msg> {
    /// Set explicit content height (optional, auto-detected for Column)
    pub fn content_height(mut self, height: usize) -> Self {
        self.content_height = Some(height);
        self
    }

    /// Set callback when scroll position changes
    pub fn on_scroll(mut self, msg: fn(usize) -> Msg) -> Self {
        self.on_scroll = Some(msg);
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
            on_scroll: self.on_scroll,
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}

/// Builder for select/dropdown elements
pub struct SelectBuilder<Msg> {
    id: FocusId,
    options: Vec<String>,
    selected: usize,
    is_open: bool,
    highlight: usize,
    on_select: Option<fn(usize) -> Msg>,
    on_toggle: Option<Msg>,
    on_navigate: Option<fn(crossterm::event::KeyCode) -> Msg>,
    on_focus: Option<Msg>,
    on_blur: Option<Msg>,
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
            on_focus: self.on_focus,
            on_blur: self.on_blur,
        }
    }
}

impl<Msg> Default for Element<Msg> {
    fn default() -> Self {
        Element::None
    }
}