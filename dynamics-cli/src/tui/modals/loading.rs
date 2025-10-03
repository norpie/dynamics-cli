use crate::tui::{Element, Theme};
use crate::tui::element::LayoutConstraint;
use ratatui::prelude::*;
use ratatui::text::{Line, Span};

/// Builder for loading modals with task tracking
///
/// # Example
/// ```rust
/// let modal = LoadingModal::new("Loading entities...")
///     .tasks(vec!["Fetching contacts", "Fetching accounts"])
///     .build(theme);
/// ```
pub struct LoadingModal<Msg> {
    title: String,
    tasks: Vec<String>,
    width: Option<u16>,
    height: Option<u16>,
    _phantom: std::marker::PhantomData<Msg>,
}

impl<Msg> LoadingModal<Msg> {
    /// Create a new loading modal with a title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            tasks: vec![],
            width: None,
            height: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Set the list of tasks being performed
    pub fn tasks(mut self, tasks: Vec<impl Into<String>>) -> Self {
        self.tasks = tasks.into_iter().map(|s| s.into()).collect();
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
        // Loading spinner + title
        let title_element = Element::styled_text(Line::from(vec![
            Span::styled("⏳ ", Style::default().fg(theme.blue).bold()),
            Span::styled(self.title, Style::default().fg(theme.text).bold())
        ])).build();

        // Task list elements
        let mut task_elements: Vec<(LayoutConstraint, Element<Msg>)> = vec![];

        if !self.tasks.is_empty() {
            task_elements.push((LayoutConstraint::Length(1), Element::text("")));

            for task in self.tasks {
                let task_line = Element::styled_text(Line::from(vec![
                    Span::styled("  • ", Style::default().fg(theme.subtext1)),
                    Span::styled(task, Style::default().fg(theme.subtext1))
                ])).build();
                task_elements.push((LayoutConstraint::Length(1), task_line));
            }
        }

        // Build the modal content
        let mut content_items = vec![
            (LayoutConstraint::Length(1), title_element),
        ];
        content_items.extend(task_elements);
        content_items.push((LayoutConstraint::Length(1), Element::text("")));

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
