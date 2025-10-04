//! TreeItem implementations for entity comparison

use crate::tui::{Element, Theme, widgets::TreeItem};
use crate::api::{FieldMetadata, RelationshipMetadata, ViewMetadata, FormMetadata};
use ratatui::{style::Style, text::{Line, Span}};
use super::models::{MatchInfo, MatchType};

/// Field node in the tree
#[derive(Clone)]
pub struct FieldNode {
    pub metadata: FieldMetadata,
    pub match_info: Option<MatchInfo>,
    pub example_value: Option<String>,
}

impl TreeItem for FieldNode {
    type Msg = super::Msg;

    fn id(&self) -> String {
        self.metadata.logical_name.clone()
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Self> {
        vec![]
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        // TODO: Implement rich field rendering with:
        // - Match status badge
        // - Example value display
        // - Color coding by match status
        // - Field type information

        let indent = "  ".repeat(depth);
        let field_name = &self.metadata.logical_name;

        let mut text = format!("{}{}", indent, field_name);

        // Add match badge if present
        if let Some(match_info) = &self.match_info {
            text.push_str(" ");
            text.push_str(match_info.match_type.label());
        }

        // Add example value if present
        if let Some(example) = &self.example_value {
            text.push_str(" = ");
            text.push_str(example);
        }

        Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        ))).build()
    }
}

/// Relationship node in the tree
#[derive(Clone)]
pub struct RelationshipNode {
    pub metadata: RelationshipMetadata,
}

impl TreeItem for RelationshipNode {
    type Msg = super::Msg;

    fn id(&self) -> String {
        format!("rel_{}", self.metadata.name)
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Self> {
        vec![]
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        // TODO: Implement relationship rendering

        let indent = "  ".repeat(depth);
        let text = format!("{}{}", indent, self.metadata.name);

        Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        ))).build()
    }
}

/// View node in the tree
#[derive(Clone)]
pub struct ViewNode {
    pub metadata: ViewMetadata,
}

impl TreeItem for ViewNode {
    type Msg = super::Msg;

    fn id(&self) -> String {
        format!("view_{}", self.metadata.id)
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Self> {
        vec![]
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        // TODO: Implement view rendering

        let indent = "  ".repeat(depth);
        let text = format!("{}{}", indent, self.metadata.name);

        Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        ))).build()
    }
}

/// Form node in the tree
#[derive(Clone)]
pub struct FormNode {
    pub metadata: FormMetadata,
}

impl TreeItem for FormNode {
    type Msg = super::Msg;

    fn id(&self) -> String {
        format!("form_{}", self.metadata.id)
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Self> {
        vec![]
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        // TODO: Implement form rendering

        let indent = "  ".repeat(depth);
        let text = format!("{}{}", indent, self.metadata.name);

        Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        ))).build()
    }
}
