//! Examples modal for managing example record pairs

use crate::tui::{Element, Theme, FocusId};
use crate::tui::element::{LayoutConstraint, RowBuilder, ColumnBuilder};
use crate::tui::widgets::{ListState, ListItem, TextInputField};
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

/// Example pair for display in the list
#[derive(Clone)]
pub struct ExamplePairItem<Msg> {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub label: Option<String>,
    pub on_delete: Msg,
}

impl<Msg: Clone> ListItem for ExamplePairItem<Msg> {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Self::Msg> {
        let display = if let Some(label) = &self.label {
            format!("{} ({}... → {}...)",
                label,
                &self.source_id[..8.min(self.source_id.len())],
                &self.target_id[..8.min(self.target_id.len())]
            )
        } else {
            format!("{}... → {}...",
                &self.source_id[..8.min(self.source_id.len())],
                &self.target_id[..8.min(self.target_id.len())]
            )
        };

        let style = if is_selected {
            Style::default().fg(theme.lavender).bold()
        } else {
            Style::default().fg(theme.text)
        };

        Element::styled_text(Line::from(vec![
            Span::styled(display, style)
        ])).build()
    }
}

/// Builder for examples management modal
///
/// # Example
/// ```rust
/// let modal = ExamplesModal::new()
///     .pairs(example_pairs)
///     .source_input_field(source_field)
///     .target_input_field(target_field)
///     .label_input_field(label_field)
///     .list_state(list_state)
///     .on_add(Msg::AddExamplePair)
///     .on_delete(Msg::DeleteExamplePair)
///     .on_fetch(Msg::FetchExampleData)
///     .on_close(Msg::CloseExamplesModal)
///     .build(theme);
/// ```
pub struct ExamplesModal<Msg> {
    pairs: Vec<ExamplePairItem<Msg>>,
    source_input_state: TextInputField,
    target_input_state: TextInputField,
    label_input_state: TextInputField,
    list_state: ListState,
    on_source_input_event: Option<fn(crate::tui::widgets::TextInputEvent) -> Msg>,
    on_target_input_event: Option<fn(crate::tui::widgets::TextInputEvent) -> Msg>,
    on_label_input_event: Option<fn(crate::tui::widgets::TextInputEvent) -> Msg>,
    on_list_event: Option<fn(crate::tui::widgets::ListEvent) -> Msg>,
    on_add: Option<Msg>,
    on_delete: Option<Msg>,
    on_fetch: Option<Msg>,
    on_close: Option<Msg>,
    width: Option<u16>,
    height: Option<u16>,
}

