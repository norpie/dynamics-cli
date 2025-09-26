use anyhow::Result;
use std::collections::{HashMap, HashSet};
use log::debug;

use super::config::{DeadlineConfig, EnvironmentConfig};
use super::excel_parser::SheetData;

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub total_columns: usize,
    pub entity_columns: Vec<String>,
    pub matched_entities: Vec<EntityMatch>,
    pub unmatched_columns: Vec<String>,
    pub missing_entities: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EntityMatch {
    pub column_header: String,
    pub logical_type: String,
    pub entity_name: String,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.unmatched_columns.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Validation Summary:\n\
             • Total columns: {}\n\
             • Entity reference columns: {}\n\
             • Matched entities: {}\n\
             • Unmatched columns: {}\n\
             • Missing entities: {}",
            self.total_columns,
            self.entity_columns.len(),
            self.matched_entities.len(),
            self.unmatched_columns.len(),
            self.missing_entities.len()
        )
    }
}

/// Validate Excel checkbox columns against configured entity mappings
pub fn validate_excel_entities(
    sheet_data: &SheetData,
    environment_name: &str,
    deadline_config: &DeadlineConfig,
) -> Result<ValidationResult> {
    debug!("Starting Excel entity validation for environment: {}", environment_name);

    let env_config = deadline_config.get_environment(environment_name)
        .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in deadline config", environment_name))?;

    debug!("Found {} headers in Excel sheet", sheet_data.headers.len());
    debug!("Headers: {:?}", sheet_data.headers);

    // Identify checkbox columns (after "Beslissing" column, like in Python system)
    let checkbox_columns = identify_checkbox_columns(&sheet_data.headers);
    debug!("Identified {} checkbox columns: {:?}", checkbox_columns.len(), checkbox_columns);

    // Build entity lookup map from config
    let entity_lookup = build_entity_lookup(env_config);
    debug!("Built entity lookup with {} entries", entity_lookup.len());

    // Match Excel columns against configured entities
    let mut matched_entities = Vec::new();
    let mut unmatched_columns = Vec::new();

    for column_header in &checkbox_columns {
        if let Some((logical_type, entity_name)) = find_matching_entity(column_header, &entity_lookup) {
            matched_entities.push(EntityMatch {
                column_header: column_header.clone(),
                logical_type: logical_type.clone(),
                entity_name: entity_name.clone(),
            });
            debug!("Matched column '{}' to entity '{}' (logical type: {})",
                   column_header, entity_name, logical_type);
        } else {
            unmatched_columns.push(column_header.clone());
            debug!("No match found for column '{}'", column_header);
        }
    }

    // Find entities that are configured but not present in Excel
    let matched_logical_types: HashSet<_> = matched_entities.iter()
        .map(|m| m.logical_type.as_str())
        .collect();

    let missing_entities: Vec<String> = entity_lookup.keys()
        .filter(|logical_type| !matched_logical_types.contains(logical_type.as_str()))
        .cloned()
        .collect();

    debug!("Missing entities (configured but not in Excel): {:?}", missing_entities);

    Ok(ValidationResult {
        total_columns: sheet_data.headers.len(),
        entity_columns: checkbox_columns.clone(),
        matched_entities,
        unmatched_columns,
        missing_entities,
    })
}

/// Identify checkbox columns (after "Beslissing" column, like in Python system)
fn identify_checkbox_columns(headers: &[String]) -> Vec<String> {
    // Find the "Beslissing" column index
    let beslissing_index = headers.iter()
        .position(|h| h.to_lowercase().contains("beslissing"));

    if let Some(start_index) = beslissing_index {
        // Checkbox columns start after "Beslissing"
        headers.iter()
            .skip(start_index + 1)
            .filter(|header| !header.trim().is_empty()) // Skip empty headers
            .cloned()
            .collect()
    } else {
        // Fallback: if no "Beslissing" found, assume last portion are checkbox columns
        // Look for common data column patterns and take everything after
        let data_columns = ["domein", "fonds", "deadline", "projectbeheerder", "informatie", "datum", "commissie"];

        let last_data_column_index = headers.iter()
            .rposition(|h| {
                let header_lower = h.to_lowercase();
                data_columns.iter().any(|col| header_lower.contains(col))
            });

        if let Some(start_index) = last_data_column_index {
            headers.iter()
                .skip(start_index + 1)
                .filter(|header| !header.trim().is_empty())
                .cloned()
                .collect()
        } else {
            // Ultimate fallback: all non-empty headers
            headers.iter()
                .filter(|header| !header.trim().is_empty())
                .cloned()
                .collect()
        }
    }
}

