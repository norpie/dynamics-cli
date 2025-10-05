//! TreeItem implementations for entity comparison

use crate::tui::{Element, Theme, widgets::TreeItem};
use crate::api::{FieldMetadata, RelationshipMetadata, ViewMetadata, FormMetadata};
use ratatui::{style::Style, text::{Line, Span}, prelude::Stylize};
use super::models::{MatchInfo, MatchType};

/// Unified tree item that can represent any metadata type
#[derive(Clone)]
pub enum ComparisonTreeItem {
    Container(ContainerNode),
    Field(FieldNode),
    Relationship(RelationshipNode),
    View(ViewNode),
    Form(FormNode),
}

impl TreeItem for ComparisonTreeItem {
    type Msg = super::Msg;

    fn id(&self) -> String {
        match self {
            Self::Container(node) => node.id.clone(),
            Self::Field(node) => node.id(),
            Self::Relationship(node) => node.id(),
            Self::View(node) => node.id(),
            Self::Form(node) => node.id(),
        }
    }

    fn has_children(&self) -> bool {
        match self {
            Self::Container(node) => !node.children.is_empty(),
            Self::Field(node) => node.has_children(),
            Self::Relationship(node) => node.has_children(),
            Self::View(node) => node.has_children(),
            Self::Form(node) => node.has_children(),
        }
    }

    fn children(&self) -> Vec<Self> {
        match self {
            Self::Container(node) => node.children.clone(),
            Self::Field(node) => node.children().into_iter().map(Self::Field).collect(),
            Self::Relationship(node) => node.children().into_iter().map(Self::Relationship).collect(),
            Self::View(node) => node.children().into_iter().map(Self::View).collect(),
            Self::Form(node) => node.children().into_iter().map(Self::Form).collect(),
        }
    }

    fn to_element(
        &self,
        theme: &Theme,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Element<Self::Msg> {
        match self {
            Self::Container(node) => {
                let indent = "  ".repeat(depth);
                let text = format!("{}{}", indent, node.label);

                // Use stored container_match_type for color (keep color even when selected)
                let color = match node.container_match_type {
                    ContainerMatchType::FullMatch => theme.green,
                    ContainerMatchType::Mixed => theme.yellow,
                    ContainerMatchType::NoMatch => theme.red,
                };

                let mut builder = Element::styled_text(Line::from(Span::styled(
                    text,
                    Style::default().fg(color).bold(),
                )));

                if is_selected {
                    builder = builder.background(Style::default().bg(theme.surface0));
                }

                builder.build()
            }
            Self::Field(node) => node.to_element(theme, depth, is_selected, is_expanded),
            Self::Relationship(node) => node.to_element(theme, depth, is_selected, is_expanded),
            Self::View(node) => node.to_element(theme, depth, is_selected, is_expanded),
            Self::Form(node) => node.to_element(theme, depth, is_selected, is_expanded),
        }
    }
}

/// Generic container node (for FormType, Form, Tab, Section, ViewType, View, etc.)
#[derive(Clone)]
pub struct ContainerNode {
    pub id: String,
    pub label: String,
    pub children: Vec<ComparisonTreeItem>,
    pub container_match_type: ContainerMatchType, // Unmapped, FullMatch, or Mixed
}

/// Container match type (aggregated from children)
#[derive(Clone, Debug, PartialEq)]
pub enum ContainerMatchType {
    NoMatch,    // Container not matched
    FullMatch,  // Container matched AND all children matched
    Mixed,      // Container matched BUT not all children matched
}

/// Truncate a value string to a maximum length for display
fn truncate_value(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len.saturating_sub(3)])
    }
}

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
        let indent = "  ".repeat(depth);
        let mut spans = Vec::new();

        // Indent
        if depth > 0 {
            spans.push(Span::styled(indent, Style::default()));
        }

        // Field name - colored by match state (keep color even when selected)
        let field_name_color = if let Some(match_info) = &self.match_info {
            match match_info.match_type {
                MatchType::Exact => theme.green,        // Exact name + type match
                MatchType::Prefix => theme.green,       // Prefix name + type match
                MatchType::Manual => theme.green,       // User override
                MatchType::TypeMismatch => theme.yellow, // Name match but type differs
            }
        } else {
            theme.red  // No match
        };

        let field_name_style = Style::default().fg(field_name_color);

        spans.push(Span::styled(
            self.metadata.logical_name.clone(),
            field_name_style,
        ));

        // Required indicator (red asterisk)
        if self.metadata.is_required {
            spans.push(Span::styled(" *", Style::default().fg(theme.red)));
        }

        // Mapping arrow and target field (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(" → ", Style::default().fg(theme.overlay1)));
            spans.push(Span::styled(
                match_info.target_field.clone(),
                Style::default().fg(theme.blue),
            ));
        }

        // Field type in angle brackets
        spans.push(Span::styled(
            format!(" <{:?}>", self.metadata.field_type),
            Style::default().fg(theme.overlay1),
        ));

        // Mapping source badge (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(
                format!(" {}", match_info.match_type.label()),
                Style::default().fg(theme.overlay1),
            ));
        }

        // Example value (if present)
        if let Some(example) = &self.example_value {
            spans.push(Span::styled(" | ", Style::default().fg(theme.overlay1)));
            spans.push(Span::styled(
                truncate_value(example, 60),
                Style::default().fg(theme.sky),
            ));
        }

        let mut builder = Element::styled_text(Line::from(spans));

        if is_selected {
            builder = builder.background(Style::default().bg(theme.surface0));
        }

        builder.build()
    }
}

/// Relationship node in the tree
#[derive(Clone)]
pub struct RelationshipNode {
    pub metadata: RelationshipMetadata,
    pub match_info: Option<MatchInfo>,
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
        let indent = "  ".repeat(depth);
        let mut spans = Vec::new();

        // Indent
        if depth > 0 {
            spans.push(Span::styled(indent, Style::default()));
        }

        // Relationship name - colored by match state
        let rel_name_color = if let Some(match_info) = &self.match_info {
            match match_info.match_type {
                MatchType::Exact => theme.green,        // Exact name + type match
                MatchType::Prefix => theme.green,       // Prefix name + type match
                MatchType::Manual => theme.green,       // User override
                MatchType::TypeMismatch => theme.yellow, // Name match but type differs
            }
        } else {
            theme.red  // No match
        };

        spans.push(Span::styled(
            self.metadata.name.clone(),
            Style::default().fg(rel_name_color),
        ));

        // Mapping arrow and target relationship (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(" → ", Style::default().fg(theme.overlay1)));
            spans.push(Span::styled(
                match_info.target_field.clone(),
                Style::default().fg(theme.blue),
            ));
        }

        // Related entity in angle brackets
        spans.push(Span::styled(
            format!(" <{}>", self.metadata.related_entity),
            Style::default().fg(theme.overlay1),
        ));

        // Mapping source badge (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(
                format!(" {}", match_info.match_type.label()),
                Style::default().fg(theme.overlay1),
            ));
        }

        let mut builder = Element::styled_text(Line::from(spans));

        if is_selected {
            builder = builder.background(Style::default().bg(theme.surface0));
        }

        builder.build()
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

        let mut builder = Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        )));

        if is_selected {
            builder = builder.background(Style::default().bg(theme.surface0));
        }

        builder.build()
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

        let mut builder = Element::styled_text(Line::from(Span::styled(
            text,
            if is_selected {
                Style::default().fg(theme.lavender)
            } else {
                Style::default().fg(theme.text)
            },
        )));

        if is_selected {
            builder = builder.background(Style::default().bg(theme.surface0));
        }

        builder.build()
    }
}
