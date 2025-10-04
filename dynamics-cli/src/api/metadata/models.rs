//! Dynamics 365 metadata models

use serde::{Deserialize, Serialize};

/// Dynamics 365 field metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMetadata {
    pub logical_name: String,
    pub display_name: Option<String>,
    pub field_type: FieldType,
    pub is_required: bool,
    pub is_primary_key: bool,
    pub max_length: Option<i32>,
    pub related_entity: Option<String>, // For lookups
}

/// Field data types in Dynamics 365
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Integer,
    Decimal,
    Boolean,
    DateTime,
    Lookup,
    OptionSet,
    Money,
    Memo,
    UniqueIdentifier,
    Other(String),
}

/// Relationship metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipMetadata {
    pub name: String,
    pub relationship_type: RelationshipType,
    pub related_entity: String,
    pub related_attribute: String,
}

/// Relationship types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToMany,
    ManyToOne,
    ManyToMany,
}

/// View metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewMetadata {
    pub id: String,
    pub name: String,
    pub view_type: String,
    pub columns: Vec<String>,
}

/// Form metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormMetadata {
    pub id: String,
    pub name: String,
    pub form_type: String,
}

/// Complete entity metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub fields: Vec<FieldMetadata>,
    pub relationships: Vec<RelationshipMetadata>,
    pub views: Vec<ViewMetadata>,
    pub forms: Vec<FormMetadata>,
}

impl Default for EntityMetadata {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            relationships: Vec::new(),
            views: Vec::new(),
            forms: Vec::new(),
        }
    }
}
