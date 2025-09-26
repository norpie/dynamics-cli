use crate::dynamics::metadata::{FieldInfo, FormInfo, ViewInfo};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveTab {
    Fields,
    Relationships,
    Views,
    Forms,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedSide {
    Source,
    Target,
}

#[derive(Debug, Clone)]
pub struct Match<T> {
    pub source: T,
    pub target: Option<T>,
    pub match_score: f64,
    pub is_manual: bool,
}

#[derive(Debug, Clone)]
pub struct ComparisonData {
    // Core entity metadata
    pub source_fields: Vec<FieldInfo>,
    pub target_fields: Vec<FieldInfo>,
    pub source_views: Vec<ViewInfo>,
    pub target_views: Vec<ViewInfo>,
    pub source_forms: Vec<FormInfo>,
    pub target_forms: Vec<FormInfo>,

    // Entity names for titles
    pub source_entity: String,
    pub target_entity: String,

    // Environment names for back navigation
    pub source_env: String,
    pub target_env: String,

    // Field matches drive everything
    pub field_matches: Vec<Match<FieldInfo>>,

    // Computed from field matches
    pub view_matches: Vec<Match<ViewInfo>>,
    pub form_matches: Vec<Match<FormInfo>>,
}

#[derive(Debug, Clone)]
pub struct ExamplePair {
    pub id: String,
    pub source_uuid: String,
    pub target_uuid: String,
    pub label: Option<String>,
}

impl ExamplePair {
    pub fn new(source_uuid: String, target_uuid: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_uuid,
            target_uuid,
            label: None,
        }
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    pub fn display_name(&self) -> String {
        if let Some(label) = &self.label {
            format!("{} ({}...→{}...)",
                label,
                &self.source_uuid[..8],
                &self.target_uuid[..8]
            )
        } else {
            format!("{}... → {}...",
                &self.source_uuid[..8],
                &self.target_uuid[..8]
            )
        }
    }

    /// Convert to ConfigExamplePair for persistence
    pub fn to_config(&self) -> crate::config::ConfigExamplePair {
        crate::config::ConfigExamplePair {
            id: self.id.clone(),
            source_uuid: self.source_uuid.clone(),
            target_uuid: self.target_uuid.clone(),
            label: self.label.clone(),
        }
    }

    /// Convert from ConfigExamplePair
    pub fn from_config(config: &crate::config::ConfigExamplePair) -> Self {
        Self {
            id: config.id.clone(),
            source_uuid: config.source_uuid.clone(),
            target_uuid: config.target_uuid.clone(),
            label: config.label.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExamplesState {
    pub examples: Vec<ExamplePair>,
    pub active_example_id: Option<String>,
    pub examples_mode_enabled: bool,
    pub example_data: HashMap<String, Value>, // UUID -> fetched record data
}

impl Default for ExamplesState {
    fn default() -> Self {
        Self {
            examples: Vec::new(),
            active_example_id: None,
            examples_mode_enabled: false,
            example_data: HashMap::new(),
        }
    }
}

impl ExamplesState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_example(&mut self, example: ExamplePair) {
        // If this is the first example, make it active
        if self.examples.is_empty() {
            self.active_example_id = Some(example.id.clone());
        }
        self.examples.push(example);
    }

    pub fn remove_example(&mut self, id: &str) {
        if let Some(pos) = self.examples.iter().position(|e| e.id == id) {
            self.examples.remove(pos);

            // If we removed the active example, set a new one or clear
            if self.active_example_id.as_ref() == Some(&id.to_string()) {
                self.active_example_id = self.examples.first().map(|e| e.id.clone());
            }
        }
    }

    pub fn set_active_example(&mut self, id: &str) {
        if self.examples.iter().any(|e| e.id == id) {
            self.active_example_id = Some(id.to_string());
        }
    }

    pub fn get_active_example(&self) -> Option<&ExamplePair> {
        if let Some(active_id) = &self.active_example_id {
            self.examples.iter().find(|e| &e.id == active_id)
        } else {
            None
        }
    }

    pub fn toggle_examples_mode(&mut self) {
        self.examples_mode_enabled = !self.examples_mode_enabled;
    }

    pub fn get_example_value(&self, field_name: &str, is_source: bool) -> Option<String> {
        log::debug!("=== get_example_value called for field: {}, is_source: {} ===", field_name, is_source);
        log::debug!("  examples_mode_enabled: {}", self.examples_mode_enabled);

        if !self.examples_mode_enabled {
            log::debug!("  examples mode is disabled, returning None");
            return None;
        }

        let active_example = self.get_active_example()?;
        log::debug!("  active_example: {} -> {}", active_example.source_uuid, active_example.target_uuid);

        let (uuid, lookup_key) = if is_source {
            (&active_example.source_uuid, format!("source:{}", active_example.source_uuid))
        } else {
            (&active_example.target_uuid, format!("target:{}", active_example.target_uuid))
        };
        log::debug!("  🔍 LOOKUP CALCULATION: is_source={}, source_uuid={}, target_uuid={}",
                   is_source, active_example.source_uuid, active_example.target_uuid);
        log::debug!("  🎯 USING: uuid={}, key={}", uuid, lookup_key);

        log::debug!("  example_data has {} entries", self.example_data.len());
        for (data_uuid, _) in &self.example_data {
            log::debug!("    available UUID: {}", data_uuid);
        }

        if let Some(record_data) = self.example_data.get(&lookup_key) {
            log::debug!("  ✅ FOUND record data for key: {} (UUID: {})", lookup_key, uuid);
            log::debug!("  📊 record has {} fields", record_data.as_object().map_or(0, |obj| obj.len()));

            // Try to extract the field value using Dynamics 365 lookup fallback logic
            self.extract_field_value_with_fallbacks(record_data, field_name)
        } else {
            log::error!("  ❌ NO RECORD DATA FOUND for key: {} (UUID: {})", lookup_key, uuid);
            log::error!("  🔍 Requested: is_source={}, field_name={}", is_source, field_name);
            Some("[no data]".to_string())
        }
    }

    /// Extract field value with Dynamics 365 lookup fallback logic
    ///
    /// Priority order:
    /// 1. Formatted value annotation (human-readable names for lookups)
    /// 2. Direct field value (original approach)
    /// 3. Navigation property fallback (expanded lookup data)
    /// 4. Related field patterns (e.g., field_value -> field for lookups)
    /// 5. Smart GUID detection and abbreviation for lookup fields
    fn extract_field_value_with_fallbacks(&self, record_data: &Value, field_name: &str) -> Option<String> {
        // 1. Try formatted value annotation first (best for lookups)
        let formatted_key = format!("{}@OData.Community.Display.V1.FormattedValue", field_name);
        if let Some(value) = record_data.get(&formatted_key) {
            log::debug!("  found formatted value for field {}: {:?}", field_name, value);
            return self.convert_json_value_to_string(value, "formatted");
        }

        // 2. Try direct field value (original approach)
        if let Some(value) = record_data.get(field_name) {
            log::debug!("  found direct value for field {}: {:?}", field_name, value);
            return self.convert_json_value_to_string(value, "direct");
        }

        // 3. Try navigation property fallback for lookup fields
        if field_name.ends_with("_value") {
            let nav_prop_name = field_name.strip_suffix("_value").unwrap_or(field_name);

            // Try nav_prop/name pattern
            let name_path_key = format!("{}/name", nav_prop_name);
            if let Some(value) = record_data.get(&name_path_key) {
                log::debug!("  found navigation property name for field {}: {:?}", field_name, value);
                return self.convert_json_value_to_string(value, "navigation/name");
            }

            // Try nav_prop/fullname pattern
            let fullname_path_key = format!("{}/fullname", nav_prop_name);
            if let Some(value) = record_data.get(&fullname_path_key) {
                log::debug!("  found navigation property fullname for field {}: {:?}", field_name, value);
                return self.convert_json_value_to_string(value, "navigation/fullname");
            }

            // Try expanded navigation property object
            if let Some(nav_obj) = record_data.get(nav_prop_name) {
                if let Some(obj) = nav_obj.as_object() {
                    // Look for common display name fields in the expanded object
                    for display_field in ["name", "fullname", "subject", "title"] {
                        if let Some(value) = obj.get(display_field) {
                            log::debug!("  found expanded navigation property {}.{} for field {}: {:?}",
                                       nav_prop_name, display_field, field_name, value);
                            return self.convert_json_value_to_string(value, "expanded");
                        }
                    }
                }
            }
        }

        // 4. For non-_value fields that might be lookup references, try adding _value
        if !field_name.ends_with("_value") && !field_name.starts_with("_") {
            let value_field_name = format!("_{}_value", field_name);
            if let Some(value) = record_data.get(&value_field_name) {
                log::debug!("  found related _value field for {}: {:?}", field_name, value);
                return self.convert_json_value_to_string(value, "related_value");
            }

            // Also try the formatted value for the _value field
            let formatted_value_key = format!("_{}_value@OData.Community.Display.V1.FormattedValue", field_name);
            if let Some(value) = record_data.get(&formatted_value_key) {
                log::debug!("  found related formatted _value field for {}: {:?}", field_name, value);
                return self.convert_json_value_to_string(value, "related_formatted");
            }
        }

        // 5. Final fallback - field not found anywhere
        log::warn!("❌ FIELD NOT FOUND: {} not found in record data with any fallback", field_name);
        if let Some(obj) = record_data.as_object() {
            let available_fields: Vec<String> = obj.keys().cloned().collect();
            log::warn!("  📋 Available fields in JSON: {}", available_fields.join(", "));

            // Show similar field names to help debug mapping issues
            let similar_fields: Vec<String> = available_fields.iter()
                .filter(|field| field.contains(&field_name.replace("cgk", "").replace("nrq", "")) || field_name.contains(&field.replace("cgk", "").replace("nrq", "")))
                .cloned()
                .collect();
            if !similar_fields.is_empty() {
                log::warn!("  🔍 Similar field names found: {}", similar_fields.join(", "));
            }
        }
        Some("[no value]".to_string())
    }

    /// Convert a JSON Value to a display string with smart GUID handling
    fn convert_json_value_to_string(&self, value: &Value, source_type: &str) -> Option<String> {
        let result = match value {
            Value::String(s) => {
                if s.is_empty() {
                    "[empty]".to_string()
                } else {
                    s.clone()
                }
            },
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => "[null]".to_string(),
            Value::Array(arr) => {
                if arr.is_empty() {
                    "[empty array]".to_string()
                } else {
                    format!("[array with {} items]", arr.len())
                }
            },
            Value::Object(obj) => {
                if obj.is_empty() {
                    "[empty object]".to_string()
                } else {
                    format!("[object with {} fields]", obj.len())
                }
            }
        };

        log::debug!("  converted {} value to string: {}", source_type, result);
        Some(result)
    }

}

#[derive(Debug, Clone)]
pub struct SharedState {
    pub field_mappings: HashMap<String, String>,
    pub prefix_mappings: HashMap<String, String>,
    pub hide_matched: bool,
    pub examples: ExamplesState,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    NotStarted,
    LoadingSourceFields,
    LoadingTargetFields,
    LoadingSourceViews,
    LoadingTargetViews,
    LoadingSourceForms,
    LoadingTargetForms,
    ComputingMatches,
    Complete,
    Error(String),
}
