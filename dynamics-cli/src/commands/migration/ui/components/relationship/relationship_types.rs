/// Represents different types of relationships in Dynamics 365
#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipType {
    /// One-to-Many relationship (1:N)
    OneToMany,
    /// Many-to-One relationship (N:1) - typically lookup fields
    ManyToOne,
    /// Many-to-Many relationship (N:N) - junction tables
    ManyToMany,
    /// One-to-One relationship (1:1) - rare
    OneToOne,
}

impl RelationshipType {
    /// Parse relationship type from field type string
    pub fn from_field_type(field_type: &str) -> Option<Self> {
        if field_type.starts_with("1:N") {
            Some(RelationshipType::OneToMany)
        } else if field_type.starts_with("N:1") || field_type.starts_with("nav") {
            Some(RelationshipType::ManyToOne)
        } else if field_type.starts_with("N:N") {
            Some(RelationshipType::ManyToMany)
        } else if field_type.starts_with("1:1") {
            Some(RelationshipType::OneToOne)
        } else {
            None
        }
    }

    /// Get display string for this relationship type
    pub fn display_name(&self) -> &'static str {
        match self {
            RelationshipType::OneToMany => "Lookup (1:Many)",
            RelationshipType::ManyToOne => "Reference (Many:1)",
            RelationshipType::ManyToMany => "Junction (Many:Many)",
            RelationshipType::OneToOne => "Direct (1:1)",
        }
    }

    /// Get short display string for relationship type
    pub fn short_name(&self) -> &'static str {
        match self {
            RelationshipType::OneToMany => "1:Many",
            RelationshipType::ManyToOne => "Many:1",
            RelationshipType::ManyToMany => "Many:Many",
            RelationshipType::OneToOne => "1:1",
        }
    }
}
