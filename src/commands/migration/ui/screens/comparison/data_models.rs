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
        log::debug!("get_example_value called for field: {}, is_source: {}", field_name, is_source);
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
        log::debug!("  looking for data for UUID: {} (key: {})", uuid, lookup_key);

        log::debug!("  example_data has {} entries", self.example_data.len());
        for (data_uuid, _) in &self.example_data {
            log::debug!("    available UUID: {}", data_uuid);
        }

        if let Some(record_data) = self.example_data.get(&lookup_key) {
            log::debug!("  found record data for UUID: {}", uuid);
            log::debug!("  record has {} fields", record_data.as_object().map_or(0, |obj| obj.len()));

            // Try to extract the field value from the JSON data
            if let Some(value) = record_data.get(field_name) {
                log::debug!("  found value for field {}: {:?}", field_name, value);
                match value {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    Value::Bool(b) => Some(b.to_string()),
                    Value::Null => Some("[null]".to_string()),
                    _ => Some("[complex]".to_string()),
                }
            } else {
                log::debug!("  field {} not found in record data", field_name);
                if let Some(obj) = record_data.as_object() {
                    log::debug!("  available fields: {:?}", obj.keys().collect::<Vec<_>>());
                }
                Some("[no value]".to_string())
            }
        } else {
            log::debug!("  no record data found for UUID: {}", uuid);
            Some("[no data]".to_string())
        }
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
