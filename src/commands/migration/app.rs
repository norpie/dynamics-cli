use ratatui::widgets::ListState;
use std::collections::HashMap;

use crate::config::Config;
use crate::dynamics::metadata::{FieldInfo, ViewInfo, ViewStructure, parse_view_structure};

#[derive(PartialEq, Clone, Copy)]
pub enum FocusedPanel {
    Source,
    Target,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HideMode {
    ShowAll,
    HideMatches,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ViewCompareResult {
    BackToViewSelection,
    Exit,
}

pub struct CompareApp {
    pub source_fields: Vec<FieldInfo>,
    pub target_fields: Vec<FieldInfo>,
    pub source_list_state: ListState,
    pub target_list_state: ListState,
    pub focused_panel: FocusedPanel,
    pub quit: bool,
    pub source_entity_name: String,
    pub target_entity_name: String,
    pub source_env: String,
    pub target_env: String,
    pub source_area: ratatui::layout::Rect,
    pub target_area: ratatui::layout::Rect,
    pub hide_mode: HideMode,
    pub field_mappings: HashMap<String, String>,
    pub show_mapping_popup: bool,
    pub mapping_popup_state: ListState,
    pub prefix_mappings: HashMap<String, String>, // source_prefix -> target_prefix
    pub show_prefix_popup: bool,
    pub prefix_popup_state: ListState,
    pub show_prefix_input: bool,
    pub prefix_input_source: String,
    pub prefix_input_target: String,
    pub prefix_input_field: usize, // 0 = source, 1 = target
    pub show_copy_mappings_popup: bool,
    pub copy_mappings_state: ListState,
    pub available_comparisons: Vec<String>, // entity:entity pairs that have mappings
    // New fields for enhanced functionality
    pub search_mode: bool,
    pub search_query: String,
    pub search_panel: FocusedPanel, // which panel to search in
    pub mapping_history: Vec<(String, String)>, // history for undo functionality (source, target)
    pub show_fuzzy_popup: bool,
    pub fuzzy_suggestions: Vec<String>,
    pub fuzzy_popup_state: ListState,
    pub selected_field_for_mapping: Option<String>, // field we're trying to map
}

impl CompareApp {
    pub fn new(
        source_fields: Vec<FieldInfo>,
        target_fields: Vec<FieldInfo>,
        source_entity_name: String,
        target_entity_name: String,
        source_env: String,
        target_env: String,
    ) -> Self {
        let mut app = Self {
            source_fields,
            target_fields,
            source_list_state: ListState::default(),
            target_list_state: ListState::default(),
            focused_panel: FocusedPanel::Source,
            quit: false,
            source_entity_name,
            target_entity_name,
            source_env,
            target_env,
            source_area: ratatui::layout::Rect::default(),
            target_area: ratatui::layout::Rect::default(),
            hide_mode: HideMode::HideMatches,
            field_mappings: HashMap::new(),
            show_mapping_popup: false,
            mapping_popup_state: ListState::default(),
            prefix_mappings: HashMap::new(),
            show_prefix_popup: false,
            prefix_popup_state: ListState::default(),
            show_prefix_input: false,
            prefix_input_source: String::new(),
            prefix_input_target: String::new(),
            prefix_input_field: 0,
            show_copy_mappings_popup: false,
            copy_mappings_state: ListState::default(),
            available_comparisons: Vec::new(),
            // Initialize new fields
            search_mode: false,
            search_query: String::new(),
            search_panel: FocusedPanel::Source,
            mapping_history: Vec::new(),
            show_fuzzy_popup: false,
            fuzzy_suggestions: Vec::new(),
            fuzzy_popup_state: ListState::default(),
            selected_field_for_mapping: None,
        };

        // Set initial selection
        app.source_list_state.select(Some(0));
        app.target_list_state.select(Some(0));

        app
    }

    pub fn load_field_mappings(&mut self, config: &Config) {
        if let Some(mappings) = config.get_field_mappings(&self.source_entity_name, &self.target_entity_name) {
            self.field_mappings = mappings.clone();
        }
    }

    pub fn load_prefix_mappings(&mut self, config: &Config) {
        if let Some(mappings) = config.get_prefix_mappings(&self.source_entity_name, &self.target_entity_name) {
            self.prefix_mappings = mappings.clone();
        }
    }

    pub fn load_available_comparisons(&mut self, config: &Config) {
        let current_comparison = format!("{}:{}", self.source_entity_name, self.target_entity_name);
        let mut comparisons = Vec::new();

        // Get all field mappings
        for (key, mappings) in config.list_field_mappings() {
            if key != &current_comparison && !mappings.is_empty() {
                comparisons.push(format!("{} (field mappings)", key.replace(':', " → ")));
            }
        }

        // Get all prefix mappings
        for (key, mappings) in config.list_prefix_mappings() {
            if key != &current_comparison && !mappings.is_empty() {
                let display_key = key.replace(':', " → ");
                if !comparisons.iter().any(|c| c.starts_with(&display_key)) {
                    comparisons.push(format!("{} (prefix mappings)", display_key));
                } else {
                    // Update existing entry to show both types
                    if let Some(existing) = comparisons.iter_mut().find(|c| c.starts_with(&display_key)) {
                        *existing = format!("{} (field + prefix mappings)", display_key);
                    }
                }
            }
        }

        comparisons.sort();
        let is_empty = comparisons.is_empty();
        self.available_comparisons = comparisons;
        self.copy_mappings_state.select(if is_empty { None } else { Some(0) });
    }

    pub fn copy_mappings_from(&mut self, source_comparison: &str, config: &mut Config) -> anyhow::Result<(usize, usize)> {
        // Extract the actual entity pair from the display string
        let entity_pair = if let Some(pos) = source_comparison.find(" (") {
            source_comparison[..pos].replace(" → ", ":")
        } else {
            return Err(anyhow::anyhow!("Invalid comparison format"));
        };

        // Split into source and target entities
        let parts: Vec<&str> = entity_pair.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid entity pair format"));
        }
        let (source_entity, target_entity) = (parts[0], parts[1]);

        let mut copied_field_mappings = 0;
        let mut copied_prefix_mappings = 0;

        // Copy field mappings
        if let Some(source_field_mappings) = config.get_field_mappings(source_entity, target_entity) {
            let mappings_to_copy: Vec<(String, String)> = source_field_mappings.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            for (source_field, target_field) in mappings_to_copy {
                self.field_mappings.insert(source_field.clone(), target_field.clone());
                config.add_field_mapping(
                    &self.source_entity_name,
                    &self.target_entity_name,
                    &source_field,
                    &target_field,
                )?;
                copied_field_mappings += 1;
            }
        }

        // Copy prefix mappings
        if let Some(source_prefix_mappings) = config.get_prefix_mappings(source_entity, target_entity) {
            let mappings_to_copy: Vec<(String, String)> = source_prefix_mappings.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            for (source_prefix, target_prefix) in mappings_to_copy {
                self.prefix_mappings.insert(source_prefix.clone(), target_prefix.clone());
                config.add_prefix_mapping(
                    &self.source_entity_name,
                    &self.target_entity_name,
                    &source_prefix,
                    &target_prefix,
                )?;
                copied_prefix_mappings += 1;
            }
        }

        Ok((copied_field_mappings, copied_prefix_mappings))
    }

    fn fields_match_with_prefix(&self, source_field: &str, target_field: &str) -> bool {
        // First check exact match
        if source_field == target_field {
            return true;
        }

        // Then check prefix-based matching
        for (source_prefix, target_prefix) in &self.prefix_mappings {
            if source_field.starts_with(source_prefix) && target_field.starts_with(target_prefix) {
                let source_without_prefix = &source_field[source_prefix.len()..];
                let target_without_prefix = &target_field[target_prefix.len()..];
                if source_without_prefix == target_without_prefix {
                    return true;
                }
            }
        }

        false
    }

    pub fn get_prefix_matched_target(&self, source_field: &str) -> Option<String> {
        for (source_prefix, target_prefix) in &self.prefix_mappings {
            if source_field.starts_with(source_prefix) {
                let source_without_prefix = &source_field[source_prefix.len()..];
                let target_field = format!("{}{}", target_prefix, source_without_prefix);

                // Check if this target field actually exists
                if self.target_fields.iter().any(|f| f.name == target_field) {
                    return Some(target_field);
                }
            }
        }
        None
    }

    pub fn get_prefix_matched_source(&self, target_field: &str) -> Option<String> {
        for (source_prefix, target_prefix) in &self.prefix_mappings {
            if target_field.starts_with(target_prefix) {
                let target_without_prefix = &target_field[target_prefix.len()..];
                let source_field = format!("{}{}", source_prefix, target_without_prefix);

                // Check if this source field actually exists
                if self.source_fields.iter().any(|f| f.name == source_field) {
                    return Some(source_field);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    fn debug_prefix_mappings(&self) {
        eprintln!("=== DEBUG: Prefix Mappings ===");
        for (source_prefix, target_prefix) in &self.prefix_mappings {
            eprintln!("  '{}' -> '{}'", source_prefix, target_prefix);
        }
        eprintln!("=== Total: {} mappings ===", self.prefix_mappings.len());
    }

    pub fn get_filtered_source_fields(&self) -> Vec<&FieldInfo> {
        let mut fields: Vec<&FieldInfo> = match self.hide_mode {
            HideMode::ShowAll => self.source_fields.iter().collect(),
            HideMode::HideMatches => {
                self.source_fields
                    .iter()
                    .filter(|field| {
                        // Check if field has exact match
                        let has_exact_match = self.target_fields
                            .iter()
                            .any(|target| target.name == field.name);

                        // Check if field has manual mapping
                        let has_manual_mapping = self.field_mappings.contains_key(&field.name);

                        // Check if field has prefix-based match
                        let has_prefix_match = self.target_fields
                            .iter()
                            .any(|target| self.fields_match_with_prefix(&field.name, &target.name));

                        // Hide if any type of match exists
                        !(has_exact_match || has_manual_mapping || has_prefix_match)
                    })
                    .collect()
            }
        };

        // Apply search filter if in search mode and searching source panel
        if self.search_mode && self.search_panel == FocusedPanel::Source && !self.search_query.is_empty() {
            fields = fields
                .into_iter()
                .filter(|field| field.name.to_lowercase().contains(&self.search_query.to_lowercase()))
                .collect();
        }

        fields
    }

    pub fn get_filtered_target_fields(&self) -> Vec<&FieldInfo> {
        let mut fields: Vec<&FieldInfo> = match self.hide_mode {
            HideMode::ShowAll => self.target_fields.iter().collect(),
            HideMode::HideMatches => {
                self.target_fields
                    .iter()
                    .filter(|field| {
                        // Check if field has exact match
                        let has_exact_match = self.source_fields
                            .iter()
                            .any(|source| source.name == field.name);

                        // Check if field has manual mapping (reverse lookup)
                        let has_manual_mapping = self.field_mappings.values().any(|v| v == &field.name);

                        // Check if field has prefix-based match
                        let has_prefix_match = self.source_fields
                            .iter()
                            .any(|source| self.fields_match_with_prefix(&source.name, &field.name));

                        // Hide if any type of match exists
                        !(has_exact_match || has_manual_mapping || has_prefix_match)
                    })
                    .collect()
            }
        };

        // Apply search filter if in search mode and searching target panel
        if self.search_mode && self.search_panel == FocusedPanel::Target && !self.search_query.is_empty() {
            fields = fields
                .into_iter()
                .filter(|field| field.name.to_lowercase().contains(&self.search_query.to_lowercase()))
                .collect();
        }

        fields
    }

    pub fn cycle_hide_mode(&mut self) {
        self.hide_mode = match self.hide_mode {
            HideMode::ShowAll => HideMode::HideMatches,
            HideMode::HideMatches => HideMode::ShowAll,
        };

        // Reset selections to ensure they're valid for the new filtered list
        let source_count = self.get_filtered_source_fields().len();
        let target_count = self.get_filtered_target_fields().len();

        if source_count > 0 {
            self.source_list_state.select(Some(0));
        } else {
            self.source_list_state.select(None);
        }

        if target_count > 0 {
            self.target_list_state.select(Some(0));
        } else {
            self.target_list_state.select(None);
        }
    }

    pub fn create_manual_mapping(&mut self) {
        if let (Some(source_idx), Some(target_idx)) = (
            self.source_list_state.selected(),
            self.target_list_state.selected(),
        ) {
            let source_fields = self.get_filtered_source_fields();
            let target_fields = self.get_filtered_target_fields();

            if let (Some(source_field), Some(target_field)) = (
                source_fields.get(source_idx),
                target_fields.get(target_idx),
            ) {
                let source_name = source_field.name.clone();
                let target_name = target_field.name.clone();

                // Add to mapping history for undo functionality
                self.mapping_history.push((source_name.clone(), target_name.clone()));

                // Add the mapping
                self.field_mappings.insert(source_name.clone(), target_name.clone());

                // Save to config
                if let Ok(mut config) = Config::load() {
                    let _ = config.add_field_mapping(
                        &self.source_entity_name,
                        &self.target_entity_name,
                        &source_name,
                        &target_name,
                    );
                }
            }
        }
    }

    // New helper methods for enhanced functionality
    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if self.search_mode {
            self.search_panel = self.focused_panel;
            self.search_query.clear();
        }
        // Reset list selections when entering/exiting search mode
        self.reset_list_selections();
    }

    pub fn reset_list_selections(&mut self) {
        let source_count = self.get_filtered_source_fields().len();
        let target_count = self.get_filtered_target_fields().len();

        if source_count > 0 {
            self.source_list_state.select(Some(0));
        } else {
            self.source_list_state.select(None);
        }

        if target_count > 0 {
            self.target_list_state.select(Some(0));
        } else {
            self.target_list_state.select(None);
        }
    }

    pub fn add_to_search_query(&mut self, c: char) {
        if self.search_mode {
            self.search_query.push(c);
            self.reset_list_selections();
        }
    }

    pub fn remove_from_search_query(&mut self) {
        if self.search_mode {
            self.search_query.pop();
            self.reset_list_selections();
        }
    }

    pub fn undo_last_mapping(&mut self) {
        if let Some((source_field, _target_field)) = self.mapping_history.pop() {
            self.field_mappings.remove(&source_field);

            // Also remove from config
            if let Ok(mut config) = Config::load() {
                let _ = config.remove_field_mapping(
                    &self.source_entity_name,
                    &self.target_entity_name,
                    &source_field,
                );
            }
        }
    }

    pub fn calculate_mapping_progress(&self) -> (usize, usize) {
        let total_mappable_fields = self.source_fields.len();
        let mut mapped_fields = 0;

        for source_field in &self.source_fields {
            // Check if field has exact match
            let has_exact_match = self.target_fields
                .iter()
                .any(|target| target.name == source_field.name);

            // Check if field has manual mapping
            let has_manual_mapping = self.field_mappings.contains_key(&source_field.name);

            // Check if field has prefix-based match
            let has_prefix_match = self.get_prefix_matched_target(&source_field.name).is_some();

            if has_exact_match || has_manual_mapping || has_prefix_match {
                mapped_fields += 1;
            }
        }

        (mapped_fields, total_mappable_fields)
    }

    pub fn get_current_field_name(&self) -> Option<String> {
        match self.focused_panel {
            FocusedPanel::Source => {
                let source_fields = self.get_filtered_source_fields();
                if let Some(idx) = self.source_list_state.selected() {
                    source_fields.get(idx).map(|f| f.name.clone())
                } else {
                    None
                }
            }
            FocusedPanel::Target => {
                let target_fields = self.get_filtered_target_fields();
                if let Some(idx) = self.target_list_state.selected() {
                    target_fields.get(idx).map(|f| f.name.clone())
                } else {
                    None
                }
            }
        }
    }

    pub fn start_fuzzy_mapping(&mut self) {
        if let Some(source_field) = self.get_current_field_name() {
            if self.focused_panel == FocusedPanel::Source {
                self.selected_field_for_mapping = Some(source_field.clone());
                self.generate_fuzzy_suggestions(&source_field);
                self.show_fuzzy_popup = true;
                self.fuzzy_popup_state.select(if self.fuzzy_suggestions.is_empty() { None } else { Some(0) });
            }
        }
    }

    pub fn generate_fuzzy_suggestions(&mut self, source_field: &str) {
        self.fuzzy_suggestions.clear();

        // Create a vector of (field_name, similarity_score) tuples
        let mut suggestions_with_scores: Vec<(String, f64)> = self.target_fields
            .iter()
            .filter(|target_field| {
                // Skip if already has manual mapping
                let has_manual_mapping = self.field_mappings.values().any(|v| v == &target_field.name);

                // Skip if has exact match
                let has_exact_match = self.source_fields.iter().any(|source| source.name == target_field.name);

                // Skip if has prefix match
                let has_prefix_match = self.source_fields
                    .iter()
                    .any(|source| self.fields_match_with_prefix(&source.name, &target_field.name));

                // Only include unmatched fields
                !(has_manual_mapping || has_exact_match || has_prefix_match)
            })
            .map(|target_field| {
                let similarity = calculate_similarity(source_field, &target_field.name);
                (target_field.name.clone(), similarity)
            })
            .collect();

        // Sort by similarity (highest first)
        suggestions_with_scores.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Convert to field names only for the fuzzy_suggestions vec (for compatibility with existing code)
        self.fuzzy_suggestions = suggestions_with_scores.into_iter().map(|(name, _)| name).collect();
    }

    pub fn get_fuzzy_match_percentage(&self, source_field: &str, target_field: &str) -> f64 {
        calculate_similarity(source_field, target_field) * 100.0
    }

    pub fn apply_fuzzy_mapping(&mut self) {
        if let (Some(source_field), Some(selected_idx)) = (
            &self.selected_field_for_mapping,
            self.fuzzy_popup_state.selected(),
        ) {
            if let Some(target_field) = self.fuzzy_suggestions.get(selected_idx) {
                // Add to mapping history for undo functionality
                self.mapping_history.push((source_field.clone(), target_field.clone()));

                // Add the mapping
                self.field_mappings.insert(source_field.clone(), target_field.clone());

                // Save to config
                if let Ok(mut config) = Config::load() {
                    let _ = config.add_field_mapping(
                        &self.source_entity_name,
                        &self.target_entity_name,
                        source_field,
                        target_field,
                    );
                }

                // Close popup and reset
                self.show_fuzzy_popup = false;
                self.selected_field_for_mapping = None;
                self.fuzzy_suggestions.clear();
            }
        }
    }

    pub fn cancel_fuzzy_mapping(&mut self) {
        self.show_fuzzy_popup = false;
        self.selected_field_for_mapping = None;
        self.fuzzy_suggestions.clear();
    }

    pub fn has_type_mismatch(&self, source_field: &str, target_field: &str) -> bool {
        let source_type = self.source_fields.iter()
            .find(|f| f.name == source_field)
            .map(|f| &f.field_type);
        let target_type = self.target_fields.iter()
            .find(|f| f.name == target_field)
            .map(|f| &f.field_type);

        match (source_type, target_type) {
            (Some(s), Some(t)) => !types_compatible(s, t),
            _ => false,
        }
    }

    pub fn copy_field_name_to_clipboard(&self) {
        if let Some(field_name) = self.get_current_field_name() {
            // In a real implementation, you would use a clipboard crate
            // For now, we'll just log the field name
            log::info!("Field name copied to clipboard: {}", field_name);
            eprintln!("Copied field name: {}", field_name);
        }
    }

    pub fn get_status_line_text(&self) -> String {
        match self.get_current_field_name() {
            Some(field_name) => {
                let field_info = match self.focused_panel {
                    FocusedPanel::Source => self.source_fields.iter().find(|f| f.name == field_name),
                    FocusedPanel::Target => self.target_fields.iter().find(|f| f.name == field_name),
                };

                if let Some(field) = field_info {
                    let panel_name = match self.focused_panel {
                        FocusedPanel::Source => "Source",
                        FocusedPanel::Target => "Target",
                    };

                    let required_text = if field.is_required { " [REQUIRED]" } else { "" };
                    let custom_text = if field.is_custom { " [CUSTOM]" } else { "" };

                    // Check for mappings
                    let mapping_info = if self.focused_panel == FocusedPanel::Source {
                        if let Some(target) = self.field_mappings.get(&field_name) {
                            format!(" → Mapped to: {}", target)
                        } else if self.target_fields.iter().any(|t| t.name == field_name) {
                            format!(" → Exact match available")
                        } else if let Some(prefix_match) = self.get_prefix_matched_target(&field_name) {
                            format!(" → Prefix match: {}", prefix_match)
                        } else {
                            " → Not mapped".to_string()
                        }
                    } else {
                        if let Some((source, _)) = self.field_mappings.iter().find(|(_, v)| *v == &field_name) {
                            format!(" ← Mapped from: {}", source)
                        } else if self.source_fields.iter().any(|s| s.name == field_name) {
                            format!(" ← Exact match available")
                        } else if let Some(prefix_match) = self.get_prefix_matched_source(&field_name) {
                            format!(" ← Prefix match: {}", prefix_match)
                        } else {
                            " ← Not mapped".to_string()
                        }
                    };

                    format!("{}: {} ({}){}{}{}",
                           panel_name, field_name, field.field_type,
                           required_text, custom_text, mapping_info)
                } else {
                    format!("Selected: {}", field_name)
                }
            }
            None => "No field selected".to_string(),
        }
    }

    pub fn export_to_excel_tui(&self) {
        // Generate a default filename based on entities and timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = if self.source_entity_name == self.target_entity_name {
            format!("{}_{}_to_{}_{}.xlsx", self.source_entity_name, self.source_env, self.target_env, timestamp)
        } else {
            format!("{}_to_{}_{}_to_{}_{}.xlsx", self.source_entity_name, self.target_entity_name, self.source_env, self.target_env, timestamp)
        };

        match self.export_to_excel_silent(&filename) {
            Ok(()) => {
                // Successfully exported - the export function will handle opening Excel
                // In a real implementation, you might want to show a success popup
            }
            Err(_) => {
                // In a real implementation, you might want to show an error popup
                // For now, this will just continue the TUI
            }
        }
    }
}

// Helper function to calculate similarity between two strings
fn calculate_similarity(a: &str, b: &str) -> f64 {
    let a = a.to_lowercase();
    let b = b.to_lowercase();

    // Simple Levenshtein distance-based similarity
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }

    let distance = levenshtein_distance(&a, &b);
    1.0 - (distance as f64 / max_len as f64)
}

// Simple Levenshtein distance implementation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];

    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }

    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[a_len][b_len]
}

