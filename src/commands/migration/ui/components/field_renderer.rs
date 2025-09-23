use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::{
    commands::migration::ui::components::relationship_renderer::RelationshipType,
    dynamics::metadata::FieldInfo,
};

/// Truncate a value string to a maximum length for display
fn truncate_value(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        value.to_string()
    } else {
        format!("{}...", &value[..max_len.saturating_sub(3)])
    }
}

#[derive(Debug, Clone)]
pub enum MappingSource {
    Exact,
    Prefix,
    Manual,
}

#[derive(Debug, Clone)]
pub enum MatchState {
    FullMatch,    // Green - field and type match
    TypeMismatch, // Yellow - field matches but type differs
    MixedMatch,   // Yellow - hierarchical node with mixed child matches
    NoMatch,      // Red - no match found yet
}

impl MappingSource {
    pub fn as_str(&self) -> &str {
        match self {
            MappingSource::Exact => "exact",
            MappingSource::Prefix => "prefix",
            MappingSource::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub mapped_field_name: String,
    pub mapping_source: MappingSource,
    pub match_state: MatchState,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    Regular {
        mapping: Option<FieldMapping>,
    },
    Relationship {
        target_entity: String,
        relationship_type: RelationshipType,
        mapping: Option<FieldMapping>,
    },
}

#[derive(Debug, Clone)]
pub struct FieldDisplayInfo {
    pub field: FieldInfo,
    pub field_type: FieldType,
    pub match_state: MatchState,
    pub source_example_value: Option<String>,
    pub target_example_value: Option<String>,
}

impl FieldDisplayInfo {
    /// Create a new regular field with no mapping
    pub fn new(field: FieldInfo) -> Self {
        Self {
            field,
            field_type: FieldType::Regular { mapping: None },
            match_state: MatchState::NoMatch,
            source_example_value: None,
            target_example_value: None,
        }
    }

    /// Create a regular field with mapping
    pub fn with_mapping(field: FieldInfo, mapping: FieldMapping) -> Self {
        let match_state = mapping.match_state.clone();
        Self {
            field,
            field_type: FieldType::Regular {
                mapping: Some(mapping),
            },
            match_state,
            source_example_value: None,
            target_example_value: None,
        }
    }

    /// Create a relationship field with no mapping
    pub fn new_relationship(
        field: FieldInfo,
        target_entity: String,
        relationship_type: RelationshipType,
    ) -> Self {
        Self {
            field,
            field_type: FieldType::Relationship {
                target_entity,
                relationship_type,
                mapping: None,
            },
            match_state: MatchState::NoMatch,
            source_example_value: None,
            target_example_value: None,
        }
    }

    /// Create a relationship field with mapping
    pub fn relationship_with_mapping(
        field: FieldInfo,
        target_entity: String,
        relationship_type: RelationshipType,
        mapping: FieldMapping,
    ) -> Self {
        let match_state = mapping.match_state.clone();
        Self {
            field,
            field_type: FieldType::Relationship {
                target_entity,
                relationship_type,
                mapping: Some(mapping),
            },
            match_state,
            source_example_value: None,
            target_example_value: None,
        }
    }

    /// Get the mapping for this field, regardless of field type
    pub fn get_mapping(&self) -> Option<&FieldMapping> {
        match &self.field_type {
            FieldType::Regular { mapping } => mapping.as_ref(),
            FieldType::Relationship { mapping, .. } => mapping.as_ref(),
        }
    }

    /// Set example values for this field
    pub fn with_example_values(mut self, source_value: Option<String>, target_value: Option<String>) -> Self {
        self.source_example_value = source_value;
        self.target_example_value = target_value;
        self
    }

    /// Check if this field has example values to display
    pub fn has_example_values(&self) -> bool {
        self.source_example_value.is_some() || self.target_example_value.is_some()
    }
}

pub struct FieldRenderer;

impl FieldRenderer {
    /// Renders any field type with unified styling and consistent match state visualization
    ///
    /// Formats:
    /// Regular:      field_name * → mapped_field <text> [exact]
    /// Relationship: field_name * → target_entity <lookup> [manual]
    ///
    /// Colors: field name = match state (green/yellow/red), required * = red, arrow = gray,
    ///         target = blue, type/relationship = gray, mapping source = gray
    pub fn render_field_line(field_info: &FieldDisplayInfo) -> Line<'static> {
        let mut spans = vec![];

        // Field name - ALWAYS colored by match state for consistency
        let field_name_style = match field_info.match_state {
            MatchState::FullMatch => Style::default().fg(Color::Green),
            MatchState::TypeMismatch => Style::default().fg(Color::Yellow),
            MatchState::MixedMatch => Style::default().fg(Color::Yellow),
            MatchState::NoMatch => Style::default().fg(Color::Red),
        };
        spans.push(Span::styled(
            field_info.field.name.clone(),
            field_name_style,
        ));

        // Required indicator - consistent for all field types
        if field_info.field.is_required {
            spans.push(Span::styled(" *", Style::default().fg(Color::Red)));
        }

        // Handle field type-specific rendering
        match &field_info.field_type {
            FieldType::Regular { mapping } => {
                if let Some(mapping) = mapping {
                    // Arrow and mapped field name
                    spans.push(Span::styled(" → ", Style::default().fg(Color::Gray)));
                    spans.push(Span::styled(
                        mapping.mapped_field_name.clone(),
                        Style::default().fg(Color::Blue),
                    ));

                    // Field type in angle brackets
                    spans.push(Span::styled(
                        format!(" <{}>", field_info.field.field_type),
                        Style::default().fg(Color::Gray),
                    ));

                    // Mapping source
                    spans.push(Span::styled(
                        format!(" [{}]", mapping.mapping_source.as_str()),
                        Style::default().fg(Color::Gray),
                    ));
                } else {
                    // No mapping - show field type
                    spans.push(Span::styled(
                        format!(" <{}>", field_info.field.field_type),
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
            FieldType::Relationship {
                target_entity,
                relationship_type,
                mapping,
            } => {
                // Arrow and target entity - consistent with regular fields
                spans.push(Span::styled(" → ", Style::default().fg(Color::Gray)));

                // Target entity (could be mapped entity name if mapping exists)
                let target_name = if let Some(mapping) = mapping {
                    mapping.mapped_field_name.clone()
                } else {
                    target_entity.clone()
                };
                spans.push(Span::styled(target_name, Style::default().fg(Color::Blue)));

                // Relationship type in angle brackets - consistent with field types
                spans.push(Span::styled(
                    format!(" <{}>", relationship_type.short_name()),
                    Style::default().fg(Color::Gray),
                ));

                // Mapping source if available
                if let Some(mapping) = mapping {
                    spans.push(Span::styled(
                        format!(" [{}]", mapping.mapping_source.as_str()),
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
        }

        // Add example values if available
        if field_info.has_example_values() {
            spans.push(Span::styled(" | ", Style::default().fg(Color::Gray)));

            if let Some(source_value) = &field_info.source_example_value {
                spans.push(Span::styled("Source: ", Style::default().fg(Color::Gray)));
                spans.push(Span::styled(
                    truncate_value(source_value, 20),
                    Style::default().fg(Color::Cyan),
                ));
            }

            if field_info.source_example_value.is_some() && field_info.target_example_value.is_some() {
                spans.push(Span::styled(" | ", Style::default().fg(Color::Gray)));
            }

            if let Some(target_value) = &field_info.target_example_value {
                spans.push(Span::styled("Target: ", Style::default().fg(Color::Gray)));
                spans.push(Span::styled(
                    truncate_value(target_value, 20),
                    Style::default().fg(Color::Cyan),
                ));
            }
        }

        Line::from(spans)
    }

    /// Renders a field as a simple string for list components that expect String items
    pub fn render_field_string(field_info: &FieldDisplayInfo) -> String {
        let mut result = field_info.field.name.clone();

        // Add required indicator
        if field_info.field.is_required {
            result.push_str(" *");
        }

        // Handle field type-specific rendering
        match &field_info.field_type {
            FieldType::Regular { mapping } => {
                if let Some(mapping) = mapping {
                    result.push_str(&format!(" → {}", mapping.mapped_field_name));
                    result.push_str(&format!(" <{}>", field_info.field.field_type));
                    result.push_str(&format!(" [{}]", mapping.mapping_source.as_str()));
                } else {
                    result.push_str(&format!(" <{}>", field_info.field.field_type));
                }
            }
            FieldType::Relationship {
                target_entity,
                relationship_type,
                mapping,
            } => {
                let target_name = if let Some(mapping) = mapping {
                    mapping.mapped_field_name.clone()
                } else {
                    target_entity.clone()
                };
                result.push_str(&format!(" → {}", target_name));
                result.push_str(&format!(" <{}>", relationship_type.short_name()));
                if let Some(mapping) = mapping {
                    result.push_str(&format!(" [{}]", mapping.mapping_source.as_str()));
                }
            }
        }

        // Add example values if available
        if field_info.has_example_values() {
            result.push_str(" | ");

            if let Some(source_value) = &field_info.source_example_value {
                result.push_str(&format!("Source: {}", truncate_value(source_value, 20)));
            }

            if field_info.source_example_value.is_some() && field_info.target_example_value.is_some() {
                result.push_str(" | ");
            }

            if let Some(target_value) = &field_info.target_example_value {
                result.push_str(&format!("Target: {}", truncate_value(target_value, 20)));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_display_without_mapping() {
        let field = FieldInfo {
            name: "accountname".to_string(),
            field_type: "nvarchar".to_string(),
            is_required: true,
            is_custom: false,
        };
        let field_info = FieldDisplayInfo::new(field);
        let result = FieldRenderer::render_field_string(&field_info);
        assert_eq!(result, "accountname * <nvarchar>");
    }

    #[test]
    fn test_field_display_with_exact_mapping() {
        let field = FieldInfo {
            name: "accountname".to_string(),
            field_type: "nvarchar".to_string(),
            is_required: true,
            is_custom: false,
        };
        let mapping = FieldMapping {
            mapped_field_name: "name".to_string(),
            mapping_source: MappingSource::Exact,
            match_state: MatchState::FullMatch,
        };
        let field_info = FieldDisplayInfo::with_mapping(field, mapping);
        let result = FieldRenderer::render_field_string(&field_info);
        assert_eq!(result, "accountname * → name <nvarchar> [exact]");
    }

    #[test]
    fn test_custom_field_display() {
        let field = FieldInfo {
            name: "new_customfield".to_string(),
            field_type: "nvarchar".to_string(),
            is_required: false,
            is_custom: true,
        };
        let field_info = FieldDisplayInfo::new(field);
        let result = FieldRenderer::render_field_string(&field_info);
        assert_eq!(result, "new_customfield <nvarchar>");
    }

    #[test]
    fn test_field_display_with_example_values() {
        let field = FieldInfo {
            name: "accountname".to_string(),
            field_type: "nvarchar".to_string(),
            is_required: true,
            is_custom: false,
        };
        let field_info = FieldDisplayInfo::new(field)
            .with_example_values(Some("Acme Corp".to_string()), Some("Acme Corporation".to_string()));
        let result = FieldRenderer::render_field_string(&field_info);
        assert_eq!(result, "accountname * <nvarchar> | Source: Acme Corp | Target: Acme Corporation");
    }

    #[test]
    fn test_field_display_with_truncated_example_values() {
        let field = FieldInfo {
            name: "description".to_string(),
            field_type: "nvarchar".to_string(),
            is_required: false,
            is_custom: false,
        };
        let long_value = "This is a very long description that should be truncated";
        let field_info = FieldDisplayInfo::new(field)
            .with_example_values(Some(long_value.to_string()), None);
        let result = FieldRenderer::render_field_string(&field_info);
        assert_eq!(result, "description <nvarchar> | Source: This is a very lon...");
    }
}
