use anyhow::Result;
use std::collections::HashMap;
use log::debug;

use super::config::EnvironmentConfig;
use super::excel_parser::SheetData;
use super::field_mapping_tui::{FieldMapping, FieldType};
use super::validation::{ValidationResult, identify_checkbox_columns};

/// Create hardcoded field mappings based on Python transformation logic
/// This replaces the complex field mapping TUI with business-rule-based mapping
pub fn create_hardcoded_field_mappings(
    sheet_data: &SheetData,
    env_config: &EnvironmentConfig,
    validation_result: &ValidationResult,
) -> Result<HashMap<String, FieldMapping>> {
    debug!("Creating hardcoded field mappings for {} columns", sheet_data.headers.len());

    let mut field_mappings = HashMap::new();

    // Get checkbox columns (those that might be X-marked for N:N relationships)
    let checkbox_columns = identify_checkbox_columns(&sheet_data.headers);
    debug!("Identified {} checkbox columns", checkbox_columns.len());

    for header in &sheet_data.headers {
        // Skip unmatched columns from validation
        if validation_result.unmatched_columns.contains(header) {
            debug!("Skipping unmatched column: {}", header);
            continue;
        }

        let header_lower = header.to_lowercase();
        let mapping = match header_lower.as_str() {
            // Core deadline fields
            h if h.contains("deadline") => Some(create_deadline_name_mapping(header)),

            // Date/Time fields (check before commission to handle "Datum Commissie")
            h if h.contains("datum") || h.contains("date") => Some(create_date_field_mapping(header)),
            h if h.contains("tijd") || h.contains("time") => Some(create_time_field_mapping(header)),

            // Entity lookups
            h if h.contains("domein") || h.contains("domain") => Some(create_domain_lookup_mapping(header, env_config)),
            h if h.contains("fonds") || h.contains("fund") => Some(create_fund_lookup_mapping(header, env_config)),
            h if h.contains("commissie") || h.contains("commission") => Some(create_commission_lookup_mapping(header, env_config)),

            // Project manager / User fields
            h if h.contains("projectbeheerder") || h.contains("project") => Some(create_user_lookup_mapping(header)),

            // Information fields
            h if h.contains("informatie") || h.contains("information") || h.contains("info") => Some(create_info_field_mapping(header)),

            // Board meeting field (environment-aware) - support both "raad van bestuur" and "rvb"
            h if (h.contains("raad") && h.contains("bestuur")) || h.contains("rvb") => Some(create_board_meeting_mapping(header, env_config)),

            // N:N relationship checkbox columns (X-marked columns)
            _ if checkbox_columns.contains(header) => {
                // Check if this column was matched during validation
                if let Some(entity_match) = validation_result.matched_entities.iter()
                    .find(|m| m.column_header == *header) {
                    Some(create_multiselect_mapping(header, &entity_match.logical_type, env_config))
                } else {
                    debug!("Checkbox column '{}' not matched during validation, ignoring", header);
                    None
                }
            },

            // Skip other columns (e.g., ID columns, unknown columns)
            _ => {
                debug!("No mapping rule for column: {}", header);
                None
            }
        };

        if let Some(field_mapping) = mapping {
            debug!("Created mapping: {} -> {}", header, field_mapping.target_field);
            field_mappings.insert(header.clone(), field_mapping);
        }
    }

    debug!("Created {} field mappings total", field_mappings.len());
    Ok(field_mappings)
}

/// Create mapping for deadline name field
fn create_deadline_name_mapping(excel_column: &str) -> FieldMapping {
    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: "cgk_name".to_string(), // This will be environment-specific
        field_type: FieldType::DirectField,
        target_entity: None,
        junction_entity: None,
    }
}

/// Create mapping for domain/pillar lookup field
fn create_domain_lookup_mapping(excel_column: &str, env_config: &EnvironmentConfig) -> FieldMapping {
    let target_field = if env_config.prefix.starts_with("nrq") {
        "nrq_domainid".to_string()
    } else {
        "cgk_pillarid".to_string()
    };

    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field,
        field_type: FieldType::LookupField,
        target_entity: Some("pillar".to_string()),
        junction_entity: None,
    }
}

/// Create mapping for fund lookup field
fn create_fund_lookup_mapping(excel_column: &str, env_config: &EnvironmentConfig) -> FieldMapping {
    let target_field = if env_config.prefix.starts_with("nrq") {
        "nrq_fundid".to_string()
    } else {
        "cgk_fundid".to_string()
    };

    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field,
        field_type: FieldType::LookupField,
        target_entity: Some("fund".to_string()),
        junction_entity: None,
    }
}

/// Create mapping for commission lookup field
fn create_commission_lookup_mapping(excel_column: &str, env_config: &EnvironmentConfig) -> FieldMapping {
    let target_field = if env_config.prefix.starts_with("nrq") {
        "nrq_commissionid".to_string()
    } else {
        "cgk_commissionid".to_string()
    };

    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field,
        field_type: FieldType::LookupField,
        target_entity: Some("commission".to_string()),
        junction_entity: None,
    }
}

