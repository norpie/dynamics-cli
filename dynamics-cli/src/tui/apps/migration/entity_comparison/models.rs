//! Data models for entity comparison app

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Sort mode for tree items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Alphabetical,
    MatchesFirst,
    SourceMatches,
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            SortMode::Alphabetical => "Alphabetical",
            SortMode::MatchesFirst => "Matches First",
            SortMode::SourceMatches => "Source Matches",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            SortMode::Alphabetical => SortMode::MatchesFirst,
            SortMode::MatchesFirst => SortMode::SourceMatches,
            SortMode::SourceMatches => SortMode::Alphabetical,
        }
    }
}

/// Hide mode for filtering tree items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HideMode {
    #[default]
    Off,         // Show all items
    HideMatched, // Hide items with matches
    HideIgnored, // Hide ignored items
    HideBoth,    // Hide matched AND ignored items
}

impl HideMode {
    pub fn label(&self) -> &'static str {
        match self {
            HideMode::Off => "Show All",
            HideMode::HideMatched => "Hide Matched",
            HideMode::HideIgnored => "Hide Ignored",
            HideMode::HideBoth => "Hide Both",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            HideMode::Off => HideMode::HideMatched,
            HideMode::HideMatched => HideMode::HideIgnored,
            HideMode::HideIgnored => HideMode::HideBoth,
            HideMode::HideBoth => HideMode::Off,
        }
    }
}

/// Active tab in the comparison view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Fields,
    Relationships,
    Views,
    Forms,
    Entities,
}

impl ActiveTab {
    /// Get tab label for display
    pub fn label(&self) -> &'static str {
        match self {
            ActiveTab::Fields => "Fields",
            ActiveTab::Relationships => "Relationships",
            ActiveTab::Views => "Views",
            ActiveTab::Forms => "Forms",
            ActiveTab::Entities => "Entities",
        }
    }

    /// Get tab number (1-indexed for keyboard shortcuts)
    pub fn number(&self) -> usize {
        match self {
            ActiveTab::Fields => 1,
            ActiveTab::Relationships => 2,
            ActiveTab::Views => 3,
            ActiveTab::Forms => 4,
            ActiveTab::Entities => 5,
        }
    }

    /// Switch to tab by number (1-indexed)
    pub fn from_number(n: usize) -> Option<Self> {
        match n {
            1 => Some(ActiveTab::Fields),
            2 => Some(ActiveTab::Relationships),
            3 => Some(ActiveTab::Views),
            4 => Some(ActiveTab::Forms),
            5 => Some(ActiveTab::Entities),
            _ => None,
        }
    }
}

/// Which side of the comparison is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Side {
    #[default]
    Source,
    Target,
}

/// Search mode for filtering tree items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Unified,      // One search box filters both sides
    Independent,  // Two search boxes, each filters one side
}

impl SearchMode {
    pub fn toggle(&self) -> Self {
        match self {
            SearchMode::Unified => SearchMode::Independent,
            SearchMode::Independent => SearchMode::Unified,
        }
    }
}

/// Example record pair for live data preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamplePair {
    pub id: String,
    pub source_record_id: String,
    pub target_record_id: String,
    pub label: Option<String>,
}

impl ExamplePair {
    pub fn new(source_record_id: String, target_record_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_record_id,
            target_record_id,
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
                &self.source_record_id[..8.min(self.source_record_id.len())],
                &self.target_record_id[..8.min(self.target_record_id.len())]
            )
        } else {
            format!("{}... → {}...",
                &self.source_record_id[..8.min(self.source_record_id.len())],
                &self.target_record_id[..8.min(self.target_record_id.len())]
            )
        }
    }
}

/// Field mapping information
#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub target_field: String,
    pub match_type: MatchType,
    pub confidence: f64,
}

/// Type of field match/mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Exact,        // Exact name match, types match
    Prefix,       // Prefix name match, types match
    TypeMismatch, // Name match but types differ
    Manual,       // User-created mapping (overrides type checking)
    ExampleValue, // Value-based match from example data
    Import,       // Imported from C# mapping file
}

impl MatchType {
    /// Get display label for match type
    pub fn label(&self) -> &'static str {
        match self {
            MatchType::Exact => "[Exact]",
            MatchType::Prefix => "[Prefix]",
            MatchType::TypeMismatch => "[Type Mismatch]",
            MatchType::Manual => "[Manual]",
            MatchType::ExampleValue => "[Example]",
            MatchType::Import => "[Import]",
        }
    }
}

/// Examples state
#[derive(Debug, Clone)]
pub struct ExamplesState {
    pub pairs: Vec<ExamplePair>,
    pub active_pair_id: Option<String>,
    pub enabled: bool,
    pub cache: HashMap<String, serde_json::Value>, // (entity:record_id) -> data
}

impl Default for ExamplesState {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            active_pair_id: None,
            enabled: false,
            cache: HashMap::new(),
        }
    }
}