// Helper function to check if two field types are compatible
fn types_compatible(type1: &str, type2: &str) -> bool {
    // Exact match
    if type1 == type2 {
        return true;
    }

    // Compatible numeric types
    let numeric_types = ["integer", "long", "decimal", "double"];
    if numeric_types.contains(&type1) && numeric_types.contains(&type2) {
        return true;
    }

    // String types are usually compatible with most things
    if type1 == "string" || type2 == "string" {
        return true;
    }

    // Navigation properties should be compatible with each other
    if (type1.contains("→") && type2.contains("→")) ||
       (type1.contains("nav") && type2.contains("nav")) {
        return true;
    }

    false
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchStatus {
    Exact,      // Identical
    Different,  // Present but different values
    Missing,    // Present in source but not target
    Added,      // Present in target but not source
}

#[derive(Debug, Clone)]
pub enum TreeNode {
    ViewRoot {
        name: String,
        view_type: String,
        is_custom: bool,
        expanded: bool,
    },
    Section {
        name: String,
        icon: String,
        count: usize,
        expanded: bool,
    },
    Column {
        name: String,
        alias: Option<String>,
        data_type: String,
        aggregate: Option<String>,
        match_status: MatchStatus,
    },
    Filter {
        attribute: String,
        operator: String,
        value: Option<String>,
        match_status: MatchStatus,
    },
    SortOrder {
        attribute: String,
        direction: String,
        match_status: MatchStatus,
    },
    FetchDetail {
        key: String,
        value: String,
        match_status: MatchStatus,
    },
}

impl TreeNode {
    pub fn can_expand(&self) -> bool {
        matches!(self, TreeNode::ViewRoot { .. } | TreeNode::Section { .. })
    }

    pub fn is_expanded(&self) -> bool {
        match self {
            TreeNode::ViewRoot { expanded, .. } | TreeNode::Section { expanded, .. } => *expanded,
            _ => false,
        }
    }

    pub fn toggle_expand(&mut self) {
        match self {
            TreeNode::ViewRoot { expanded, .. } | TreeNode::Section { expanded, .. } => {
                *expanded = !*expanded;
            }
            _ => {}
        }
    }

    pub fn get_display_text(&self) -> String {
        match self {
            TreeNode::ViewRoot { name, view_type, is_custom, .. } => {
                let custom_text = if *is_custom { " [custom]" } else { "" };
                format!("📋 {} ({}){}", name, view_type, custom_text)
            }
            TreeNode::Section { name, icon, count, .. } => {
                format!("{} {} ({})", icon, name, count)
            }
            TreeNode::Column { name, alias, data_type, aggregate, .. } => {
                let alias_text = alias.as_ref().map(|a| format!(" as {}", a)).unwrap_or_default();
                let agg_text = aggregate.as_ref().map(|a| format!(" [{}]", a)).unwrap_or_default();
                format!("{}{} ({}){}", name, alias_text, data_type, agg_text)
            }
            TreeNode::Filter { attribute, operator, value, .. } => {
                let value_text = value.as_ref().map(|v| format!(" {}", v)).unwrap_or_default();
                format!("{} {}{}", attribute, operator, value_text)
            }
            TreeNode::SortOrder { attribute, direction, .. } => {
                format!("{} ({})", attribute, direction)
            }
            TreeNode::FetchDetail { key, value, .. } => {
                format!("{}: {}", key, value)
            }
        }
    }

    pub fn get_match_status(&self) -> Option<MatchStatus> {
        match self {
            TreeNode::Column { match_status, .. } |
            TreeNode::Filter { match_status, .. } |
            TreeNode::SortOrder { match_status, .. } |
            TreeNode::FetchDetail { match_status, .. } => Some(match_status.clone()),
            _ => None,
        }
    }
}

// Hierarchical view comparison app
pub struct ViewCompareApp {
    pub source_structure: Option<ViewStructure>,
    pub target_structure: Option<ViewStructure>,
    pub source_tree: Vec<TreeNode>,
    pub target_tree: Vec<TreeNode>,
    pub source_list_state: ListState,
    pub target_list_state: ListState,
    pub focused_panel: FocusedPanel,
    pub quit: bool,
    pub source_view_name: String,
    pub target_view_name: String,
    pub source_env: String,
    pub target_env: String,
    pub source_area: ratatui::layout::Rect,
    pub target_area: ratatui::layout::Rect,
    pub hide_mode: HideMode,
    pub search_mode: bool,
    pub search_query: String,
    pub search_panel: FocusedPanel,
    pub result: Option<ViewCompareResult>,
}

impl ViewCompareApp {
    pub fn new(
        source_view: ViewInfo,
        target_view: ViewInfo,
        source_env: String,
        target_env: String,
    ) -> anyhow::Result<Self> {
        let source_structure = parse_view_structure(&source_view)?;
        let target_structure = parse_view_structure(&target_view)?;

        let mut app = Self {
            source_structure: Some(source_structure.clone()),
            target_structure: Some(target_structure.clone()),
            source_tree: Vec::new(),
            target_tree: Vec::new(),
            source_list_state: ListState::default(),
            target_list_state: ListState::default(),
            focused_panel: FocusedPanel::Source,
            quit: false,
            source_view_name: source_view.name,
            target_view_name: target_view.name,
            source_env,
            target_env,
            source_area: ratatui::layout::Rect::default(),
            target_area: ratatui::layout::Rect::default(),
            hide_mode: HideMode::ShowAll,
            search_mode: false,
            search_query: String::new(),
            search_panel: FocusedPanel::Source,
            result: None,
        };

        // Build trees with comparison analysis
        app.build_comparison_trees();

        // Set initial selection
        if !app.source_tree.is_empty() {
            app.source_list_state.select(Some(0));
        }
        if !app.target_tree.is_empty() {
            app.target_list_state.select(Some(0));
        }

        Ok(app)
    }

    fn build_comparison_trees(&mut self) {
        if let (Some(source), Some(target)) = (&self.source_structure, &self.target_structure) {
            self.source_tree = self.build_tree_for_structure(source, target, true);
            self.target_tree = self.build_tree_for_structure(target, source, false);
        }
    }

    fn build_tree_for_structure(&self, primary: &ViewStructure, comparison: &ViewStructure, _is_source: bool) -> Vec<TreeNode> {
        let mut tree = Vec::new();

        // View root
        tree.push(TreeNode::ViewRoot {
            name: primary.name.clone(),
            view_type: primary.view_type.clone(),
            is_custom: primary.is_custom,
            expanded: true,
        });

        // Columns section
        tree.push(TreeNode::Section {
            name: "Columns".to_string(),
            icon: "📊".to_string(),
            count: primary.columns.len(),
            expanded: true,
        });

        // Add columns with comparison analysis
        for column in &primary.columns {
            let match_status = self.compare_column(column, &comparison.columns);
            tree.push(TreeNode::Column {
                name: column.name.clone(),
                alias: column.alias.clone(),
                data_type: column.data_type.clone(),
                aggregate: column.aggregate.clone(),
                match_status,
            });
        }

        // Filters section
        tree.push(TreeNode::Section {
            name: "Filters".to_string(),
            icon: "🔍".to_string(),
            count: primary.filters.len(),
            expanded: true,
        });

        // Add filters with comparison analysis
        for filter in &primary.filters {
            let match_status = self.compare_filter(filter, &comparison.filters);
            tree.push(TreeNode::Filter {
                attribute: filter.attribute.clone(),
                operator: filter.operator.clone(),
                value: filter.value.clone(),
                match_status,
            });
        }

        // Sort orders section
        tree.push(TreeNode::Section {
            name: "Sort Orders".to_string(),
            icon: "🔀".to_string(),
            count: primary.sort_orders.len(),
            expanded: true,
        });

        // Add sort orders with comparison analysis
        for sort in &primary.sort_orders {
            let match_status = self.compare_sort_order(sort, &comparison.sort_orders);
            tree.push(TreeNode::SortOrder {
                attribute: sort.attribute.clone(),
                direction: sort.direction.clone(),
                match_status,
            });
        }

        // FetchXML details section
        tree.push(TreeNode::Section {
            name: "FetchXML Details".to_string(),
            icon: "📄".to_string(),
            count: 4, // Entity, Top Count, Distinct, No-lock
            expanded: true,
        });

        // Add fetch details with comparison
        tree.push(TreeNode::FetchDetail {
            key: "Entity".to_string(),
            value: primary.fetch_xml_details.entity.clone(),
            match_status: if primary.fetch_xml_details.entity == comparison.fetch_xml_details.entity {
                MatchStatus::Exact
            } else {
                MatchStatus::Different
            },
        });

        if let Some(top_count) = primary.fetch_xml_details.top_count {
            let match_status = match comparison.fetch_xml_details.top_count {
                Some(target_count) if target_count == top_count => MatchStatus::Exact,
                Some(_) => MatchStatus::Different,
                None => MatchStatus::Missing,
            };
            tree.push(TreeNode::FetchDetail {
                key: "Top Count".to_string(),
                value: top_count.to_string(),
                match_status,
            });
        }

        tree.push(TreeNode::FetchDetail {
            key: "Distinct".to_string(),
            value: primary.fetch_xml_details.distinct.to_string(),
            match_status: if primary.fetch_xml_details.distinct == comparison.fetch_xml_details.distinct {
                MatchStatus::Exact
            } else {
                MatchStatus::Different
            },
        });

        tree
    }

    fn compare_column(&self, column: &crate::dynamics::metadata::ViewColumnDetail, comparison_columns: &[crate::dynamics::metadata::ViewColumnDetail]) -> MatchStatus {
        // Find exact name match
        if let Some(target_column) = comparison_columns.iter().find(|c| c.name == column.name) {
            // Check if all properties match
            if column.alias == target_column.alias &&
               column.data_type == target_column.data_type &&
               column.aggregate == target_column.aggregate {
                MatchStatus::Exact
            } else {
                MatchStatus::Different
            }
        } else {
            MatchStatus::Missing
        }
    }

    fn compare_filter(&self, filter: &crate::dynamics::metadata::ViewFilter, comparison_filters: &[crate::dynamics::metadata::ViewFilter]) -> MatchStatus {
        // Find exact filter match
        if let Some(target_filter) = comparison_filters.iter().find(|f| f.attribute == filter.attribute) {
            if filter.operator == target_filter.operator && filter.value == target_filter.value {
                MatchStatus::Exact
            } else {
                MatchStatus::Different
            }
        } else {
            MatchStatus::Missing
        }
    }

    fn compare_sort_order(&self, sort: &crate::dynamics::metadata::ViewSortOrder, comparison_sorts: &[crate::dynamics::metadata::ViewSortOrder]) -> MatchStatus {
        // Find exact sort match
        if let Some(target_sort) = comparison_sorts.iter().find(|s| s.attribute == sort.attribute) {
            if sort.direction == target_sort.direction {
                MatchStatus::Exact
            } else {
                MatchStatus::Different
            }
        } else {
            MatchStatus::Missing
        }
    }

    pub fn get_visible_source_nodes(&self) -> Vec<(usize, &TreeNode)> {
        self.get_visible_nodes(&self.source_tree)
    }

    pub fn get_visible_target_nodes(&self) -> Vec<(usize, &TreeNode)> {
        self.get_visible_nodes(&self.target_tree)
    }

    pub fn create_tree_items_for(tree: &[TreeNode]) -> Vec<ratatui::widgets::ListItem> {
        let visible_nodes = Self::get_visible_nodes_static(tree);
        visible_nodes
            .iter()
            .map(|(depth, node)| {
                crate::commands::migration::ui::render_tree_node(node, *depth)
            })
            .collect()
    }

    fn get_visible_nodes_static(tree: &[TreeNode]) -> Vec<(usize, &TreeNode)> {
        let mut visible = Vec::new();
        let mut depth = 0;
        let mut parent_expanded = true;

        for (_i, node) in tree.iter().enumerate() {
            match node {
                TreeNode::ViewRoot { expanded, .. } => {
                    visible.push((depth, node));
                    parent_expanded = *expanded;
                    depth = 1;
                }
                TreeNode::Section { expanded, .. } => {
                    if parent_expanded {
                        visible.push((depth, node));
                        if *expanded {
                            depth += 1;
                        }
                    }
                }
                TreeNode::Column { .. } | TreeNode::Filter { .. } | TreeNode::SortOrder { .. } | TreeNode::FetchDetail { .. } => {
                    if parent_expanded && depth > 1 {
                        visible.push((depth, node));
                    }
                }
            }
        }

        visible
    }

    fn get_visible_nodes<'a>(&self, tree: &'a [TreeNode]) -> Vec<(usize, &'a TreeNode)> {
        let mut visible = Vec::new();
        let mut depth = 0;
        let mut parent_expanded = true;

        for (_i, node) in tree.iter().enumerate() {
            match node {
                TreeNode::ViewRoot { expanded, .. } => {
                    visible.push((depth, node));
                    parent_expanded = *expanded;
                    depth = 1;
                }
                TreeNode::Section { expanded, .. } => {
                    if parent_expanded {
                        visible.push((depth, node));
                        if !*expanded {
                            // Skip children if section is collapsed
                            let _section_type = match node {
                                TreeNode::Section { name, .. } => name.as_str(),
                                _ => "",
                            };
                            // Skip nodes until next section or view root
                            continue;
                        }
                    }
                }
                _ => {
                    if parent_expanded {
                        // Apply filters
                        let should_show = match self.hide_mode {
                            HideMode::ShowAll => true,
                            HideMode::HideMatches => {
                                if let Some(status) = node.get_match_status() {
                                    !matches!(status, MatchStatus::Exact)
                                } else {
                                    true
                                }
                            }
                        };

                        // Apply search filter
                        let search_match = if self.search_mode && !self.search_query.is_empty() {
                            node.get_display_text().to_lowercase().contains(&self.search_query.to_lowercase())
                        } else {
                            true
                        };

                        if should_show && search_match {
                            visible.push((depth + 1, node));
                        }
                    }
                }
            }
        }

        visible
    }

    pub fn cycle_hide_mode(&mut self) {
        self.hide_mode = match self.hide_mode {
            HideMode::ShowAll => HideMode::HideMatches,
            HideMode::HideMatches => HideMode::ShowAll,
        };
        self.reset_list_selections();
    }

    pub fn toggle_search_mode(&mut self) {
        self.search_mode = !self.search_mode;
        if self.search_mode {
            self.search_panel = self.focused_panel;
            self.search_query.clear();
        }
        self.reset_list_selections();
    }

    pub fn reset_list_selections(&mut self) {
        let source_count = self.get_visible_source_nodes().len();
        let target_count = self.get_visible_target_nodes().len();

        if source_count > 0 {
            self.source_list_state.select(Some(0));
        } else {
            self.source_list_state.select(None);
        }

        if target_count > 0 {
            self.target_list_state.select(Some(0));
        } else {
            self.target_list_state.select(None);
        }
    }

    pub fn add_to_search_query(&mut self, c: char) {
        if self.search_mode {
            self.search_query.push(c);
            self.reset_list_selections();
        }
    }

    pub fn remove_from_search_query(&mut self) {
        if self.search_mode {
            self.search_query.pop();
            self.reset_list_selections();
        }
    }

    pub fn toggle_expand(&mut self) {
        let selected_idx = match self.focused_panel {
            FocusedPanel::Source => self.source_list_state.selected(),
            FocusedPanel::Target => self.target_list_state.selected(),
        };

        if let Some(idx) = selected_idx {
            // Get node name/identifier for matching
            let node_identifier = match self.focused_panel {
                FocusedPanel::Source => {
                    let visible_nodes = self.get_visible_source_nodes();
                    visible_nodes.get(idx).and_then(|(_, node)| {
                        match node {
                            TreeNode::ViewRoot { name, .. } => Some(("root", name.clone())),
                            TreeNode::Section { name, .. } => Some(("section", name.clone())),
                            _ => None,
                        }
                    })
                }
                FocusedPanel::Target => {
                    let visible_nodes = self.get_visible_target_nodes();
                    visible_nodes.get(idx).and_then(|(_, node)| {
                        match node {
                            TreeNode::ViewRoot { name, .. } => Some(("root", name.clone())),
                            TreeNode::Section { name, .. } => Some(("section", name.clone())),
                            _ => None,
                        }
                    })
                }
            };

            if let Some((node_type, name)) = node_identifier {
                let tree = match self.focused_panel {
                    FocusedPanel::Source => &mut self.source_tree,
                    FocusedPanel::Target => &mut self.target_tree,
                };

                // Find and toggle the matching node
                if let Some(tree_node) = tree.iter_mut().find(|n| {
                    match (node_type, n) {
                        ("root", TreeNode::ViewRoot { name: n1, .. }) => n1 == &name,
                        ("section", TreeNode::Section { name: n1, .. }) => n1 == &name,
                        _ => false,
                    }
                }) {
                    tree_node.toggle_expand();
                }
            }
        }
    }

    pub fn get_current_node(&self) -> Option<&TreeNode> {
        let visible_nodes = match self.focused_panel {
            FocusedPanel::Source => self.get_visible_source_nodes(),
            FocusedPanel::Target => self.get_visible_target_nodes(),
        };

        let selected_idx = match self.focused_panel {
            FocusedPanel::Source => self.source_list_state.selected(),
            FocusedPanel::Target => self.target_list_state.selected(),
        };

        if let Some(idx) = selected_idx {
            visible_nodes.get(idx).map(|(_, node)| *node)
        } else {
            None
        }
    }

    pub fn get_status_line_text(&self) -> String {
        match self.get_current_node() {
            Some(node) => {
                let panel_name = match self.focused_panel {
                    FocusedPanel::Source => "Source",
                    FocusedPanel::Target => "Target",
                };

                let display_text = node.get_display_text();
                let match_status = match node.get_match_status() {
                    Some(MatchStatus::Exact) => " [EXACT MATCH]",
                    Some(MatchStatus::Different) => " [DIFFERENT]",
                    Some(MatchStatus::Missing) => " [MISSING]",
                    Some(MatchStatus::Added) => " [ADDED]",
                    None => "",
                };

                format!("{}: {}{}", panel_name, display_text, match_status)
            }
            None => "No item selected".to_string(),
        }
    }

    pub fn calculate_comparison_progress(&self) -> (usize, usize, usize, usize) {
        let mut exact_matches = 0;
        let mut differences = 0;
        let mut missing = 0;
        let mut total = 0;

        // Count items in source tree (excluding root and sections)
        for node in &self.source_tree {
            if let Some(status) = node.get_match_status() {
                total += 1;
                match status {
                    MatchStatus::Exact => exact_matches += 1,
                    MatchStatus::Different => differences += 1,
                    MatchStatus::Missing => missing += 1,
                    MatchStatus::Added => {} // Don't count in source
                }
            }
        }

        (exact_matches, differences, missing, total)
    }

    pub fn next(&mut self) {
        match self.focused_panel {
            FocusedPanel::Source => {
                let source_nodes = self.get_visible_source_nodes();
                if let Some(selected) = self.source_list_state.selected() {
                    if selected < source_nodes.len().saturating_sub(1) {
                        self.source_list_state.select(Some(selected + 1));
                    }
                }
            }
            FocusedPanel::Target => {
                let target_nodes = self.get_visible_target_nodes();
                if let Some(selected) = self.target_list_state.selected() {
                    if selected < target_nodes.len().saturating_sub(1) {
                        self.target_list_state.select(Some(selected + 1));
                    }
                }
            }
        }
    }

    pub fn previous(&mut self) {
        match self.focused_panel {
            FocusedPanel::Source => {
                if let Some(selected) = self.source_list_state.selected() {
                    if selected > 0 {
                        self.source_list_state.select(Some(selected - 1));
                    }
                }
            }
            FocusedPanel::Target => {
                if let Some(selected) = self.target_list_state.selected() {
                    if selected > 0 {
                        self.target_list_state.select(Some(selected - 1));
                    }
                }
            }
        }
    }
}