/// Create mapping for date field
fn create_date_field_mapping(excel_column: &str) -> FieldMapping {
    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: "cgk_date".to_string(), // This will be environment-specific
        field_type: FieldType::DirectField,
        target_entity: None,
        junction_entity: None,
    }
}

/// Create mapping for time field
fn create_time_field_mapping(excel_column: &str) -> FieldMapping {
    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: "cgk_time".to_string(), // This will be environment-specific
        field_type: FieldType::DirectField,
        target_entity: None,
        junction_entity: None,
    }
}

/// Create mapping for user/project manager field
fn create_user_lookup_mapping(excel_column: &str) -> FieldMapping {
    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: "ownerid".to_string(), // Standard Dynamics user field
        field_type: FieldType::LookupField,
        target_entity: Some("systemuser".to_string()),
        junction_entity: None,
    }
}

/// Create mapping for information field
fn create_info_field_mapping(excel_column: &str) -> FieldMapping {
    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: "cgk_info".to_string(), // This will be environment-specific
        field_type: FieldType::DirectField,
        target_entity: None,
        junction_entity: None,
    }
}

/// Create mapping for board meeting field (environment-aware)
fn create_board_meeting_mapping(excel_column: &str, env_config: &EnvironmentConfig) -> FieldMapping {
    if let Some(board_config) = &env_config.board_meeting {
        let target_field = board_config.relationship_field.clone();
        let target_entity = if board_config.entity_type == "cgk_deadline" {
            Some("deadline".to_string()) // Self-referencing for CGK
        } else {
            Some("boardmeeting".to_string()) // Separate entity for NRQ
        };

        FieldMapping {
            excel_column: excel_column.to_string(),
            target_field,
            field_type: FieldType::LookupField,
            target_entity,
            junction_entity: None,
        }
    } else {
        // Fallback if no board meeting config
        FieldMapping {
            excel_column: excel_column.to_string(),
            target_field: "cgk_boardmeeting".to_string(),
            field_type: FieldType::Ignore,
            target_entity: None,
            junction_entity: None,
        }
    }
}

/// Create mapping for N:N relationship fields (checkbox columns)
fn create_multiselect_mapping(excel_column: &str, logical_type: &str, env_config: &EnvironmentConfig) -> FieldMapping {
    // Create junction entity name based on logical type and environment
    let junction_entity = if env_config.prefix.starts_with("cgk") {
        // CGK: cgk_cgk_deadline_cgk_<entity>
        match logical_type {
            "support" => "cgk_cgk_deadline_cgk_support".to_string(),
            "category" => "cgk_cgk_deadline_cgk_category".to_string(),
            "length" => "cgk_cgk_deadline_cgk_length".to_string(),
            "flemish_share" => "cgk_cgk_deadline_cgk_flemishshare".to_string(),
            _ => format!("cgk_cgk_deadline_cgk_{}", logical_type), // Fallback for CGK
        }
    } else if env_config.prefix.starts_with("nrq") {
        // NRQ: nrq_Deadline_nrq_<Entity>_nrq_<Entity> (note capitalization)
        let entity_capitalized = capitalize_entity_name(logical_type);
        match logical_type {
            "support" => format!("nrq_Deadline_nrq_{}_nrq_{}", entity_capitalized, entity_capitalized),
            "category" => format!("nrq_Deadline_nrq_{}_nrq_{}", entity_capitalized, entity_capitalized),
            "length" => format!("nrq_Deadline_nrq_Subcategory_nrq_Subcategory"), // Length maps to Subcategory in NRQ
            "flemish_share" => format!("nrq_Deadline_nrq_Flemishshare_nrq_Flemishshare"),
            _ => format!("nrq_Deadline_nrq_{}_nrq_{}", entity_capitalized, entity_capitalized),
        }
    } else {
        // Fallback for unknown environments
        format!("{}_deadline_{}", env_config.prefix, logical_type)
    };

    FieldMapping {
        excel_column: excel_column.to_string(),
        target_field: format!("{}_multiselect", logical_type), // Virtual field for N:N
        field_type: FieldType::MultiSelect,
        target_entity: Some(logical_type.to_string()),
        junction_entity: Some(junction_entity),
    }
}

/// Capitalize entity name for NRQ junction entities
fn capitalize_entity_name(logical_type: &str) -> String {
    match logical_type {
        "support" => "Support".to_string(),
        "category" => "Category".to_string(),
        "length" => "Subcategory".to_string(), // Length is Subcategory in NRQ
        "flemish_share" => "Flemishshare".to_string(),
        _ => {
            // Capitalize first letter for other entities
            let mut chars = logical_type.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }
    }
}