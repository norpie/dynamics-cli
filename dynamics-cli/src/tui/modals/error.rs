use crate::tui::{Element, Theme, FocusId};
use crate::tui::element::LayoutConstraint;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

/// Builder for error modals with auto-hotkey generation
///
/// # Example
/// ```rust
/// let modal = ErrorModal::new("Failed to load data")
///     .details(error_string)
///     .on_close(Msg::DismissError)
///     .build(theme);
/// ```
pub struct ErrorModal<Msg> {
    title: String,
    details: Option<String>,
    on_close: Option<Msg>,
    width: Option<u16>,
    height: Option<u16>,
}

impl<Msg: Clone> ErrorModal<Msg> {
    /// Create a new error modal with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            details: None,
            on_close: None,
            width: None,
            height: None,
        }
    }

    /// Set error details/description
    pub fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Set the message sent when closed (Escape)
    pub fn on_close(mut self, msg: Msg) -> Self {
        self.on_close = Some(msg);
        self
    }

    /// Set modal width (optional, auto-sizes by default)
    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set modal height (optional, auto-sizes by default)
    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    /// Build the modal Element
    pub fn build(self, theme: &Theme) -> Element<Msg> {
        // Error icon + title
        let title_element = Element::styled_text(Line::from(vec![
            Span::styled("✗ ", Style::default().fg(theme.red).bold()),
            Span::styled(self.title, Style::default().fg(theme.red).bold())
        ])).build();

        // Details element (if present)
        let details_elements: Vec<(LayoutConstraint, Element<Msg>)> = if let Some(details) = self.details {
            vec![
                (LayoutConstraint::Length(1), Element::text("")),
                (LayoutConstraint::Min(3), Element::text(details)),
            ]
        } else {
            vec![]
        };

        // Close button with hotkey indicator
        let close_button = Element::button(
            FocusId::new("error-close"),
            "[ Close (Esc) ]".to_string(),
        )
        .on_press(self.on_close.clone())
        .build();

        let button_row = Element::row()
            .item(LayoutConstraint::Fill(1), Element::text(""))
            .item(LayoutConstraint::Length(16), close_button)
            .item(LayoutConstraint::Fill(1), Element::text(""))
            .build();

        // Build the modal content
        let mut content_items = vec![
            (LayoutConstraint::Length(1), title_element),
        ];
        content_items.extend(details_elements);
        content_items.push((LayoutConstraint::Length(1), Element::text("")));
        content_items.push((LayoutConstraint::Length(3), button_row));

        let content = Element::column()
            .items(content_items)
            .build();

        // Wrap in panel with optional size
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
