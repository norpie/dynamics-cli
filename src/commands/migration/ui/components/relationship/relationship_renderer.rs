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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_field_info(name: &str, field_type: &str) -> FieldInfo {
        FieldInfo {
            name: name.to_string(),
            field_type: field_type.to_string(),
            is_required: false,
            is_custom: false,
        }
    }

    #[test]
    fn test_relationship_type_parsing() {
        assert_eq!(
            RelationshipType::from_field_type("1:N → Contact"),
            Some(RelationshipType::OneToMany)
        );

        assert_eq!(
            RelationshipType::from_field_type("N:1 → Account"),
            Some(RelationshipType::ManyToOne)
        );

        assert_eq!(
            RelationshipType::from_field_type("nav → User"),
            Some(RelationshipType::ManyToOne)
        );

        assert_eq!(RelationshipType::from_field_type("string"), None);
    }

    #[test]
    fn test_relationship_field_creation() {
        let field_info = create_test_field_info("primarycontactid", "N:1 → Contact");
        let rel_field = RelationshipField::from_field_info(field_info);

        assert!(rel_field.is_some());
        let rel_field = rel_field.unwrap();
        assert_eq!(rel_field.target_entity, "Contact");
        assert_eq!(rel_field.relationship_type, RelationshipType::ManyToOne);
    }

    #[test]
    fn test_relationship_extraction() {
        let fields = vec![
            create_test_field_info("accountname", "string"),
            create_test_field_info("primarycontactid", "N:1 → Contact"),
            create_test_field_info("revenue", "money"),
            create_test_field_info("contacts", "1:N → Contact"),
        ];

        let relationship_fields = RelationshipRenderer::extract_relationship_fields(&fields);
        assert_eq!(relationship_fields.len(), 2);

        let regular_fields = RelationshipRenderer::filter_out_relationships(&fields);
        assert_eq!(regular_fields.len(), 2);
        assert_eq!(regular_fields[0].name, "accountname");
        assert_eq!(regular_fields[1].name, "revenue");
    }

    #[test]
    fn test_extract_target_entity() {
        // Test normal case with Unicode arrow
        assert_eq!(
            RelationshipField::extract_target_entity("N:1 → Contact"),
            "Contact"
        );
        assert_eq!(
            RelationshipField::extract_target_entity(
                "1:N → new_bpf_fd08e4482f51463fbcf966f706a5a983"
            ),
            "new_bpf_fd08e4482f51463fbcf966f706a5a983"
        );

        // Test edge cases
        assert_eq!(
            RelationshipField::extract_target_entity("no arrow here"),
            "Unknown"
        );
        assert_eq!(RelationshipField::extract_target_entity("N:1 → "), "");
        assert_eq!(
            RelationshipField::extract_target_entity("N:1 →   Account   "),
            "Account"
        );
    }

    #[test]
    fn test_relationship_grouping() {
        let fields = vec![
            RelationshipField::from_field_info(create_test_field_info(
                "primarycontactid",
                "N:1 → Contact",
            ))
            .unwrap(),
            RelationshipField::from_field_info(create_test_field_info("contacts", "1:N → Contact"))
                .unwrap(),
            RelationshipField::from_field_info(create_test_field_info("ownerid", "N:1 → User"))
                .unwrap(),
        ];

        let groups = RelationshipGroup::create_groups(fields);
        assert_eq!(groups.len(), 2); // One group for 1:N, one for N:1

        let one_to_many = groups
            .iter()
            .find(|g| g.relationship_type == RelationshipType::OneToMany)
            .unwrap();
        assert_eq!(one_to_many.fields.len(), 1);

        let many_to_one = groups
            .iter()
            .find(|g| g.relationship_type == RelationshipType::ManyToOne)
            .unwrap();
        assert_eq!(many_to_one.fields.len(), 2);
    }
}
