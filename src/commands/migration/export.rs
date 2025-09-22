use anyhow::{Context, Result};
use chrono::Utc;
use rust_xlsxwriter::*;
use std::collections::HashMap;
use std::process::Command;
use crate::config::Config;
use crate::dynamics::metadata::FieldInfo;

use super::app::CompareApp;

pub struct ExportData {
    pub source_entity: String,
    pub target_entity: String,
    pub source_env: String,
    pub target_env: String,
    pub source_fields: Vec<FieldInfo>,
    pub target_fields: Vec<FieldInfo>,
    pub field_mappings: HashMap<String, String>,
    pub prefix_mappings: HashMap<String, String>,
    pub config: Config,
}

impl CompareApp {
    pub fn export_to_excel(&self, file_path: &str) -> Result<()> {
        self.export_to_excel_with_options(file_path, true)
    }

    pub fn export_to_excel_silent(&self, file_path: &str) -> Result<()> {
        self.export_to_excel_with_options(file_path, false)
    }

    fn export_to_excel_with_options(&self, file_path: &str, print_messages: bool) -> Result<()> {
        let export_data = self.prepare_export_data()?;

        let mut workbook = Workbook::new();

        // Create all sheets
        self.create_executive_summary(&mut workbook, &export_data)?;
        self.create_field_mappings_sheet(&mut workbook, &export_data)?;
        self.create_unmapped_fields_sheet(&mut workbook, &export_data)?;
        self.create_type_mismatches_sheet(&mut workbook, &export_data)?;
        self.create_source_entity_sheet(&mut workbook, &export_data)?;
        self.create_target_entity_sheet(&mut workbook, &export_data)?;
        self.create_mapping_rules_sheet(&mut workbook, &export_data)?;

        workbook.save(file_path)
            .with_context(|| format!("Failed to save Excel file: {}", file_path))?;

        // Try to open the Excel file if Excel is available
        self.try_open_excel_file(file_path, print_messages);

        Ok(())
    }

    fn prepare_export_data(&self) -> Result<ExportData> {
        let config = Config::load()?;

        // Get field mappings for this entity comparison
        let field_mappings = config
            .get_field_mappings(&self.source_entity_name, &self.target_entity_name)
            .cloned()
            .unwrap_or_default();

        let prefix_mappings = config
            .get_prefix_mappings(&self.source_entity_name, &self.target_entity_name)
            .cloned()
            .unwrap_or_default();

        Ok(ExportData {
            source_entity: self.source_entity_name.clone(),
            target_entity: self.target_entity_name.clone(),
            source_env: self.source_env.clone(),
            target_env: self.target_env.clone(),
            source_fields: self.source_fields.clone(),
            target_fields: self.target_fields.clone(),
            field_mappings,
            prefix_mappings,
            config,
        })
    }

    fn create_executive_summary(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Executive Summary")?;

        // Create formats
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White)
            .set_font_size(14);

        let title_format = Format::new()
            .set_bold()
            .set_font_size(16);