impl ExamplesState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_active_pair(&self) -> Option<&ExamplePair> {
        if let Some(active_id) = &self.active_pair_id {
            self.pairs.iter().find(|p| &p.id == active_id)
        } else {
            None
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Get example value for a field
    /// For hierarchical paths (Forms/Views), extracts just the field name from the path
    /// field_name can be either a simple name like "accountname" or a path like "formtype/main/form/MainForm/tab/General/accountname"
    /// entity_name is the entity logical name (e.g., "cgk_deadline", "nrq_deadline")
    pub fn get_field_value(&self, field_name: &str, is_source: bool, entity_name: &str) -> Option<String> {
        if !self.enabled {
            return None;
        }

        // Get active pair
        let active_pair = self.get_active_pair()?;

        // Get the record ID for the appropriate side
        let record_id = if is_source {
            &active_pair.source_record_id
        } else {
            &active_pair.target_record_id
        };

        // Create composite cache key: entity:record_id
        let cache_key = format!("{}:{}", entity_name, record_id);

        log::debug!("Looking up field '{}' for cache_key '{}' (is_source: {})", field_name, cache_key, is_source);

        // Get the cached record data
        let record_data = self.cache.get(&cache_key);

        if record_data.is_none() {
            log::warn!("No cached data for cache_key: {} (is_source: {})", cache_key, is_source);
            log::warn!("Available cache keys: {:?}", self.cache.keys().collect::<Vec<_>>());
            return None;
        }

        let record_data = record_data.unwrap();

        log::debug!("Found cached data with {} fields", record_data.as_object().map(|o| o.len()).unwrap_or(0));

        // Extract just the field name from hierarchical path if present
        // e.g., "formtype/main/form/MainForm/tab/General/accountname" -> "accountname"
        let extracted_field_name = field_name
            .split('/')
            .last()
            .unwrap_or(field_name);

        log::debug!("Extracted field name: '{}' (from '{}')", extracted_field_name, field_name);

        // Try to get the field value from the JSON
        if let Some(value) = record_data.get(extracted_field_name) {
            log::debug!("Found value for field '{}': {:?}", extracted_field_name, value);
            // Format the value based on its type
            match value {
                serde_json::Value::String(s) => Some(format!("\"{}\"", s)),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                serde_json::Value::Null => Some("null".to_string()),
                serde_json::Value::Array(_) => Some("[...]".to_string()),
                serde_json::Value::Object(_) => {
                    // For lookups, try to get the formatted value
                    if let Some(formatted) = value.get("@OData.Community.Display.V1.FormattedValue") {
                        if let Some(s) = formatted.as_str() {
                            return Some(format!("\"{}\"", s));
                        }
                    }
                    Some("{...}".to_string())
                }
            }
        } else {
            // Field not found directly - check if it's a Virtual display field (e.g., organizationidname, createdbyyominame)
            // These are formatted as _field_value@FormattedValue in the API response

            // Try common patterns for virtual display fields:
            // Pattern: {field}yominame -> _{field}_value@FormattedValue
            // Pattern: {field}name -> _{field}_value@FormattedValue
            let lookup_formatted_key = if extracted_field_name.ends_with("yominame") {
                // Remove "yominame" suffix
                let base = extracted_field_name.strip_suffix("yominame").unwrap_or(extracted_field_name);
                format!("_{}_value@OData.Community.Display.V1.FormattedValue", base)
            } else if extracted_field_name.ends_with("name") {
                // Remove "name" suffix
                let base = extracted_field_name.strip_suffix("name").unwrap_or(extracted_field_name);
                format!("_{}_value@OData.Community.Display.V1.FormattedValue", base)
            } else {
                String::new()
            };

            if !lookup_formatted_key.is_empty() {
                if let Some(formatted_value) = record_data.get(&lookup_formatted_key) {
                    log::debug!("Found formatted value for virtual field '{}' at key '{}'", extracted_field_name, lookup_formatted_key);
                    if let Some(s) = formatted_value.as_str() {
                        return Some(format!("\"{}\"", s));
                    }
                }

                // If formatted value not found, check if the base lookup field exists and is null
                // e.g., for "organizationidname", check "_organizationid_value"
                let base_lookup_key = if extracted_field_name.ends_with("yominame") {
                    let base = extracted_field_name.strip_suffix("yominame").unwrap_or(extracted_field_name);
                    format!("_{}_value", base)
                } else if extracted_field_name.ends_with("name") {
                    let base = extracted_field_name.strip_suffix("name").unwrap_or(extracted_field_name);
                    format!("_{}_value", base)
                } else {
                    String::new()
                };

                if !base_lookup_key.is_empty() {
                    if let Some(base_value) = record_data.get(&base_lookup_key) {
                        // Lookup field exists - check if it's null
                        if base_value.is_null() {
                            log::debug!("Virtual field '{}' has null lookup value at '{}'", extracted_field_name, base_lookup_key);
                            return Some("null".to_string());
                        }
                    }
                }
            }

            log::debug!("Field '{}' not found in cached data (tried lookup key: '{}')", extracted_field_name, lookup_formatted_key);
            None
        }
    }
}