impl<Msg: Clone> ExamplesModal<Msg> {
    /// Create a new examples modal
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            source_input_state: TextInputField::new(),
            target_input_state: TextInputField::new(),
            label_input_state: TextInputField::new(),
            list_state: ListState::new(),
            on_source_input_event: None,
            on_target_input_event: None,
            on_label_input_event: None,
            on_list_event: None,
            on_add: None,
            on_delete: None,
            on_fetch: None,
            on_close: None,
            width: Some(80),
            height: Some(30),
        }
    }

    /// Set the list of example pairs
    pub fn pairs(mut self, pairs: Vec<ExamplePairItem<Msg>>) -> Self {
        self.pairs = pairs;
        self
    }

    /// Set the source input state
    pub fn source_input_state(mut self, state: TextInputField) -> Self {
        self.source_input_state = state;
        self
    }

    /// Set the target input state
    pub fn target_input_state(mut self, state: TextInputField) -> Self {
        self.target_input_state = state;
        self
    }

    /// Set the label input state
    pub fn label_input_state(mut self, state: TextInputField) -> Self {
        self.label_input_state = state;
        self
    }

    /// Set the list state
    pub fn list_state(mut self, state: ListState) -> Self {
        self.list_state = state;
        self
    }

    /// Set source input event handler
    pub fn on_source_input_event(mut self, handler: fn(crate::tui::widgets::TextInputEvent) -> Msg) -> Self {
        self.on_source_input_event = Some(handler);
        self
    }

    /// Set target input event handler
    pub fn on_target_input_event(mut self, handler: fn(crate::tui::widgets::TextInputEvent) -> Msg) -> Self {
        self.on_target_input_event = Some(handler);
        self
    }

    /// Set label input event handler
    pub fn on_label_input_event(mut self, handler: fn(crate::tui::widgets::TextInputEvent) -> Msg) -> Self {
        self.on_label_input_event = Some(handler);
        self
    }

    /// Set list event handler
    pub fn on_list_event(mut self, handler: fn(crate::tui::widgets::ListEvent) -> Msg) -> Self {
        self.on_list_event = Some(handler);
        self
    }

    /// Set the message sent when Add is clicked
    pub fn on_add(mut self, msg: Msg) -> Self {
        self.on_add = Some(msg);
        self
    }

    /// Set the message sent when Delete is clicked
    pub fn on_delete(mut self, msg: Msg) -> Self {
        self.on_delete = Some(msg);
        self
    }

    /// Set the message sent when Fetch is clicked
    pub fn on_fetch(mut self, msg: Msg) -> Self {
        self.on_fetch = Some(msg);
        self
    }

    /// Set the message sent when Close is clicked
    pub fn on_close(mut self, msg: Msg) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// Set modal width
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set modal height
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    /// Build the modal Element
    pub fn build(self, theme: &Theme) -> Element<Msg> {
        // Title
        let title = Element::styled_text(Line::from(vec![
            Span::styled("Manage Example Pairs", Style::default().fg(theme.mauve).bold())
        ])).build();

        // Existing pairs list
        let list_label = Element::text("Existing Pairs:");

        let pairs_list = if !self.pairs.is_empty() {
            Element::list(
                FocusId::new("examples-list"),
                &self.pairs,
                &self.list_state,
                theme,
            ).build()
        } else {
            Element::text("  (no pairs yet)")
        };

        // Form section
        let form_label = Element::styled_text(Line::from(vec![
            Span::styled("Add New Pair:", Style::default().fg(theme.blue).bold())
        ])).build();

        // Build text input elements from state
        let source_handler = self.on_source_input_event
            .expect("ExamplesModal requires on_source_input_event");
        let source_input = Element::text_input(
            FocusId::new("examples-source-input"),
            &self.source_input_state.value,
            &self.source_input_state.state,
        )
        .placeholder("Enter source record UUID")
        .on_event(source_handler)
        .build();

        let target_handler = self.on_target_input_event
            .expect("ExamplesModal requires on_target_input_event");
        let target_input = Element::text_input(
            FocusId::new("examples-target-input"),
            &self.target_input_state.value,
            &self.target_input_state.state,
        )
        .placeholder("Enter target record UUID")
        .on_event(target_handler)
        .build();

        let label_handler = self.on_label_input_event
            .expect("ExamplesModal requires on_label_input_event");
        let label_input = Element::text_input(
            FocusId::new("examples-label-input"),
            &self.label_input_state.value,
            &self.label_input_state.state,
        )
        .placeholder("Optional label")
        .on_event(label_handler)
        .build();

        // Buttons
        let add_msg = self.on_add.clone()
            .expect("ExamplesModal requires on_add callback");
        let delete_msg = self.on_delete.clone()
            .expect("ExamplesModal requires on_delete callback");
        let fetch_msg = self.on_fetch.clone()
            .expect("ExamplesModal requires on_fetch callback");
        let close_msg = self.on_close.clone()
            .expect("ExamplesModal requires on_close callback");

        let add_button = Element::button(
            FocusId::new("examples-add"),
            "[ (a)dd ]",
        )
        .on_press(add_msg)
        .style(Style::default().fg(theme.green))
        .build();

        let delete_button = Element::button(
            FocusId::new("examples-delete"),
            "[ (d)elete ]",
        )
        .on_press(delete_msg)
        .style(Style::default().fg(theme.red))
        .build();

        let fetch_button = Element::button(
            FocusId::new("examples-fetch"),
            "[ (f)etch ]",
        )
        .on_press(fetch_msg)
        .style(Style::default().fg(theme.blue))
        .build();

        let close_button = Element::button(
            FocusId::new("examples-close"),
            "[ (c)lose ]",
        )
        .on_press(close_msg)
        .build();

        let buttons = RowBuilder::new()
            .add(add_button, LayoutConstraint::Fill(1))
            .add(delete_button, LayoutConstraint::Fill(1))
            .add(fetch_button, LayoutConstraint::Fill(1))
            .add(close_button, LayoutConstraint::Fill(1))
            .spacing(2)
            .build();

        // Build modal content
        let content = ColumnBuilder::new()
            .add(title, LayoutConstraint::Length(1))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(list_label, LayoutConstraint::Length(1))
            .add(pairs_list, LayoutConstraint::Fill(1))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(form_label, LayoutConstraint::Length(1))
            .add(Element::text("Source Record ID:"), LayoutConstraint::Length(1))
            .add(source_input, LayoutConstraint::Length(3))
            .add(Element::text("Target Record ID:"), LayoutConstraint::Length(1))
            .add(target_input, LayoutConstraint::Length(3))
            .add(Element::text("Label (optional):"), LayoutConstraint::Length(1))
            .add(label_input, LayoutConstraint::Length(3))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(buttons, LayoutConstraint::Length(3))
            .build();

        // Wrap in panel
        let mut panel = Element::panel(content);

        if let Some(w) = self.width {
            panel = panel.width(w);
        }
        if let Some(h) = self.height {
            panel = panel.height(h);
        }

        panel.build()
    }
}

impl<Msg: Clone> Default for ExamplesModal<Msg> {
    fn default() -> Self {
        Self::new()
    }
}
