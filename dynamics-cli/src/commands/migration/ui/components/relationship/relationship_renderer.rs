use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::{RelationshipField, RelationshipType};
use crate::dynamics::metadata::FieldInfo;

/// Utility functions for rendering relationship information
pub struct RelationshipRenderer;

impl RelationshipRenderer {
    /// Create a styled line for a relationship field
    pub fn render_relationship_field(field: &RelationshipField) -> Line<'static> {
        let mut spans = Vec::new();

        // Field name
        spans.push(Span::styled(
            field.field_info.name.clone(),
            Style::default().fg(Color::White),
        ));

        // Required indicator
        if field.field_info.is_required {
            spans.push(Span::styled(" *", Style::default().fg(Color::Red)));
        }

        // Arrow and target entity
        spans.push(Span::styled(" → ", Style::default().fg(Color::Gray)));
        spans.push(Span::styled(
            field.target_entity.clone(),
            Style::default().fg(Color::Blue),
        ));

        // Relationship type in angle brackets
        spans.push(Span::styled(
            format!(" <{}>", field.relationship_type.short_name()),
            Style::default().fg(Color::Gray),
        ));

        Line::from(spans)
    }

    /// Extract relationship fields from a list of all fields
    pub fn extract_relationship_fields(fields: &[FieldInfo]) -> Vec<RelationshipField> {
        fields
            .iter()
            .filter_map(|field| RelationshipField::from_field_info(field.clone()))
            .collect()
    }

    /// Filter out relationship fields from a list, leaving only regular fields
    pub fn filter_out_relationships(fields: &[FieldInfo]) -> Vec<FieldInfo> {
        fields
            .iter()
            .filter(|field| RelationshipType::from_field_type(&field.field_type).is_none())
            .cloned()
            .collect()
    }
}

