//! Data models for entity comparison app

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Active tab in the comparison view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Fields,
    Relationships,
    Views,
    Forms,
}

impl ActiveTab {
    /// Get tab label for display
    pub fn label(&self) -> &'static str {
        match self {
            ActiveTab::Fields => "Fields",
            ActiveTab::Relationships => "Relationships",
            ActiveTab::Views => "Views",
            ActiveTab::Forms => "Forms",
        }
    }

    /// Get tab number (1-indexed for keyboard shortcuts)
    pub fn number(&self) -> usize {
        match self {
            ActiveTab::Fields => 1,
            ActiveTab::Relationships => 2,
            ActiveTab::Views => 3,
            ActiveTab::Forms => 4,
        }
    }

    /// Switch to tab by number (1-indexed)
    pub fn from_number(n: usize) -> Option<Self> {
        match n {
            1 => Some(ActiveTab::Fields),
            2 => Some(ActiveTab::Relationships),
            3 => Some(ActiveTab::Views),
            4 => Some(ActiveTab::Forms),
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
}

impl MatchType {
    /// Get display label for match type
    pub fn label(&self) -> &'static str {
        match self {
            MatchType::Exact => "[Exact]",
            MatchType::Prefix => "[Prefix]",
            MatchType::TypeMismatch => "[Type Mismatch]",
            MatchType::Manual => "[Manual]",
        }
    }
}

/// Sorting mode for field lists
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Alphabetical,
    MatchedFirst,
    UnmatchedFirst,
}

/// Examples state
#[derive(Debug, Clone)]
pub struct ExamplesState {
    pub pairs: Vec<ExamplePair>,
    pub active_pair_id: Option<String>,
    pub enabled: bool,
    pub cache: HashMap<String, serde_json::Value>, // record_id -> data
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
    /// TODO: Implement field value extraction logic
    pub fn get_field_value(&self, _field_name: &str, _is_source: bool) -> Option<String> {
        // Placeholder - will implement extraction logic later
        None
    }
}
