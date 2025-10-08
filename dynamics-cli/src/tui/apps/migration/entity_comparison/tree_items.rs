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
    Entity(EntityNode),
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
            Self::Entity(node) => node.id(),
        }
    }

    fn has_children(&self) -> bool {
        match self {
            Self::Container(node) => !node.children.is_empty(),
            Self::Field(node) => node.has_children(),
            Self::Relationship(node) => node.has_children(),
            Self::View(node) => node.has_children(),
            Self::Form(node) => node.has_children(),
            Self::Entity(node) => node.has_children(),
        }
    }

    fn children(&self) -> Vec<Self> {
        match self {
            Self::Container(node) => node.children.clone(),
            Self::Field(node) => node.children().into_iter().map(Self::Field).collect(),
            Self::Relationship(node) => node.children().into_iter().map(Self::Relationship).collect(),
            Self::View(node) => node.children().into_iter().map(Self::View).collect(),
            Self::Form(node) => node.children().into_iter().map(Self::Form).collect(),
            Self::Entity(node) => node.children().into_iter().map(Self::Entity).collect(),
        }
    }

    fn to_element(
        &self,
        depth: usize,
        is_selected: bool,
        is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
        match self {
            Self::Container(node) => {
                let indent = "  ".repeat(depth);
                let mut spans = Vec::new();

                // Indent
                if depth > 0 {
                    spans.push(Span::styled(indent, Style::default()));
                }

                // Use stored container_match_type for color (keep color even when selected)
                let color = match node.container_match_type {
                    ContainerMatchType::FullMatch => theme.green,
                    ContainerMatchType::Mixed => theme.yellow,
                    ContainerMatchType::NoMatch => theme.red,
                };

                // Container label
                spans.push(Span::styled(
                    node.label.clone(),
                    Style::default().fg(color).bold(),
                ));

                // Show match info if container has a mapping
                if let Some(match_info) = &node.match_info {
                    spans.push(Span::styled(" → ", Style::default().fg(theme.overlay1)));

                    // Extract just the container name from target path
                    let target_display = match_info.target_field
                        .split('/')
                        .last()
                        .unwrap_or(&match_info.target_field)
                        .to_string();

                    spans.push(Span::styled(
                        target_display,
                        Style::default().fg(theme.blue),
                    ));

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
            Self::Field(node) => node.to_element(depth, is_selected, is_expanded),
            Self::Relationship(node) => node.to_element(depth, is_selected, is_expanded),
            Self::View(node) => node.to_element(depth, is_selected, is_expanded),
            Self::Form(node) => node.to_element(depth, is_selected, is_expanded),
            Self::Entity(node) => node.to_element(depth, is_selected, is_expanded),
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
    pub match_info: Option<MatchInfo>, // Match info if this container is manually/automatically mapped
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
    pub display_name: String, // Computed name to display (technical or friendly)
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
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
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
                MatchType::ExampleValue => theme.sky,   // Example value match
                MatchType::TypeMismatch => theme.yellow, // Name match but type differs
            }
        } else {
            theme.red  // No match
        };

        let field_name_style = Style::default().fg(field_name_color);

        // Use the pre-computed display name (which can be either technical/logical or user-friendly)
        spans.push(Span::styled(
            self.display_name.clone(),
            field_name_style,
        ));

        // Required indicator (red asterisk)
        if self.metadata.is_required {
            spans.push(Span::styled(" *", Style::default().fg(theme.red)));
        }

        // Mapping arrow and target field (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(" → ", Style::default().fg(theme.overlay1)));

            // Extract just the field name from target path
            let target_display = match_info.target_field
                .split('/')
                .last()
                .unwrap_or(&match_info.target_field)
                .to_string();

            spans.push(Span::styled(
                target_display,
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
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
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
                MatchType::ExampleValue => theme.sky,   // Example value match
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

        // Related entity and relationship type in angle brackets
        // Format: <entity [ManyToOne]> or <unknown [OneToMany]>
        let rel_type_label = match self.metadata.relationship_type {
            crate::api::metadata::RelationshipType::ManyToOne => "N:1",
            crate::api::metadata::RelationshipType::OneToMany => "1:N",
            crate::api::metadata::RelationshipType::ManyToMany => "N:N",
        };

        let entity_display = if self.metadata.related_entity == "unknown" || self.metadata.related_entity.is_empty() {
            format!(" <{}>", rel_type_label)
        } else {
            format!(" <{} {}>", self.metadata.related_entity, rel_type_label)
        };

        spans.push(Span::styled(
            entity_display,
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
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
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
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
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

/// Entity node in the tree (for entity type mapping)
#[derive(Clone)]
pub struct EntityNode {
    pub name: String,
    pub match_info: Option<MatchInfo>,
    pub usage_count: usize,
}

impl TreeItem for EntityNode {
    type Msg = super::Msg;

    fn id(&self) -> String {
        format!("entity_{}", self.name)
    }

    fn has_children(&self) -> bool {
        false
    }

    fn children(&self) -> Vec<Self> {
        vec![]
    }

    fn to_element(
        &self,
        depth: usize,
        is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
        let indent = "  ".repeat(depth);
        let mut spans = Vec::new();

        // Indent
        if depth > 0 {
            spans.push(Span::styled(indent, Style::default()));
        }

        // Entity name - colored by match state (keep color even when selected)
        let entity_name_color = if let Some(match_info) = &self.match_info {
            match match_info.match_type {
                MatchType::Exact => theme.green,        // Exact name match
                MatchType::Prefix => theme.green,       // Prefix name match
                MatchType::Manual => theme.green,       // User override
                MatchType::ExampleValue => theme.sky,   // Example value match
                MatchType::TypeMismatch => theme.yellow, // Should not happen for entities
            }
        } else {
            theme.red  // No match
        };

        let entity_name_style = Style::default().fg(entity_name_color);

        spans.push(Span::styled(
            self.name.clone(),
            entity_name_style,
        ));

        // Usage count
        spans.push(Span::styled(
            format!(" ({} uses)", self.usage_count),
            Style::default().fg(theme.overlay1),
        ));

        // Mapping arrow and target entity (if mapped)
        if let Some(match_info) = &self.match_info {
            spans.push(Span::styled(" → ", Style::default().fg(theme.overlay1)));
            spans.push(Span::styled(
                match_info.target_field.clone(),
                Style::default().fg(theme.blue),
            ));
        }

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
