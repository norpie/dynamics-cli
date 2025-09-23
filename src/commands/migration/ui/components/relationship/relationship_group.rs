use super::{RelationshipField, RelationshipType};
use crate::commands::migration::ui::components::hierarchy_tree::HierarchyNode;

/// Represents a group of relationships by type
#[derive(Debug, Clone)]
pub struct RelationshipGroup {
    pub relationship_type: RelationshipType,
    pub fields: Vec<RelationshipField>,
}

impl RelationshipGroup {
    pub fn new(relationship_type: RelationshipType) -> Self {
        Self {
            relationship_type,
            fields: Vec::new(),
        }
    }

    pub fn add_field(&mut self, field: RelationshipField) {
        self.fields.push(field);
    }

    /// Check if this group has any fields
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Create groups from a list of relationship fields
    pub fn create_groups(fields: Vec<RelationshipField>) -> Vec<RelationshipGroup> {
        let mut groups = vec![
            RelationshipGroup::new(RelationshipType::OneToMany),
            RelationshipGroup::new(RelationshipType::ManyToOne),
            RelationshipGroup::new(RelationshipType::ManyToMany),
            RelationshipGroup::new(RelationshipType::OneToOne),
        ];

        for field in fields {
            if let Some(group) = groups
                .iter_mut()
                .find(|g| g.relationship_type == field.relationship_type)
            {
                group.add_field(field);
            }
        }

        // Remove empty groups
        groups.retain(|g| !g.is_empty());
        groups
    }

    /// Create a special RelationshipGroup that represents a single field
    /// This is used for leaf nodes in the hierarchy to avoid key collisions
    pub fn new_field_node(field: RelationshipField) -> Self {
        Self {
            relationship_type: field.relationship_type.clone(),
            fields: vec![field],
        }
    }
}

impl HierarchyNode for RelationshipGroup {
    fn display_name(&self) -> String {
        if self.fields.len() == 1 {
            // This is a single-field node, show the field name instead of the group type
            self.fields[0].field_info.name.clone()
        } else {
            // This is a multi-field group, show the relationship type
            self.relationship_type.display_name().to_string()
        }
    }

    fn clean_name(&self) -> &str {
        if self.fields.len() == 1 {
            &self.fields[0].field_info.name
        } else {
            self.relationship_type.display_name()
        }
    }

    fn item_count(&self) -> usize {
        self.fields.len()
    }

    fn is_expandable(&self) -> bool {
        // Single-field nodes are not expandable (they are leaves)
        // Multi-field groups are expandable
        self.fields.len() > 1
    }

    fn mapping_target(&self) -> Option<String> {
        if self.fields.len() == 1 {
            // Single-field nodes can show mapping info from the field
            self.fields[0].mapping_target.clone()
        } else {
            // Groups don't have mappings
            None
        }
    }

    fn node_key(&self) -> String {
        if self.fields.len() == 1 {
            // Single-field nodes use the field name for unique keys
            format!("field_{}", self.fields[0].field_info.name)
        } else {
            // Multi-field groups use the relationship type
            format!("group_{}", self.relationship_type.short_name())
        }
    }
}