/// Build a lookup map from logical types to entity names
fn build_entity_lookup(env_config: &EnvironmentConfig) -> HashMap<String, String> {
    let mut lookup = HashMap::new();

    for (logical_type, mapping) in &env_config.entities {
        lookup.insert(logical_type.clone(), mapping.entity.clone());
    }

    lookup
}

/// Find a matching entity for a column header
fn find_matching_entity(
    column_header: &str,
    entity_lookup: &HashMap<String, String>
) -> Option<(String, String)> {
    let header_lower = column_header.to_lowercase();

    // First try exact match on logical type
    if let Some(entity_name) = entity_lookup.get(&header_lower) {
        return Some((header_lower, entity_name.clone()));
    }

    // Try partial matches - column header contains logical type or vice versa
    for (logical_type, entity_name) in entity_lookup {
        let logical_lower = logical_type.to_lowercase();

        // Check if column contains logical type
        if header_lower.contains(&logical_lower) {
            return Some((logical_type.clone(), entity_name.clone()));
        }

        // Check if logical type contains column (for abbreviations)
        if logical_lower.contains(&header_lower) {
            return Some((logical_type.clone(), entity_name.clone()));
        }

        // Check entity name matching
        let entity_parts: Vec<&str> = entity_name.split('_').collect();
        if entity_parts.len() > 1 {
            let entity_suffix = entity_parts.last().unwrap().to_lowercase();
            if header_lower.contains(&entity_suffix) || entity_suffix.contains(&header_lower) {
                return Some((logical_type.clone(), entity_name.clone()));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::deadlines::config::{EntityMapping, EnvironmentConfig};

    #[test]
    fn test_identify_checkbox_columns() {
        let headers = vec![
            "id".to_string(),
            "name".to_string(),
            "category".to_string(),
            "fund".to_string(),
            "pillar".to_string(),
            "commission".to_string(),
            "created_date".to_string(),
        ];

        let checkbox_columns = identify_checkbox_columns(&headers);

        // Should identify category, fund, pillar, commission as checkbox columns
        // Should exclude id, name, created_date as standard columns
        assert!(checkbox_columns.contains(&"category".to_string()));
        assert!(checkbox_columns.contains(&"fund".to_string()));
        assert!(checkbox_columns.contains(&"pillar".to_string()));
        assert!(checkbox_columns.contains(&"commission".to_string()));
        assert!(!checkbox_columns.contains(&"id".to_string()));
        assert!(!checkbox_columns.contains(&"name".to_string()));
        assert!(!checkbox_columns.contains(&"created_date".to_string()));
    }

    #[test]
    fn test_find_matching_entity() {
        let mut entity_lookup = HashMap::new();
        entity_lookup.insert("category".to_string(), "cgk_category".to_string());
        entity_lookup.insert("fund".to_string(), "cgk_fund".to_string());
        entity_lookup.insert("pillar".to_string(), "cgk_pillar".to_string());

        // Test exact match
        assert_eq!(
            find_matching_entity("category", &entity_lookup),
            Some(("category".to_string(), "cgk_category".to_string()))
        );

        // Test partial match
        assert_eq!(
            find_matching_entity("Category Type", &entity_lookup),
            Some(("category".to_string(), "cgk_category".to_string()))
        );

        // Test no match
        assert_eq!(
            find_matching_entity("unknown", &entity_lookup),
            None
        );
    }
}