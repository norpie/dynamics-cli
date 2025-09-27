use anyhow::Result;
use std::collections::HashMap;
use log::debug;
use chrono::{NaiveDate, NaiveTime, DateTime};
use serde::{Serialize, Deserialize};

use super::config::EnvironmentConfig;
use super::excel_parser::SheetData;
use super::field_mapping_tui::FieldMapping;
use super::csv_cache::CsvCacheManager;
use super::timezone_utils::{combine_brussels_datetime, parse_time_string, excel_serial_to_date};

/// Calculate similarity between two strings using Levenshtein distance
fn calculate_similarity(a: &str, b: &str) -> f64 {
    let len_a = a.len();
    let len_b = b.len();

    if len_a == 0 { return if len_b == 0 { 1.0 } else { 0.0 }; }
    if len_b == 0 { return 0.0; }

    let max_len = len_a.max(len_b);
    let distance = levenshtein_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];

    for i in 0..=len_a { matrix[i][0] = i; }
    for j in 0..=len_b { matrix[0][j] = j; }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len_a][len_b]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformedRecord {
    pub excel_row_number: usize,
    pub main_entity: TransformedEntity,
    pub lookup_fields: HashMap<String, LookupResult>,
    pub junction_relationships: Vec<JunctionRelationship>,
    pub validation_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformedEntity {
    pub entity_name: String,
    pub fields: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResult {
    pub source_value: String,
    pub resolved_id: Option<String>,
    pub resolved_name: Option<String>,
    pub entity_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JunctionRelationship {
    pub source_column: String,
    pub source_value: String,
    pub junction_entity: String,
    pub target_entity: String,
    pub target_ids: Vec<String>,
    pub target_names: Vec<String>,
}

pub struct DataTransformer {
    env_config: EnvironmentConfig,
    cache_manager: CsvCacheManager,
    field_mappings: HashMap<String, FieldMapping>,
}

impl DataTransformer {
    pub fn new(
        env_config: EnvironmentConfig,
        environment_name: String,
        field_mappings: HashMap<String, FieldMapping>,
    ) -> Self {
        let cache_manager = CsvCacheManager::new(environment_name);

        Self {
            env_config,
            cache_manager,
            field_mappings,
        }
    }

    /// Transform Excel sheet data into Dynamics 365 ready records
    pub async fn transform_sheet_data(&self, sheet_data: &SheetData) -> Result<Vec<TransformedRecord>> {
        debug!("Starting data transformation for {} rows", sheet_data.rows.len());

        let mut transformed_records = Vec::new();

        // Skip the first row (row 1 in Excel) which contains explanation text
        // Headers are at row 2, data starts at row 3
        for (row_index, row_data) in sheet_data.rows.iter().enumerate().skip(1) {
            let excel_row_number = row_index + 2; // Excel rows start at 1, explanation at 1, headers at 2, data at 3

            debug!("Transforming row {}", excel_row_number);

            match self.transform_single_row(row_data, &sheet_data.headers, excel_row_number).await {
                Ok(transformed_record) => {
                    transformed_records.push(transformed_record);
                }
                Err(e) => {
                    debug!("Failed to transform row {}: {}", excel_row_number, e);
                    // Create a record with error information
                    let mut warnings = vec![format!("Transformation failed: {}", e)];

                    let error_record = TransformedRecord {
                        excel_row_number,
                        main_entity: TransformedEntity {
                            entity_name: self.get_main_entity_name(),
                            fields: HashMap::new(),
                        },
                        lookup_fields: HashMap::new(),
                        junction_relationships: Vec::new(),
                        validation_warnings: warnings,
                    };

                    transformed_records.push(error_record);
                }
            }
        }

        debug!("Transformation complete: {} records processed", transformed_records.len());
        Ok(transformed_records)
    }

    async fn transform_single_row(
        &self,
        row_data: &[String],
        headers: &[String],
        excel_row_number: usize,
    ) -> Result<TransformedRecord> {
        let mut main_entity_fields = HashMap::new();
        let mut lookup_fields = HashMap::new();
        let mut junction_relationships = Vec::new();
        let mut validation_warnings = Vec::new();

        // Create row data map for easier access
        let row_map: HashMap<String, String> = headers.iter()
            .zip(row_data.iter())
            .map(|(h, d)| (h.clone(), d.clone()))
            .collect();

        debug!("Processing row with {} columns", row_map.len());

        // Process each field mapping
        for (excel_column, field_mapping) in &self.field_mappings {
            if let Some(excel_value) = row_map.get(excel_column) {
                if excel_value.trim().is_empty() {
                    continue; // Skip empty values
                }

                match &field_mapping.field_type {
                    super::field_mapping_tui::FieldType::DirectField => {
                        let transformed_value = self.transform_direct_field(excel_column, excel_value, &row_map)?;
                        main_entity_fields.insert(field_mapping.target_field.clone(), transformed_value);
                    }

                    super::field_mapping_tui::FieldType::LookupField => {
                        let lookup_result = self.transform_lookup_field(excel_column, excel_value, field_mapping, excel_row_number).await?;

                        if let Some(resolved_id) = &lookup_result.resolved_id {
                            main_entity_fields.insert(field_mapping.target_field.clone(), serde_json::Value::String(resolved_id.clone()));
                        } else {
                            validation_warnings.push(format!("Could not resolve lookup for '{}': '{}'", excel_column, lookup_result.source_value));
                        }

                        lookup_fields.insert(excel_column.clone(), lookup_result);
                    }

                    super::field_mapping_tui::FieldType::MultiSelect => {
                        let junction_relationship = self.transform_multiselect_field(excel_column, excel_value, field_mapping).await?;

                        if !junction_relationship.target_ids.is_empty() {
                            junction_relationships.push(junction_relationship);
                        } else {
                            validation_warnings.push(format!("No entities found for checkbox column '{}': '{}'", excel_column, excel_value));
                        }
                    }

                    super::field_mapping_tui::FieldType::Ignore => {
                        debug!("Ignoring column '{}' as configured", excel_column);
                    }
                }
            }
        }

        // Apply special transformation logic (from Python parser)
        self.apply_special_transformations(&mut main_entity_fields, &row_map, &mut lookup_fields, &mut validation_warnings).await?;

        Ok(TransformedRecord {
            excel_row_number,
            main_entity: TransformedEntity {
                entity_name: self.get_main_entity_name(),
                fields: main_entity_fields,
            },
            lookup_fields,
            junction_relationships,
            validation_warnings,
        })
    }

    /// Transform direct field values (strings, dates, numbers)
    fn transform_direct_field(
        &self,
        excel_column: &str,
        excel_value: &str,
        row_map: &HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        let column_lower = excel_column.to_lowercase();

        match column_lower.as_str() {
            // Date fields
            h if h.contains("datum") || h.contains("date") => {
                // Check if there's a corresponding time field
                let time_value = self.find_time_field(row_map);
                self.transform_date_time_combination(excel_value, time_value.as_deref())
            }

            // Time fields (handled with date above, so skip standalone)
            h if h.contains("tijd") || h.contains("time") => {
                Ok(serde_json::Value::Null) // Skip standalone time fields
            }

            // Deadline name field - apply entity name generation
            h if h.contains("deadline") => {
                let entity_name = self.generate_entity_name(excel_value, row_map)?;
                Ok(serde_json::Value::String(entity_name))
            }

            // Default: string value
            _ => Ok(serde_json::Value::String(excel_value.to_string())),
        }
    }

    /// Transform lookup field values by resolving against CSV cache
    async fn transform_lookup_field(
        &self,
        excel_column: &str,
        excel_value: &str,
        field_mapping: &FieldMapping,
        excel_row: usize,
    ) -> Result<LookupResult> {
        let column_lower = excel_column.to_lowercase();

        // Special case: project manager (generate email from name)
        if column_lower.contains("projectbeheerder") || column_lower.contains("project") {
            return self.transform_project_manager_field(excel_value).await;
        }

        // Special case: Raad van Bestuur (environment-aware board meeting lookup)
        // Support both "Raad van Bestuur" and "RvB" patterns
        if (column_lower.contains("raad") && column_lower.contains("bestuur")) ||
           (column_lower.contains("rvb")) {
            debug!("Detected board meeting field '{}' with value '{}'", excel_column, excel_value);
            return self.transform_board_meeting_field(excel_value).await;
        }

        // Standard lookup: resolve against CSV cache
        if let Some(target_entity) = &field_mapping.target_entity {
            let entity_mapping = self.env_config.entities.get(target_entity);

            if let Some(mapping) = entity_mapping {
                return self.resolve_csv_lookup_with_context(&mapping.entity, excel_value, target_entity, Some(excel_row)).await;
            }
        }

        // Fallback: unresolved lookup
        Ok(LookupResult {
            source_value: excel_value.to_string(),
            resolved_id: None,
            resolved_name: None,
            entity_type: field_mapping.target_entity.clone().unwrap_or_default(),
        })
    }

    /// Transform multiselect field values (X-marked columns to N:N relationships)
    async fn transform_multiselect_field(
        &self,
        excel_column: &str,
        excel_value: &str,
        field_mapping: &FieldMapping,
    ) -> Result<JunctionRelationship> {
        // Check if the value indicates "selected" (X, checkmark, etc.)
        let is_selected = self.is_checkbox_selected(excel_value);

        let mut target_ids = Vec::new();
        let mut target_names = Vec::new();

        if is_selected {
            // Look up the entity by the column header (entity name)
            if let Some(target_entity) = &field_mapping.target_entity {
                let entity_mapping = self.env_config.entities.get(target_entity);

                if let Some(mapping) = entity_mapping {
                    // For multiselect, the column header is the entity name to look up
                    let lookup_result = self.resolve_csv_lookup(&mapping.entity, excel_column, target_entity).await?;

                    if let Some(id) = lookup_result.resolved_id {
                        target_ids.push(id);
                    }
                    if let Some(name) = lookup_result.resolved_name {
                        target_names.push(name);
                    }
                }
            }
        }

        Ok(JunctionRelationship {
            source_column: excel_column.to_string(),
            source_value: excel_value.to_string(),
            junction_entity: field_mapping.junction_entity.clone().unwrap_or_default(),
            target_entity: field_mapping.target_entity.clone().unwrap_or_default(),
            target_ids,
            target_names,
        })
    }

    /// Apply special transformation logic from Python parser
    async fn apply_special_transformations(
        &self,
        main_entity_fields: &mut HashMap<String, serde_json::Value>,
        row_map: &HashMap<String, String>,
        lookup_fields: &mut HashMap<String, LookupResult>,
        validation_warnings: &mut Vec<String>,
    ) -> Result<()> {
        // TODO: Implement additional Python transformation logic here
        // - Complex validation rules
        // - Cross-field dependencies
        // - Business-specific transformations

        debug!("Applied special transformations");
        Ok(())
    }

    /// Generate entity name: "Deadline Name - 2024-12-25 14:30"
    fn generate_entity_name(&self, deadline_name: &str, row_map: &HashMap<String, String>) -> Result<String> {
        // Find date and time from the row
        let mut formatted_datetime = String::new();

        // Look for date field
        if let Some((_, date_value)) = row_map.iter().find(|(k, _)| {
            let key_lower = k.to_lowercase();
            key_lower.contains("datum") || key_lower.contains("date")
        }) {
            if !date_value.trim().is_empty() {
                let time_value = self.find_time_field(row_map);
                if let Ok(datetime_val) = self.transform_date_time_combination(date_value, time_value.as_deref()) {
                    if let Some(datetime_str) = datetime_val.as_str() {
                        // Use a simpler format for entity names (not the full UTC timestamp)
                        if let Ok(parsed_dt) = chrono::DateTime::parse_from_rfc3339(datetime_str) {
                            // Convert back to Brussels time for display in entity name
                            let brussels_dt = parsed_dt.with_timezone(&chrono_tz::Europe::Brussels);
                            formatted_datetime = brussels_dt.format("%Y-%m-%d %H:%M").to_string();
                        } else {
                            formatted_datetime = datetime_str.to_string();
                        }
                    }
                }
            }
        }

        if formatted_datetime.is_empty() {
            Ok(deadline_name.to_string())
        } else {
            Ok(format!("{} - {}", deadline_name, formatted_datetime))
        }
    }

    /// Generate email from Dutch name: "Jan Van Der Berg" → "jvandenberg@vaf.be"
    async fn transform_project_manager_field(&self, name: &str) -> Result<LookupResult> {
        let email = self.generate_email_from_name(name);

        // Only try to resolve SystemUser by email since CSV contains domainname field
        self.resolve_csv_lookup("systemuser", &email, "systemuser").await
    }

    /// Transform Raad van Bestuur field (environment-aware)
    async fn transform_board_meeting_field(&self, date_value: &str) -> Result<LookupResult> {
        if let Some(board_config) = &self.env_config.board_meeting {
            // Parse the Excel serial date to a proper date
            match self.parse_date_flexible(date_value) {
                Ok(parsed_date) => {
                    // Format the date to match CSV format
                    let formatted_date = parsed_date.format("%Y-%m-%d").to_string();
                    debug!("Parsed board meeting date '{}' to '{}'", date_value, formatted_date);

                    // Create the lookup value based on environment
                    let lookup_result = if self.env_config.prefix.starts_with("nrq") {
                        // NRQ format: "RvB - DD/MM/YYYY"
                        let dd_mm_yyyy = parsed_date.format("%d/%m/%Y").to_string();
                        let lookup_value = format!("RvB - {}", dd_mm_yyyy);
                        debug!("Looking up board meeting with value: '{}'", lookup_value);
                        let mut result = self.resolve_board_meeting_csv_lookup(&lookup_value, "boardmeeting").await?;
                        result.source_value = lookup_value;
                        result
                    } else {
                        // CGK format: try "Bestuur - D/MM/YYYY" first, then "Bestuur + Algemene Vergadering - D/MM/YYYY"
                        // Use %-d to avoid leading zero for day (3/02/2025 not 03/02/2025)
                        let dd_mm_yyyy = parsed_date.format("%-d/%m/%Y").to_string();
                        let lookup_value1 = format!("Bestuur - {}", dd_mm_yyyy);
                        debug!("Looking up board meeting with value: '{}'", lookup_value1);

                        let mut result = self.resolve_board_meeting_csv_lookup(&lookup_value1, "boardmeeting").await?;

                        if result.resolved_id.is_none() {
                            // Try alternative format
                            let lookup_value2 = format!("Bestuur + Algemene Vergadering - {}", dd_mm_yyyy);
                            debug!("First lookup failed, trying: '{}'", lookup_value2);
                            result = self.resolve_board_meeting_csv_lookup(&lookup_value2, "boardmeeting").await?;
                            result.source_value = lookup_value2;
                        } else {
                            result.source_value = lookup_value1;
                        }
                        result
                    };

                    // Result is already prepared with correct source_value
                    return Ok(lookup_result);
                }
                Err(e) => {
                    // If date parsing fails, try direct lookup as fallback
                    debug!("Failed to parse board meeting date '{}': {}, trying direct lookup", date_value, e);
                    return self.resolve_csv_lookup(&board_config.entity_type, date_value, "boardmeeting").await;
                }
            }
        }

        // Fallback: no board meeting support
        debug!("No board meeting support configured for environment");
        Ok(LookupResult {
            source_value: date_value.to_string(),
            resolved_id: None,
            resolved_name: None,
            entity_type: "boardmeeting".to_string(),
        })
    }

    // Helper methods

    fn get_main_entity_name(&self) -> String {
        self.env_config.main_entity.clone()
    }

    fn find_time_field(&self, row_map: &HashMap<String, String>) -> Option<String> {
        row_map.iter()
            .find(|(k, _)| {
                let key_lower = k.to_lowercase();
                key_lower.contains("tijd") || key_lower.contains("time")
            })
            .map(|(_, v)| v.clone())
    }

    fn transform_date_time_combination(&self, date_str: &str, time_str: Option<&str>) -> Result<serde_json::Value> {
        // Parse date - try multiple formats
        let date = self.parse_date_flexible(date_str)?;

        // Parse time or use None for default handling
        let time = if let Some(time_str) = time_str {
            Some(parse_time_string(time_str).unwrap_or_else(|_| {
                debug!("Failed to parse time '{}', using default 12:00", time_str);
                NaiveTime::from_hms_opt(12, 0, 0).unwrap()
            }))
        } else {
            None // Let combine_brussels_datetime handle the default
        };

        // Combine date and time in Brussels timezone, convert to UTC
        let utc_datetime = combine_brussels_datetime(date, time)?;

        // Format as ISO 8601 for Dynamics 365 storage
        let formatted = utc_datetime.format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
        debug!("Brussels local time -> UTC: {}", formatted);

        Ok(serde_json::Value::String(formatted))
    }

    fn parse_date_flexible(&self, date_str: &str) -> Result<NaiveDate> {
        // First try to parse as Excel serial date (number)
        if let Ok(serial_date) = date_str.parse::<f64>() {
            return excel_serial_to_date(serial_date);
        }

        // Try string date formats
        let date_formats = [
            "%Y-%m-%d",
            "%d/%m/%Y",
            "%d-%m-%Y",
            "%m/%d/%Y",
            "%Y/%m/%d",
        ];

        for format in &date_formats {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                return Ok(date);
            }
        }

        Err(anyhow::anyhow!("Could not parse date: {}", date_str))
    }


    fn parse_time_flexible(&self, time_str: &str) -> Result<NaiveTime> {
        let time_formats = [
            "%H:%M",
            "%H:%M:%S",
            "%I:%M %p",
            "%H.%M",
        ];

        for format in &time_formats {
            if let Ok(time) = NaiveTime::parse_from_str(time_str, format) {
                return Ok(time);
            }
        }

        Err(anyhow::anyhow!("Could not parse time: {}", time_str))
    }

    fn generate_email_from_name(&self, name: &str) -> String {
        let name = name.trim();
        if name.is_empty() {
            return "unknown@vaf.be".to_string();
        }

        let name_parts: Vec<&str> = name.split_whitespace().collect();
        if name_parts.len() < 2 {
            return format!("{}@vaf.be", name.to_lowercase().replace(" ", ""));
        }

        // Extract first name and last name
        let first_name = name_parts[0];
        let last_name = name_parts[name_parts.len() - 1];

        // Generate email: first initial + lastname @vaf.be
        let first_initial = first_name.chars().next().unwrap_or('x').to_lowercase().to_string();

        // Handle names with prefixes (Van, De, etc.) - Python logic
        let prefixes = ["van", "de", "der", "den", "te", "ten"];

        let last_name_clean = if name_parts.len() > 2 {
            let potential_prefix = name_parts[name_parts.len() - 2].to_lowercase();
            if prefixes.contains(&potential_prefix.as_str()) {
                // Combine prefix and lastname: "Van Hellemont" -> "vanhellemont"
                format!("{}{}", potential_prefix, last_name.to_lowercase())
            } else {
                last_name.to_lowercase()
            }
        } else {
            last_name.to_lowercase()
        };

        format!("{}{}@vaf.be", first_initial, last_name_clean)
    }

    fn is_checkbox_selected(&self, value: &str) -> bool {
        // For checkbox fields, any non-empty cell content means "checked"
        // Empty cells mean "unchecked"
        !value.trim().is_empty()
    }

    async fn resolve_board_meeting_csv_lookup(&self, lookup_value: &str, logical_type: &str) -> Result<LookupResult> {
        // Load board meeting names from the specific board meeting CSV file
        let csv_filename = if let Some(board_config) = &self.env_config.board_meeting {
            &board_config.csv_name
        } else {
            "bestuur_deadlines.csv" // fallback
        };

        match self.cache_manager.load_entity_names_from_file(csv_filename) {
            Ok(entity_map) => {
                debug!("Loaded {} entries from {}", entity_map.len(), csv_filename);
                let lookup_lower = lookup_value.to_lowercase();
                debug!("Looking for '{}' (lowercase: '{}')", lookup_value, lookup_lower);

                // Debug: show first few entries
                for (i, (name, id)) in entity_map.iter().enumerate().take(5) {
                    debug!("CSV entry {}: '{}' -> {}", i, name, id);
                }

                // Debug: specifically check for our target entry
                if entity_map.contains_key("bestuur - 10/03/2025 16:30") {
                    debug!("Found exact target entry: bestuur - 10/03/2025 16:30");
                }

                // Debug: check partial match manually
                for (name, _id) in &entity_map {
                    if name.contains("10/03/2025") {
                        debug!("Found entry containing 10/03/2025: '{}'", name);
                    }
                }

                // Try exact match first (normalize non-breaking spaces)
                let lookup_normalized = lookup_lower.replace('\u{00A0}', " ");
                if let Some(id) = entity_map.get(&lookup_normalized) {
                    debug!("Found exact match: {} -> {}", lookup_normalized, id);
                    return Ok(LookupResult {
                        source_value: lookup_value.to_string(),
                        resolved_id: Some(id.clone()),
                        resolved_name: Some(lookup_value.to_string()),
                        entity_type: logical_type.to_string(),
                    });
                }

                // Try partial match with normalized strings (handle non-breaking spaces)
                for (name, id) in &entity_map {
                    // Normalize CSV name by replacing non-breaking spaces with regular spaces
                    let name_normalized = name.replace('\u{00A0}', " ");

                    if name.contains("10/03/2025") {
                        debug!("Checking potential match: '{}' vs '{}'", name, lookup_lower);
                        debug!("Name normalized: '{}' vs lookup normalized: '{}'", name_normalized, lookup_normalized);
                        debug!("name_normalized.contains(&lookup_normalized): {}", name_normalized.contains(&lookup_normalized));
                    }

                    if name_normalized.contains(&lookup_normalized) || lookup_normalized.contains(&name_normalized) {
                        debug!("Found partial match: '{}' contains '{}' -> {}", name, lookup_normalized, id);
                        return Ok(LookupResult {
                            source_value: lookup_value.to_string(),
                            resolved_id: Some(id.clone()),
                            resolved_name: Some(name.clone()),
                            entity_type: logical_type.to_string(),
                        });
                    }
                }

                debug!("No match found for '{}' in {} entries", lookup_value, entity_map.len());

                // No match found
                Ok(LookupResult {
                    source_value: lookup_value.to_string(),
                    resolved_id: None,
                    resolved_name: None,
                    entity_type: logical_type.to_string(),
                })
            }
            Err(e) => {
                debug!("Failed to load board meeting CSV: {}", e);
                Ok(LookupResult {
                    source_value: lookup_value.to_string(),
                    resolved_id: None,
                    resolved_name: None,
                    entity_type: logical_type.to_string(),
                })
            }
        }
    }

    async fn resolve_csv_lookup_with_context(&self, entity_name: &str, lookup_value: &str, logical_type: &str, excel_row: Option<usize>) -> Result<LookupResult> {
        // Load entity names from CSV cache
        match self.cache_manager.load_entity_names(entity_name) {
            Ok(entity_map) => {
                let lookup_lower = lookup_value.to_lowercase();


                // Try exact match first
                if let Some(id) = entity_map.get(&lookup_lower) {
                    return Ok(LookupResult {
                        source_value: lookup_value.to_string(),
                        resolved_id: Some(id.clone()),
                        resolved_name: Some(lookup_value.to_string()),
                        entity_type: logical_type.to_string(),
                    });
                }

                // Try partial match
                for (name, id) in &entity_map {
                    if name.contains(&lookup_lower) || lookup_lower.contains(name) {
                        return Ok(LookupResult {
                            source_value: lookup_value.to_string(),
                            resolved_id: Some(id.clone()),
                            resolved_name: Some(name.clone()),
                            entity_type: logical_type.to_string(),
                        });
                    }
                }

                // No match found - log details for categorization
                let row_info = if let Some(row) = excel_row {
                    format!(" [Excel row {}]", row)
                } else {
                    String::new()
                };
                debug!("❌ LOOKUP FAILED: entity='{}' value='{}' type='{}'{} (searched {} entries)",
                       entity_name, lookup_value, logical_type, row_info, entity_map.len());

                // Show closest matches for debugging
                let mut close_matches: Vec<(&String, f64)> = entity_map.keys()
                    .map(|name| {
                        let similarity = calculate_similarity(&lookup_lower, name);
                        (name, similarity)
                    })
                    .filter(|(_, sim)| *sim > 0.3) // Only show reasonable matches
                    .collect();
                close_matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                if !close_matches.is_empty() {
                    debug!("   Closest matches:");
                    for (name, similarity) in close_matches.iter().take(3) {
                        debug!("     - '{}' (similarity: {:.2})", name, similarity);
                    }
                } else {
                    debug!("   No similar entries found");
                }

                Ok(LookupResult {
                    source_value: lookup_value.to_string(),
                    resolved_id: None,
                    resolved_name: None,
                    entity_type: logical_type.to_string(),
                })
            }
            Err(e) => {
                debug!("Failed to load entity names for {}: {}", entity_name, e);
                Ok(LookupResult {
                    source_value: lookup_value.to_string(),
                    resolved_id: None,
                    resolved_name: None,
                    entity_type: logical_type.to_string(),
                })
            }
        }
    }

    async fn resolve_csv_lookup(&self, entity_name: &str, lookup_value: &str, logical_type: &str) -> Result<LookupResult> {
        self.resolve_csv_lookup_with_context(entity_name, lookup_value, logical_type, None).await
    }
}