        let metric_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0xF2F2F2));

        // Title
        sheet.write_string_with_format(0, 0, "Migration Analysis Report", &title_format)?;
        sheet.write_string(1, 0, &format!("Generated: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")))?;

        // Migration overview
        sheet.write_string_with_format(3, 0, "Migration Overview", &header_format)?;
        sheet.write_string(4, 0, "Source Entity:")?;
        sheet.write_string(4, 1, &data.source_entity)?;
        sheet.write_string(5, 0, "Target Entity:")?;
        sheet.write_string(5, 1, &data.target_entity)?;
        sheet.write_string(6, 0, "Source Environment:")?;
        sheet.write_string(6, 1, &data.source_env)?;
        sheet.write_string(7, 0, "Target Environment:")?;
        sheet.write_string(7, 1, &data.target_env)?;

        // Calculate metrics
        let (mapped_count, total_source_fields) = self.calculate_mapping_progress();
        let unmapped_count = total_source_fields - mapped_count;
        let type_mismatches = self.count_type_mismatches(data);
        let progress_percentage = if total_source_fields > 0 {
            (mapped_count as f64 / total_source_fields as f64) * 100.0
        } else {
            0.0
        };

        // Key metrics
        sheet.write_string_with_format(9, 0, "Key Metrics", &header_format)?;
        sheet.write_string_with_format(10, 0, "Total Source Fields:", &metric_format)?;
        sheet.write_number(10, 1, total_source_fields as f64)?;
        sheet.write_string_with_format(11, 0, "Mapped Fields:", &metric_format)?;
        sheet.write_number(11, 1, mapped_count as f64)?;
        sheet.write_string_with_format(12, 0, "Unmapped Fields:", &metric_format)?;
        sheet.write_number(12, 1, unmapped_count as f64)?;
        sheet.write_string_with_format(13, 0, "Type Mismatches:", &metric_format)?;
        sheet.write_number(13, 1, type_mismatches as f64)?;
        sheet.write_string_with_format(14, 0, "Progress:", &metric_format)?;
        sheet.write_string(14, 1, &format!("{:.1}%", progress_percentage))?;

        // Summary information only - no subjective risk assessment

        // Auto-size columns
        sheet.autofit();

        Ok(())
    }

    fn create_field_mappings_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Field Mappings")?;

        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White);

        let exact_format = Format::new()
            .set_background_color(Color::RGB(0x90EE90));

        let manual_format = Format::new()
            .set_background_color(Color::RGB(0xFFFFAA));

        let prefix_format = Format::new()
            .set_background_color(Color::RGB(0xADD8E6));

        let mismatch_format = Format::new()
            .set_background_color(Color::RGB(0xFFB6C1));

        // Headers
        let headers = ["Source Field", "Target Field", "Mapping Type", "Source Type", "Target Type", "Status", "Notes"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(0, col as u16, *header, &header_format)?;
        }

        let mut row = 1u32;

        // Process all source fields
        for source_field in &data.source_fields {
            let mapped_target = self.get_mapped_target_field(&source_field.name, data);
            let (mapping_type, target_field_name, target_field_info) = if let Some(target_name) = mapped_target {
                let target_info = data.target_fields.iter().find(|f| f.name == target_name);
                if source_field.name == target_name {
                    ("Exact", target_name, target_info)
                } else if data.field_mappings.contains_key(&source_field.name) {
                    ("Manual", target_name, target_info)
                } else {
                    ("Prefix", target_name, target_info)
                }
            } else {
                ("Unmapped", String::new(), None)
            };

            let row_format = match mapping_type {
                "Exact" => &exact_format,
                "Manual" => &manual_format,
                "Prefix" => &prefix_format,
                _ => &Format::new(),
            };

            sheet.write_string_with_format(row, 0, &source_field.name, row_format)?;
            sheet.write_string_with_format(row, 1, &target_field_name, row_format)?;
            sheet.write_string_with_format(row, 2, mapping_type, row_format)?;
            sheet.write_string_with_format(row, 3, &source_field.field_type, row_format)?;

            if let Some(target_info) = target_field_info {
                let type_format = if source_field.field_type != target_info.field_type {
                    &mismatch_format
                } else {
                    row_format
                };
                sheet.write_string_with_format(row, 4, &target_info.field_type, type_format)?;

                let status = if source_field.field_type != target_info.field_type {
                    "TYPE MISMATCH"
                } else {
                    "OK"
                };
                sheet.write_string_with_format(row, 5, status, type_format)?;
            } else if mapping_type == "Unmapped" {
                sheet.write_string_with_format(row, 4, "", row_format)?;
                sheet.write_string_with_format(row, 5, "UNMAPPED", &mismatch_format)?;
            }

            // Notes column - add any special indicators
            let mut notes = Vec::new();
            if source_field.is_required {
                notes.push("Required");
            }
            if source_field.is_custom {
                notes.push("Custom");
            }
            sheet.write_string_with_format(row, 6, &notes.join(", "), row_format)?;

            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    fn create_unmapped_fields_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Unmapped Fields")?;

        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White);

        let required_format = Format::new()
            .set_background_color(Color::RGB(0xFFB6C1))
            .set_bold();

        // Unmapped source fields
        sheet.write_string_with_format(0, 0, "Unmapped Source Fields", &header_format)?;
        let source_headers = ["Field Name", "Type", "Required", "Custom", "Suggested Matches"];
        for (col, header) in source_headers.iter().enumerate() {
            sheet.write_string_with_format(1, col as u16, *header, &header_format)?;
        }

        let mut row = 2u32;
        for source_field in &data.source_fields {
            if self.get_mapped_target_field(&source_field.name, data).is_none() {
                let row_format = if source_field.is_required { &required_format } else { &Format::new() };

                sheet.write_string_with_format(row, 0, &source_field.name, row_format)?;
                sheet.write_string_with_format(row, 1, &source_field.field_type, row_format)?;
                sheet.write_string_with_format(row, 2, if source_field.is_required { "Yes" } else { "No" }, row_format)?;
                sheet.write_string_with_format(row, 3, if source_field.is_custom { "Yes" } else { "No" }, row_format)?;

                // Generate fuzzy suggestions
                let suggestions = self.generate_fuzzy_suggestions_for_export(&source_field.name, &data.target_fields);
                let suggestions_text = suggestions.iter()
                    .take(3)
                    .map(|s| format!("{} ({}%)", s.0, s.1))
                    .collect::<Vec<_>>()
                    .join(", ");
                sheet.write_string_with_format(row, 4, &suggestions_text, row_format)?;

                row += 1;
            }
        }

        // Add some spacing
        row += 2;

        // Unmapped target fields
        sheet.write_string_with_format(row, 0, "Unmapped Target Fields", &header_format)?;
        row += 1;
        let target_headers = ["Field Name", "Type", "Required", "Custom", "Notes"];
        for (col, header) in target_headers.iter().enumerate() {
            sheet.write_string_with_format(row, col as u16, *header, &header_format)?;
        }
        row += 1;

        for target_field in &data.target_fields {
            if !self.is_target_field_mapped(&target_field.name, data) {
                let row_format = if target_field.is_required { &required_format } else { &Format::new() };

                sheet.write_string_with_format(row, 0, &target_field.name, row_format)?;
                sheet.write_string_with_format(row, 1, &target_field.field_type, row_format)?;
                sheet.write_string_with_format(row, 2, if target_field.is_required { "Yes" } else { "No" }, row_format)?;
                sheet.write_string_with_format(row, 3, if target_field.is_custom { "Yes" } else { "No" }, row_format)?;

                let notes = if target_field.is_required {
                    "Requires data mapping"
                } else {
                    "Optional field"
                };
                sheet.write_string_with_format(row, 4, notes, row_format)?;

                row += 1;
            }
        }

        sheet.autofit();
        Ok(())
    }

    fn create_type_mismatches_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Type Mismatches")?;

        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White);

        let headers = ["Source Field", "Target Field", "Source Type", "Target Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(0, col as u16, *header, &header_format)?;
        }

        let mut row = 1u32;

        for source_field in &data.source_fields {
            if let Some(target_name) = self.get_mapped_target_field(&source_field.name, data) {
                if let Some(target_field) = data.target_fields.iter().find(|f| f.name == target_name) {
                    if source_field.field_type != target_field.field_type {
                        // Just show the mismatch data without subjective risk assessment
                        let format = &Format::new();

                        sheet.write_string_with_format(row, 0, &source_field.name, format)?;
                        sheet.write_string_with_format(row, 1, &target_field.name, format)?;
                        sheet.write_string_with_format(row, 2, &source_field.field_type, format)?;
                        sheet.write_string_with_format(row, 3, &target_field.field_type, format)?;

                        row += 1;
                    }
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    fn create_source_entity_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let mut sheet = workbook.add_worksheet();
        sheet.set_name("Source Entity Detail")?;

        self.create_entity_detail_sheet(&mut sheet, &data.source_fields, &format!("{} ({})", data.source_entity, data.source_env), true, data)?;
        Ok(())
    }

    fn create_target_entity_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let mut sheet = workbook.add_worksheet();
        sheet.set_name("Target Entity Detail")?;

        self.create_entity_detail_sheet(&mut sheet, &data.target_fields, &format!("{} ({})", data.target_entity, data.target_env), false, data)?;
        Ok(())
    }

    fn create_entity_detail_sheet(&self, sheet: &mut Worksheet, fields: &[FieldInfo], title: &str, is_source: bool, data: &ExportData) -> Result<()> {
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White);

        let title_format = Format::new()
            .set_bold()
            .set_font_size(14);

        let mapped_format = Format::new()
            .set_background_color(Color::RGB(0x90EE90));

        let unmapped_format = Format::new()
            .set_background_color(Color::RGB(0xFFB6C1));

        sheet.write_string_with_format(0, 0, title, &title_format)?;

        let headers = ["Field Name", "Type", "Required", "Custom", "Description", "Mapped Status"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        for field in fields {
            let is_mapped = if is_source {
                self.get_mapped_target_field(&field.name, data).is_some()
            } else {
                self.is_target_field_mapped(&field.name, data)
            };

            let row_format = if is_mapped { &mapped_format } else { &unmapped_format };

            sheet.write_string_with_format(row, 0, &field.name, row_format)?;
            sheet.write_string_with_format(row, 1, &field.field_type, row_format)?;
            sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, row_format)?;
            sheet.write_string_with_format(row, 3, if field.is_custom { "Yes" } else { "No" }, row_format)?;
            sheet.write_string_with_format(row, 4, "", row_format)?; // Description not available in FieldInfo
            sheet.write_string_with_format(row, 5, if is_mapped { "Mapped" } else { "Unmapped" }, row_format)?;

            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    fn create_mapping_rules_sheet(&self, workbook: &mut Workbook, data: &ExportData) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Mapping Rules")?;

        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White);

        let section_format = Format::new()
            .set_bold()
            .set_font_size(12);

        let mut row = 0u32;

        // Prefix mappings
        sheet.write_string_with_format(row, 0, "Prefix Mapping Rules", &section_format)?;
        row += 1;

        if !data.prefix_mappings.is_empty() {
            sheet.write_string_with_format(row, 0, "Source Prefix", &header_format)?;
            sheet.write_string_with_format(row, 1, "Target Prefix", &header_format)?;
            row += 1;

            for (source_prefix, target_prefix) in &data.prefix_mappings {
                sheet.write_string(row, 0, source_prefix)?;
                sheet.write_string(row, 1, target_prefix)?;
                row += 1;
            }
        } else {
            sheet.write_string(row, 0, "No prefix mappings defined")?;
            row += 1;
        }

        row += 2;

        // Manual mappings
        sheet.write_string_with_format(row, 0, "Manual Field Mappings", &section_format)?;
        row += 1;

        if !data.field_mappings.is_empty() {
            sheet.write_string_with_format(row, 0, "Source Field", &header_format)?;
            sheet.write_string_with_format(row, 1, "Target Field", &header_format)?;
            row += 1;

            for (source_field, target_field) in &data.field_mappings {
                sheet.write_string(row, 0, source_field)?;
                sheet.write_string(row, 1, target_field)?;
                row += 1;
            }
        } else {
            sheet.write_string(row, 0, "No manual mappings defined")?;
            row += 1;
        }

        row += 2;

        // Configuration settings
        sheet.write_string_with_format(row, 0, "Configuration Settings", &section_format)?;
        row += 1;

        sheet.write_string_with_format(row, 0, "Setting", &header_format)?;
        sheet.write_string_with_format(row, 1, "Value", &header_format)?;
        row += 1;

        sheet.write_string(row, 0, "Default Query Limit")?;
        sheet.write_number(row, 1, data.config.get_settings().default_query_limit as f64)?;
        row += 1;

        sheet.write_string(row, 0, "Export Generated")?;
        sheet.write_string(row, 1, &Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string())?;

        sheet.autofit();
        Ok(())
    }

    // Helper methods

    fn get_mapped_target_field(&self, source_field: &str, data: &ExportData) -> Option<String> {
        // Check manual mappings first
        if let Some(target) = data.field_mappings.get(source_field) {
            return Some(target.clone());
        }

        // Check prefix mappings
        for (source_prefix, target_prefix) in &data.prefix_mappings {
            if source_field.starts_with(source_prefix) {
                let suffix = &source_field[source_prefix.len()..];
                let target_field = format!("{}{}", target_prefix, suffix);
                if data.target_fields.iter().any(|f| f.name == target_field) {
                    return Some(target_field);
                }
            }
        }

        // Check exact matches
        if data.target_fields.iter().any(|f| f.name == source_field) {
            return Some(source_field.to_string());
        }

        None
    }

    fn is_target_field_mapped(&self, target_field: &str, data: &ExportData) -> bool {
        // Check if any source field maps to this target field
        for source_field in &data.source_fields {
            if let Some(mapped_target) = self.get_mapped_target_field(&source_field.name, data) {
                if mapped_target == target_field {
                    return true;
                }
            }
        }
        false
    }

    fn count_type_mismatches(&self, data: &ExportData) -> usize {
        let mut count = 0;
        for source_field in &data.source_fields {
            if let Some(target_name) = self.get_mapped_target_field(&source_field.name, data) {
                if let Some(target_field) = data.target_fields.iter().find(|f| f.name == target_name) {
                    if source_field.field_type != target_field.field_type {
                        count += 1;
                    }
                }
            }
        }
        count
    }


    fn generate_fuzzy_suggestions_for_export(&self, source_field: &str, target_fields: &[FieldInfo]) -> Vec<(String, u32)> {
        target_fields
            .iter()
            .map(|target| {
                let similarity = self.calculate_similarity(source_field, &target.name);
                (target.name.clone(), similarity)
            })
            .filter(|(_, similarity)| *similarity > 0)
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .into_iter()
            .map(|(name, similarity)| (name, similarity))
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    fn calculate_similarity(&self, a: &str, b: &str) -> u32 {
        // Simple similarity calculation (you might want to use the same algorithm as in your fuzzy matching)
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if a_lower == b_lower {
            return 100;
        }

        // Calculate Levenshtein distance and convert to similarity percentage
        let distance = levenshtein_distance(&a_lower, &b_lower);
        let max_len = a_lower.len().max(b_lower.len());

        if max_len == 0 {
            100
        } else {
            ((max_len - distance) * 100 / max_len) as u32
        }
    }

    fn try_open_excel_file(&self, file_path: &str, print_messages: bool) {
        let result = if cfg!(target_os = "windows") {
            // On Windows, try excel.exe first, then fallback to default association
            Command::new("excel.exe")
                .arg(file_path)
                .spawn()
                .or_else(|_| {
                    // Fallback to default file association
                    Command::new("cmd")
                        .args(["/C", "start", "", file_path])
                        .spawn()
                })
        } else if cfg!(target_os = "macos") {
            // On macOS, try Excel first, then fallback to default association
            Command::new("open")
                .args(["-a", "Microsoft Excel", file_path])
                .spawn()
                .or_else(|_| {
                    // Fallback to default file association
                    Command::new("open")
                        .arg(file_path)
                        .spawn()
                })
        } else {
            // On Linux, try common Excel alternatives, then fallback to xdg-open
            Command::new("libreoffice")
                .args(["--calc", file_path])
                .spawn()
                .or_else(|_| {
                    Command::new("gnumeric")
                        .arg(file_path)
                        .spawn()
                })
                .or_else(|_| {
                    Command::new("xdg-open")
                        .arg(file_path)
                        .spawn()
                })
        };

        if print_messages {
            match result {
                Ok(_) => {
                    // Successfully opened the file
                    println!("Opening Excel file...");
                }
                Err(_) => {
                    // Failed to open, but don't error out - just inform the user
                    println!("Excel file saved to: {}", file_path);
                    println!("To open: double-click the file or use your preferred spreadsheet application");
                }
            }
        }
    }
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

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