use super::RelationshipType;
use crate::{
    commands::migration::ui::components::{
        field_renderer::{MappingSource, MatchState},
        hierarchy_tree::{FieldRenderingInfo, HierarchyNode},
    },
    dynamics::metadata::FieldInfo,
};

/// Represents a relationship field with metadata
#[derive(Debug, Clone)]
pub struct RelationshipField {
    pub field_info: FieldInfo,
    pub relationship_type: RelationshipType,
    pub target_entity: String,
    pub mapping_target: Option<String>,
}

impl RelationshipField {
    pub fn from_field_info(field_info: FieldInfo) -> Option<Self> {
        // Check for explicit relationship type strings (e.g., "N:1 → Contact")
        if let Some(rel_type) = RelationshipType::from_field_type(&field_info.field_type) {
            let target_entity = Self::extract_target_entity(&field_info.field_type);
            Some(RelationshipField {
                field_info,
                relationship_type: rel_type,
                target_entity,
                mapping_target: None,
            })
        }
        // Check for Dynamics 365 lookup fields (end with _value and type Edm.Guid)
        else if field_info.name.ends_with("_value") && field_info.field_type == "Edm.Guid" {
            let target_entity = Self::extract_target_entity_from_lookup_name(&field_info.name);
            Some(RelationshipField {
                field_info,
                relationship_type: RelationshipType::ManyToOne, // Lookup fields are Many:1
                target_entity,
                mapping_target: None,
            })
        } else {
            None
        }
    }

    pub fn with_mapping(mut self, mapping_target: String) -> Self {
        self.mapping_target = Some(mapping_target);
        self
    }

    /// Extract the target entity name from the field type string
    pub fn extract_target_entity(field_type: &str) -> String {
        if let Some(arrow_pos) = field_type.find(" → ") {
            // Use split_at to properly handle Unicode boundaries
            let (_, after_arrow) = field_type.split_at(arrow_pos);
            // Remove " → " (which is 3 Unicode characters, not bytes)
            after_arrow
                .strip_prefix(" → ")
                .unwrap_or("Unknown")
                .trim()
                .to_string()
        } else {
            "Unknown".to_string()
        }
    }

    /// Extract target entity from lookup field names like "_cgk_presidentid_value"
    fn extract_target_entity_from_lookup_name(field_name: &str) -> String {
        // Remove the "_value" suffix first
        let name_without_suffix = field_name.strip_suffix("_value").unwrap_or(field_name);

        // Handle different patterns:
        if name_without_suffix.ends_with("id") {
            // Remove "id" suffix: "_cgk_presidentid" → "_cgk_president"
            let without_id = name_without_suffix
                .strip_suffix("id")
                .unwrap_or(name_without_suffix);
            // Extract the entity name (last part after underscores)
            if let Some(last_underscore) = without_id.rfind('_') {
                let entity_name = &without_id[(last_underscore + 1)..];
                // Capitalize first letter
                entity_name
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().collect::<String>() + &entity_name[1..])
                    .unwrap_or_else(|| entity_name.to_string())
            } else {
                without_id.to_string()
            }
        } else {
            // For fields like "_modifiedby_value", extract the part after last underscore
            if let Some(last_underscore) = name_without_suffix.rfind('_') {
                let entity_name = &name_without_suffix[(last_underscore + 1)..];
                // Map common system field names to their entities
                match entity_name {
                    "modifiedby" | "createdby" | "owninguser" => "SystemUser".to_string(),
                    "owningteam" => "Team".to_string(),
                    "ownerid" => "Principal".to_string(),
                    "owningbusinessunit" => "BusinessUnit".to_string(),
                    _ => entity_name
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().collect::<String>() + &entity_name[1..])
                        .unwrap_or_else(|| entity_name.to_string()),
                }
            } else {
                "Unknown".to_string()
            }
        }
    }
}

impl HierarchyNode for RelationshipField {
    fn display_name(&self) -> String {
        format!(
            "{} → {} <{}>",
            self.field_info.name,
            self.target_entity,
            self.relationship_type.short_name()
        )
    }

    fn clean_name(&self) -> &str {
        &self.field_info.name
    }

    fn item_count(&self) -> usize {
        0 // Individual fields don't have children
    }

    fn is_expandable(&self) -> bool {
        false
    }

    fn mapping_target(&self) -> Option<String> {
        self.mapping_target.clone()
    }

    fn node_key(&self) -> String {
        format!("field_{}", self.field_info.name)
    }

    /// RelationshipField nodes should use unified field rendering
    fn is_field_node(&self) -> bool {
        true
    }

    /// Provide field information for unified rendering
    fn get_field_info(&self) -> Option<FieldRenderingInfo> {
        Some(FieldRenderingInfo {
            field_name: self.field_info.name.clone(),
            field_type: format!(
                "{} → {}",
                self.relationship_type.short_name(),
                self.target_entity
            ),
            is_required: self.field_info.is_required,
            mapping_target: self.mapping_target.clone(),
            mapping_source: if self.mapping_target.is_some() {
                Some(MappingSource::Manual)
            } else {
                None
            },
            match_state: if self.mapping_target.is_some() {
                MatchState::FullMatch
            } else {
                MatchState::NoMatch
            },
        })
    }
}
