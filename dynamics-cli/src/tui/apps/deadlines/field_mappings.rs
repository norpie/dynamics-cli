/// Static field mappings for deadline Excel imports
///
/// This module defines the mapping from Excel column names to Dynamics fields
/// for both cgk_deadlines and nrq_deadlines entities.
///
/// Checkbox columns (after "Raad van Bestuur") are NOT defined here - they are
/// dynamically discovered from entity metadata.

use std::collections::HashMap;

/// Field type for mapping configuration
#[derive(Debug, Clone)]
pub enum FieldType {
    /// Direct field - simple value copy
    Direct,
    /// Lookup field - requires entity resolution via CSV cache
    Lookup { target_entity: String },
    /// Date field - requires parsing and timezone conversion
    Date,
    /// Time field - combined with date field for datetime
    Time,
    /// Checkbox field - N:N relationship (dynamically discovered)
    Checkbox,
}

/// Single field mapping from Excel column to Dynamics field
#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub excel_column: String,
    pub dynamics_field: String,
    pub field_type: FieldType,
}

/// Get field mappings for cgk_deadlines entity
pub fn get_cgk_mappings() -> Vec<FieldMapping> {
    vec![
        // Pillar (prefer "Pillar" over "Domein*" - Pillar is machine-readable)
        FieldMapping {
            excel_column: "Pillar".to_string(),
            dynamics_field: "cgk_pillarid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "cgk_pillar".to_string(),
            },
        },

        // Fund
        FieldMapping {
            excel_column: "Fonds*".to_string(),
            dynamics_field: "cgk_fundid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "cgk_fund".to_string(),
            },
        },

        // Deadline name
        FieldMapping {
            excel_column: "Deadline*".to_string(),
            dynamics_field: "cgk_name".to_string(),
            field_type: FieldType::Direct,
        },

        // Project manager - use email column (non-naam version)
        FieldMapping {
            excel_column: "Projectbeheerder".to_string(),
            dynamics_field: "ownerid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "systemuser".to_string(),
            },
        },

        // Info field
        FieldMapping {
            excel_column: "Info".to_string(),
            dynamics_field: "cgk_info".to_string(),
            field_type: FieldType::Direct,
        },

        // Deadline date
        FieldMapping {
            excel_column: "Datum Deadline".to_string(),
            dynamics_field: "cgk_date".to_string(),
            field_type: FieldType::Date,
        },

        // Commission meeting time (machine-readable time field)
        FieldMapping {
            excel_column: "Uur".to_string(),
            dynamics_field: "cgk_time".to_string(),
            field_type: FieldType::Time,
        },

        // Commission
        FieldMapping {
            excel_column: "Commissie".to_string(),
            dynamics_field: "cgk_commissionid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "cgk_commission".to_string(),
            },
        },

        // Commission meeting date
        FieldMapping {
            excel_column: "Datum Commissievergadering".to_string(),
            dynamics_field: "cgk_commissiondate".to_string(),
            field_type: FieldType::Date,
        },

        // Board meeting (Raad van Bestuur)
        FieldMapping {
            excel_column: "Raad van Bestuur".to_string(),
            dynamics_field: "cgk_boardmeetingid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "cgk_deadline".to_string(), // Self-referencing for CGK
            },
        },
    ]
}

/// Get field mappings for nrq_deadlines entity
pub fn get_nrq_mappings() -> Vec<FieldMapping> {
    vec![
        // Domain (prefer "Pillar" over "Domein*")
        FieldMapping {
            excel_column: "Pillar".to_string(),
            dynamics_field: "nrq_domainid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "nrq_domain".to_string(),
            },
        },

        // Fund
        FieldMapping {
            excel_column: "Fonds*".to_string(),
            dynamics_field: "nrq_fundid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "nrq_fund".to_string(),
            },
        },

        // Deadline name
        FieldMapping {
            excel_column: "Deadline*".to_string(),
            dynamics_field: "nrq_name".to_string(),
            field_type: FieldType::Direct,
        },

        // Project manager - use email column (non-naam version)
        FieldMapping {
            excel_column: "Projectbeheerder".to_string(),
            dynamics_field: "ownerid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "systemuser".to_string(),
            },
        },

        // Info field
        FieldMapping {
            excel_column: "Info".to_string(),
            dynamics_field: "nrq_info".to_string(),
            field_type: FieldType::Direct,
        },

        // Deadline date
        FieldMapping {
            excel_column: "Datum Deadline".to_string(),
            dynamics_field: "nrq_date".to_string(),
            field_type: FieldType::Date,
        },

        // Commission meeting time (machine-readable time field)
        FieldMapping {
            excel_column: "Uur".to_string(),
            dynamics_field: "nrq_time".to_string(),
            field_type: FieldType::Time,
        },

        // Commission
        FieldMapping {
            excel_column: "Commissie".to_string(),
            dynamics_field: "nrq_commissionid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "nrq_commission".to_string(),
            },
        },

        // Commission meeting date
        FieldMapping {
            excel_column: "Datum Commissievergadering".to_string(),
            dynamics_field: "nrq_commissiondate".to_string(),
            field_type: FieldType::Date,
        },

        // Board meeting (Raad van Bestuur) - separate entity for NRQ
        FieldMapping {
            excel_column: "Raad van Bestuur".to_string(),
            dynamics_field: "nrq_boardmeetingid".to_string(),
            field_type: FieldType::Lookup {
                target_entity: "nrq_boardmeeting".to_string(),
            },
        },
    ]
}

/// Build a lookup map from Excel column name to FieldMapping
pub fn build_mapping_lookup(mappings: Vec<FieldMapping>) -> HashMap<String, FieldMapping> {
    mappings
        .into_iter()
        .map(|m| (m.excel_column.clone(), m))
        .collect()
}

/// Detect which entity type based on environment entities
/// Returns either "cgk_deadlines" or "nrq_deadlines" if found, None otherwise
pub fn detect_deadline_entity(entities: &[String]) -> Option<String> {
    if entities.iter().any(|e| e == "cgk_deadlines") {
        Some("cgk_deadlines".to_string())
    } else if entities.iter().any(|e| e == "nrq_deadlines") {
        Some("nrq_deadlines".to_string())
    } else {
        None
    }
}

/// Get mappings based on detected entity type
pub fn get_mappings_for_entity(entity_name: &str) -> Vec<FieldMapping> {
    match entity_name {
        "cgk_deadlines" => get_cgk_mappings(),
        "nrq_deadlines" => get_nrq_mappings(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cgk_entity() {
        let entities = vec!["cgk_pillar".to_string(), "cgk_deadlines".to_string()];
        assert_eq!(detect_deadline_entity(&entities), Some("cgk_deadlines".to_string()));
    }

    #[test]
    fn test_detect_nrq_entity() {
        let entities = vec!["nrq_domain".to_string(), "nrq_deadlines".to_string()];
        assert_eq!(detect_deadline_entity(&entities), Some("nrq_deadlines".to_string()));
    }

    #[test]
    fn test_cgk_mappings_count() {
        let mappings = get_cgk_mappings();
        assert_eq!(mappings.len(), 10); // 10 non-checkbox fields
    }

    #[test]
    fn test_nrq_mappings_count() {
        let mappings = get_nrq_mappings();
        assert_eq!(mappings.len(), 10); // 10 non-checkbox fields
    }
}
