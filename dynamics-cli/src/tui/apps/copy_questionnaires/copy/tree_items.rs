use crate::tui::{Element, Theme, widgets::TreeItem};
use serde_json::Value;

/// Tree items for questionnaire snapshot visualization
#[derive(Clone)]
pub enum SnapshotTreeItem {
    Category { name: String, count: usize, children: Vec<SnapshotTreeItem> },
    Entity { name: String, id: String, children: Vec<SnapshotTreeItem> },
    Attribute { label: String, value: String },
}

impl TreeItem for SnapshotTreeItem {
    type Msg = super::Msg;

    fn id(&self) -> String {
        match self {
            Self::Category { name, .. } => format!("category:{}", name),
            Self::Entity { id, .. } => format!("entity:{}", id),
            Self::Attribute { label, value } => {
                // Generate stable ID based on label and value only
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                label.hash(&mut hasher);
                value.hash(&mut hasher);
                format!("attr:{}", hasher.finish())
            }
        }
    }

    fn has_children(&self) -> bool {
        match self {
            Self::Category { children, .. } => !children.is_empty(),
            Self::Entity { children, .. } => !children.is_empty(),
            Self::Attribute { .. } => false,
        }
    }

    fn children(&self) -> Vec<Self> {
        match self {
            Self::Category { children, .. } => children.clone(),
            Self::Entity { children, .. } => children.clone(),
            Self::Attribute { .. } => vec![],
        }
    }

    fn to_element(
        &self,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Element<Self::Msg> {
        use ratatui::{style::Style, text::{Line, Span}, prelude::Stylize};

        let theme = &crate::global_runtime_config().theme;
        let indent = "  ".repeat(depth);

        match self {
            Self::Category { name, count, children } => {
                let mut spans = Vec::new();

                // Indent
                if depth > 0 {
                    spans.push(Span::styled(indent, Style::default()));
                }

                // Expand/collapse indicator for categories with children
                if !children.is_empty() {
                    let indicator = if is_expanded { "▼ " } else { "▶ " };
                    spans.push(Span::styled(indicator, Style::default().fg(theme.border_primary)));
                }

                // Category name with count
                spans.push(Span::styled(
                    format!("{} ({})", name, count),
                    Style::default().fg(theme.text_primary).bold(),
                ));

                let mut builder = Element::styled_text(Line::from(spans));

                if is_selected {
                    builder = builder.background(Style::default().bg(theme.bg_surface));
                }

                builder.build()
            }
            Self::Entity { name, children, .. } => {
                let mut spans = Vec::new();

                // Indent
                if depth > 0 {
                    spans.push(Span::styled(indent, Style::default()));
                }

                // Expand/collapse indicator if entity has children (tag/template)
                if !children.is_empty() {
                    let indicator = if is_expanded { "▼ " } else { "▶ " };
                    spans.push(Span::styled(indicator, Style::default().fg(theme.border_primary)));
                }

                // Entity name
                spans.push(Span::styled(
                    name.clone(),
                    Style::default().fg(theme.text_secondary),
                ));

                let mut builder = Element::styled_text(Line::from(spans));

                if is_selected {
                    builder = builder.background(Style::default().bg(theme.bg_surface));
                }

                builder.build()
            }
            Self::Attribute { label, value } => {
                let mut spans = Vec::new();

                // Indent
                if depth > 0 {
                    spans.push(Span::styled(indent, Style::default()));
                }

                // Label in dim color
                spans.push(Span::styled(
                    format!("{}: ", label),
                    Style::default().fg(theme.text_tertiary),
                ));

                // Value
                spans.push(Span::styled(
                    value.clone(),
                    Style::default().fg(theme.text_secondary),
                ));

                let mut builder = Element::styled_text(Line::from(spans));

                if is_selected {
                    builder = builder.background(Style::default().bg(theme.bg_surface));
                }

                builder.build()
            }
        }
    }
}
