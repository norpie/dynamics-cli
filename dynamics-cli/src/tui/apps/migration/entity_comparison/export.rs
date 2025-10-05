use anyhow::{Context, Result};
use chrono::Utc;
use rust_xlsxwriter::*;
use std::process::Command;

use super::app::State;
use super::models::{MatchInfo, MatchType};
use crate::api::metadata::{FieldMetadata, FieldType, RelationshipMetadata};
use crate::tui::Resource;

/// Excel export functionality for migration analysis
pub struct MigrationExporter;

impl MigrationExporter {
    /// Export migration analysis to Excel file and auto-open
    pub fn export_and_open(state: &State, file_path: &str) -> Result<()> {
        Self::export_to_excel(state, file_path)?;
        Self::try_open_file(file_path);
        Ok(())
    }

    /// Export migration analysis to Excel file
    pub fn export_to_excel(state: &State, file_path: &str) -> Result<()> {
        let mut workbook = Workbook::new();

        // Create all worksheets
        Self::create_source_entity_sheet(&mut workbook, state)?;
        Self::create_target_entity_sheet(&mut workbook, state)?;
        Self::create_source_relationships_sheet(&mut workbook, state)?;
        Self::create_target_relationships_sheet(&mut workbook, state)?;
        Self::create_source_views_sheet(&mut workbook, state)?;
        Self::create_target_views_sheet(&mut workbook, state)?;
        Self::create_source_forms_sheet(&mut workbook, state)?;
        Self::create_target_forms_sheet(&mut workbook, state)?;
        Self::create_source_entities_sheet(&mut workbook, state)?;
        Self::create_target_entities_sheet(&mut workbook, state)?;
        Self::create_examples_sheet(&mut workbook, state)?;
        Self::create_source_examples_sheet(&mut workbook, state)?;
        Self::create_target_examples_sheet(&mut workbook, state)?;

        workbook
            .save(file_path)
            .with_context(|| format!("Failed to save Excel file: {}", file_path))?;

        log::info!("Excel file exported to: {}", file_path);
        Ok(())
    }

    /// Create source entity detail sheet with mapping information
    fn create_source_entity_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Entity")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} ({})", state.source_entity, state.source_env),
            &title_format,
        )?;

        // Headers
        let headers = ["Field Name", "Type", "Required", "Primary Key", "Mapped To", "Mapping Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let type_mismatch_format = Self::create_type_mismatch_format();
        let unmapped_format = Self::create_unmapped_format();
        let required_format = Self::create_required_format();
        let indent_format = Format::new().set_indent(1);

        // Get source fields
        let source_fields = match &state.source_metadata {
            Resource::Success(metadata) => &metadata.fields,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        // Group fields by mapping status
        let (mapped_fields, unmapped_fields): (Vec<_>, Vec<_>) = source_fields
            .iter()
            .partition(|field| state.field_matches.contains_key(&field.logical_name));

        // Mapped Fields Section
        if !mapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "✓ MAPPED FIELDS", &header_format)?;
            row += 1;

            // Group by match type
            let exact_matches: Vec<_> = mapped_fields
                .iter()
                .filter(|f| {
                    state.field_matches.get(&f.logical_name)
                        .map(|m| m.match_type == MatchType::Exact)
                        .unwrap_or(false)
                })
                .collect();

            let manual_mappings: Vec<_> = mapped_fields
                .iter()
                .filter(|f| {
                    state.field_matches.get(&f.logical_name)
                        .map(|m| m.match_type == MatchType::Manual)
                        .unwrap_or(false)
                })
                .collect();

            let prefix_matches: Vec<_> = mapped_fields
                .iter()
                .filter(|f| {
                    state.field_matches.get(&f.logical_name)
                        .map(|m| m.match_type == MatchType::Prefix)
                        .unwrap_or(false)
                })
                .collect();

            let type_mismatches: Vec<_> = mapped_fields
                .iter()
                .filter(|f| {
                    state.field_matches.get(&f.logical_name)
                        .map(|m| m.match_type == MatchType::TypeMismatch)
                        .unwrap_or(false)
                })
                .collect();

            // Exact Matches
            if !exact_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Exact Name + Type Matches", &Format::new().set_bold())?;
                row += 1;

                for field in exact_matches {
                    if let Some(match_info) = state.field_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, &match_info.target_field, "Exact", &exact_match_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            // Manual Mappings
            if !manual_mappings.is_empty() {
                sheet.write_string_with_format(row, 0, "  Manual Mappings", &Format::new().set_bold())?;
                row += 1;

                for field in manual_mappings {
                    if let Some(match_info) = state.field_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, &match_info.target_field, "Manual", &manual_mapping_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            // Prefix Matches
            if !prefix_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Prefix Matches", &Format::new().set_bold())?;
                row += 1;

                for field in prefix_matches {
                    if let Some(match_info) = state.field_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, &match_info.target_field, "Prefix", &prefix_match_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            // Type Mismatches
            if !type_mismatches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Type Mismatches", &Format::new().set_bold().set_font_color(Color::RGB(0xFF8C00)))?;
                row += 1;

                for field in type_mismatches {
                    if let Some(match_info) = state.field_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, &match_info.target_field, "Type Mismatch", &type_mismatch_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }
        }

        // Unmapped Fields Section
        if !unmapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "⚠ UNMAPPED FIELDS", &header_format)?;
            row += 1;

            // Group unmapped by characteristics
            let required_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_required).collect();
            let primary_key_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_primary_key && !f.is_required).collect();
            let other_fields: Vec<_> = unmapped_fields.iter().filter(|f| !f.is_required && !f.is_primary_key).collect();

            // Required Unmapped
            if !required_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Required Fields (Need Attention)", &Format::new().set_bold().set_font_color(Color::Red))?;
                row += 1;
                for field in required_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &required_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Primary Keys
            if !primary_key_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Primary Key Fields", &Format::new().set_bold())?;
                row += 1;
                for field in primary_key_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Other Fields
            if !other_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Other Fields", &Format::new().set_bold())?;
                row += 1;
                for field in other_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target entity detail sheet with mapping information
    fn create_target_entity_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Entity")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} ({})", state.target_entity, state.target_env),
            &title_format,
        )?;

        let headers = ["Field Name", "Type", "Required", "Primary Key", "Mapped From", "Mapping Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let type_mismatch_format = Self::create_type_mismatch_format();
        let unmapped_format = Self::create_unmapped_format();
        let required_format = Self::create_required_format();
        let indent_format = Format::new().set_indent(1);

        let target_fields = match &state.target_metadata {
            Resource::Success(metadata) => &metadata.fields,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        // Reverse lookup: find source fields that map to each target field
        let mut reverse_matches: std::collections::HashMap<String, (String, MatchInfo)> = std::collections::HashMap::new();
        for (source_field, match_info) in &state.field_matches {
            reverse_matches.insert(match_info.target_field.clone(), (source_field.clone(), match_info.clone()));
        }

        let (mapped_fields, unmapped_fields): (Vec<_>, Vec<_>) = target_fields
            .iter()
            .partition(|field| reverse_matches.contains_key(&field.logical_name));

        // Mapped Fields Section
        if !mapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "✓ MAPPED FIELDS", &header_format)?;
            row += 1;

            let exact_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                reverse_matches.get(&f.logical_name)
                    .map(|(_, m)| m.match_type == MatchType::Exact)
                    .unwrap_or(false)
            }).collect();

            let manual_mappings: Vec<_> = mapped_fields.iter().filter(|f| {
                reverse_matches.get(&f.logical_name)
                    .map(|(_, m)| m.match_type == MatchType::Manual)
                    .unwrap_or(false)
            }).collect();

            let prefix_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                reverse_matches.get(&f.logical_name)
                    .map(|(_, m)| m.match_type == MatchType::Prefix)
                    .unwrap_or(false)
            }).collect();

            let type_mismatches: Vec<_> = mapped_fields.iter().filter(|f| {
                reverse_matches.get(&f.logical_name)
                    .map(|(_, m)| m.match_type == MatchType::TypeMismatch)
                    .unwrap_or(false)
            }).collect();

            if !exact_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Exact Name + Type Matches", &Format::new().set_bold())?;
                row += 1;
                for field in exact_matches {
                    if let Some((source_name, _)) = reverse_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, source_name, "Exact", &exact_match_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            if !manual_mappings.is_empty() {
                sheet.write_string_with_format(row, 0, "  Manual Mappings", &Format::new().set_bold())?;
                row += 1;
                for field in manual_mappings {
                    if let Some((source_name, _)) = reverse_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, source_name, "Manual", &manual_mapping_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            if !prefix_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Prefix Matches", &Format::new().set_bold())?;
                row += 1;
                for field in prefix_matches {
                    if let Some((source_name, _)) = reverse_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, source_name, "Prefix", &prefix_match_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            if !type_mismatches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Type Mismatches", &Format::new().set_bold().set_font_color(Color::RGB(0xFF8C00)))?;
                row += 1;
                for field in type_mismatches {
                    if let Some((source_name, _)) = reverse_matches.get(&field.logical_name) {
                        Self::write_field_row(sheet, row, field, source_name, "Type Mismatch", &type_mismatch_format, &indent_format)?;
                        row += 1;
                    }
                }
                row += 1;
            }
        }

        // Unmapped Fields Section
        if !unmapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "⚠ UNMAPPED FIELDS", &header_format)?;
            row += 1;

            let required_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_required).collect();
            let primary_key_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_primary_key && !f.is_required).collect();
            let other_fields: Vec<_> = unmapped_fields.iter().filter(|f| !f.is_required && !f.is_primary_key).collect();

            if !required_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Required Fields (Need Attention)", &Format::new().set_bold().set_font_color(Color::Red))?;
                row += 1;
                for field in required_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &required_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            if !primary_key_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Primary Key Fields", &Format::new().set_bold())?;
                row += 1;
                for field in primary_key_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            if !other_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Other Fields", &Format::new().set_bold())?;
                row += 1;
                for field in other_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source relationships sheet using RelationshipMetadata
    fn create_source_relationships_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Relationships")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Relationships", state.source_entity),
            &title_format,
        )?;

        let headers = ["Relationship Name", "Related Entity", "Type", "Schema Name"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let relationship_format = Self::create_relationship_format();
        let indent_format = Format::new().set_indent(1);

        let relationships = match &state.source_metadata {
            Resource::Success(metadata) => &metadata.relationships,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        if relationships.is_empty() {
            sheet.write_string(row, 0, "No relationship fields found")?;
        } else {
            sheet.write_string_with_format(row, 0, "🔗 RELATIONSHIPS", &header_format)?;
            row += 1;

            for rel in relationships {
                sheet.write_string_with_format(row, 0, &format!("    {}", rel.name), &indent_format)?;
                sheet.write_string_with_format(row, 1, &rel.related_entity, &relationship_format)?;
                sheet.write_string_with_format(row, 2, &format!("{:?}", rel.relationship_type), &relationship_format)?;
                sheet.write_string_with_format(row, 3, &rel.related_attribute, &relationship_format)?;
                row += 1;
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target relationships sheet
    fn create_target_relationships_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Relationships")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Relationships", state.target_entity),
            &title_format,
        )?;

        let headers = ["Relationship Name", "Related Entity", "Type", "Schema Name"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let relationship_format = Self::create_relationship_format();
        let indent_format = Format::new().set_indent(1);

        let relationships = match &state.target_metadata {
            Resource::Success(metadata) => &metadata.relationships,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        if relationships.is_empty() {
            sheet.write_string(row, 0, "No relationship fields found")?;
        } else {
            sheet.write_string_with_format(row, 0, "🔗 RELATIONSHIPS", &header_format)?;
            row += 1;

            for rel in relationships {
                sheet.write_string_with_format(row, 0, &format!("    {}", rel.name), &indent_format)?;
                sheet.write_string_with_format(row, 1, &rel.related_entity, &relationship_format)?;
                sheet.write_string_with_format(row, 2, &format!("{:?}", rel.relationship_type), &relationship_format)?;
                sheet.write_string_with_format(row, 3, &rel.related_attribute, &relationship_format)?;
                row += 1;
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source views sheet with column structure
    fn create_source_views_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Views")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Views Structure", state.source_entity),
            &title_format,
        )?;

        let headers = ["Item", "Type", "Properties", "Width/Primary"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let views = match &state.source_metadata {
            Resource::Success(metadata) => &metadata.views,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        for view in views {
            sheet.write_string_with_format(row, 0, &view.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, "View")?;
            sheet.write_string(row, 2, &format!("Type: {}, Columns: {}", view.view_type, view.columns.len()))?;
            row += 1;

            for column in &view.columns {
                sheet.write_string_with_format(row, 0, &column.name, &indent_format)?;
                sheet.write_string(row, 1, "Column")?;
                sheet.write_string(row, 2, "")?;
                let width_info = if let Some(width) = column.width {
                    format!("Width: {}, Primary: {}", width, column.is_primary)
                } else {
                    format!("Width: Auto, Primary: {}", column.is_primary)
                };
                sheet.write_string(row, 3, &width_info)?;
                row += 1;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target views sheet
    fn create_target_views_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Views")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Views Structure", state.target_entity),
            &title_format,
        )?;

        let headers = ["Item", "Type", "Properties", "Width/Primary"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let views = match &state.target_metadata {
            Resource::Success(metadata) => &metadata.views,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        for view in views {
            sheet.write_string_with_format(row, 0, &view.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, "View")?;
            sheet.write_string(row, 2, &format!("Type: {}, Columns: {}", view.view_type, view.columns.len()))?;
            row += 1;

            for column in &view.columns {
                sheet.write_string_with_format(row, 0, &column.name, &indent_format)?;
                sheet.write_string(row, 1, "Column")?;
                sheet.write_string(row, 2, "")?;
                let width_info = if let Some(width) = column.width {
                    format!("Width: {}, Primary: {}", width, column.is_primary)
                } else {
                    format!("Width: Auto, Primary: {}", column.is_primary)
                };
                sheet.write_string(row, 3, &width_info)?;
                row += 1;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source forms sheet with nested structure
    fn create_source_forms_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Forms")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);
        let indent2_format = Format::new().set_indent(2);
        let indent3_format = Format::new().set_indent(3);

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Forms Structure", state.source_entity),
            &title_format,
        )?;

        let headers = ["Item", "Type", "Properties", "Order/Position"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let forms = match &state.source_metadata {
            Resource::Success(metadata) => &metadata.forms,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        for form in forms {
            sheet.write_string_with_format(row, 0, &form.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, &form.form_type)?;
            row += 1;

            if let Some(structure) = &form.form_structure {
                for tab in &structure.tabs {
                    sheet.write_string_with_format(row, 0, &tab.label, &indent_format)?;
                    sheet.write_string(row, 1, "Tab")?;
                    sheet.write_string(row, 2, &format!("Visible: {}, Expanded: {}", tab.visible, tab.expanded))?;
                    sheet.write_string(row, 3, &tab.order.to_string())?;
                    row += 1;

                    for section in &tab.sections {
                        sheet.write_string_with_format(row, 0, &section.label, &indent2_format)?;
                        sheet.write_string(row, 1, "Section")?;
                        sheet.write_string(row, 2, &format!("Columns: {}, Visible: {}", section.columns, section.visible))?;
                        sheet.write_string(row, 3, &section.order.to_string())?;
                        row += 1;

                        for field in &section.fields {
                            sheet.write_string_with_format(row, 0, &field.label, &indent3_format)?;
                            sheet.write_string(row, 1, "Field")?;
                            sheet.write_string(row, 2, &format!(
                                "LogicalName: {}, Required: {}, ReadOnly: {}, Visible: {}",
                                field.logical_name, field.required_level, field.readonly, field.visible
                            ))?;
                            sheet.write_string(row, 3, &format!("Row: {}, Col: {}", field.row, field.column))?;
                            row += 1;
                        }
                    }
                }
            } else {
                sheet.write_string_with_format(row, 0, "No structure data", &indent_format)?;
                row += 1;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target forms sheet
    fn create_target_forms_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Forms")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);
        let indent2_format = Format::new().set_indent(2);
        let indent3_format = Format::new().set_indent(3);

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Forms Structure", state.target_entity),
            &title_format,
        )?;

        let headers = ["Item", "Type", "Properties", "Order/Position"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let forms = match &state.target_metadata {
            Resource::Success(metadata) => &metadata.forms,
            _ => {
                sheet.write_string(row, 0, "No metadata loaded")?;
                sheet.autofit();
                return Ok(());
            }
        };

        for form in forms {
            sheet.write_string_with_format(row, 0, &form.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, &form.form_type)?;
            row += 1;

            if let Some(structure) = &form.form_structure {
                for tab in &structure.tabs {
                    sheet.write_string_with_format(row, 0, &tab.label, &indent_format)?;
                    sheet.write_string(row, 1, "Tab")?;
                    sheet.write_string(row, 2, &format!("Visible: {}, Expanded: {}", tab.visible, tab.expanded))?;
                    sheet.write_string(row, 3, &tab.order.to_string())?;
                    row += 1;

                    for section in &tab.sections {
                        sheet.write_string_with_format(row, 0, &section.label, &indent2_format)?;
                        sheet.write_string(row, 1, "Section")?;
                        sheet.write_string(row, 2, &format!("Columns: {}, Visible: {}", section.columns, section.visible))?;
                        sheet.write_string(row, 3, &section.order.to_string())?;
                        row += 1;

                        for field in &section.fields {
                            sheet.write_string_with_format(row, 0, &field.label, &indent3_format)?;
                            sheet.write_string(row, 1, "Field")?;
                            sheet.write_string(row, 2, &format!(
                                "LogicalName: {}, Required: {}, ReadOnly: {}, Visible: {}",
                                field.logical_name, field.required_level, field.readonly, field.visible
                            ))?;
                            sheet.write_string(row, 3, &format!("Row: {}, Col: {}", field.row, field.column))?;
                            row += 1;
                        }
                    }
                }
            } else {
                sheet.write_string_with_format(row, 0, "No structure data", &indent_format)?;
                row += 1;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source entities sheet (NEW - not in legacy version)
    fn create_source_entities_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Entities")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Related Entities", state.source_entity),
            &title_format,
        )?;

        let headers = ["Entity Name", "Usage Count", "Mapped To", "Match Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let unmapped_format = Self::create_unmapped_format();
        let indent_format = Format::new().set_indent(1);

        if state.source_entities.is_empty() {
            sheet.write_string(row, 0, "No related entities found")?;
        } else {
            let (mapped_entities, unmapped_entities): (Vec<_>, Vec<_>) = state.source_entities
                .iter()
                .partition(|(entity_name, _)| {
                    let entity_id = format!("entity_{}", entity_name);
                    state.entity_matches.contains_key(&entity_id)
                });

            // Mapped Entities
            if !mapped_entities.is_empty() {
                sheet.write_string_with_format(row, 0, "✓ MAPPED ENTITIES", &header_format)?;
                row += 1;

                for (entity_name, usage_count) in mapped_entities {
                    let entity_id = format!("entity_{}", entity_name);
                    if let Some(match_info) = state.entity_matches.get(&entity_id) {
                        let (mapping_type, format) = match match_info.match_type {
                            MatchType::Exact => ("Exact", &exact_match_format),
                            MatchType::Manual => ("Manual", &manual_mapping_format),
                            MatchType::Prefix => ("Prefix", &prefix_match_format),
                            MatchType::TypeMismatch => ("Type Mismatch", &unmapped_format),
                        };

                        sheet.write_string_with_format(row, 0, &format!("    {}", entity_name), &indent_format)?;
                        sheet.write_string_with_format(row, 1, &usage_count.to_string(), format)?;
                        sheet.write_string_with_format(row, 2, &match_info.target_field, format)?;
                        sheet.write_string_with_format(row, 3, mapping_type, format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            // Unmapped Entities
            if !unmapped_entities.is_empty() {
                sheet.write_string_with_format(row, 0, "⚠ UNMAPPED ENTITIES", &header_format)?;
                row += 1;

                for (entity_name, usage_count) in unmapped_entities {
                    sheet.write_string_with_format(row, 0, &format!("    {}", entity_name), &indent_format)?;
                    sheet.write_string_with_format(row, 1, &usage_count.to_string(), &unmapped_format)?;
                    sheet.write_string_with_format(row, 2, "", &unmapped_format)?;
                    sheet.write_string_with_format(row, 3, "Unmapped", &unmapped_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target entities sheet (NEW - not in legacy version)
    fn create_target_entities_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Entities")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Related Entities", state.target_entity),
            &title_format,
        )?;

        let headers = ["Entity Name", "Usage Count", "Mapped From", "Match Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let unmapped_format = Self::create_unmapped_format();
        let indent_format = Format::new().set_indent(1);

        // Reverse lookup for entities
        let mut reverse_entity_matches: std::collections::HashMap<String, (String, MatchInfo)> = std::collections::HashMap::new();
        for (source_entity_id, match_info) in &state.entity_matches {
            let target_entity_name = match_info.target_field.strip_prefix("entity_").unwrap_or(&match_info.target_field);
            reverse_entity_matches.insert(target_entity_name.to_string(), (source_entity_id.clone(), match_info.clone()));
        }

        if state.target_entities.is_empty() {
            sheet.write_string(row, 0, "No related entities found")?;
        } else {
            let (mapped_entities, unmapped_entities): (Vec<_>, Vec<_>) = state.target_entities
                .iter()
                .partition(|(entity_name, _)| reverse_entity_matches.contains_key(entity_name));

            // Mapped Entities
            if !mapped_entities.is_empty() {
                sheet.write_string_with_format(row, 0, "✓ MAPPED ENTITIES", &header_format)?;
                row += 1;

                for (entity_name, usage_count) in mapped_entities {
                    if let Some((source_id, match_info)) = reverse_entity_matches.get(entity_name) {
                        let source_name = source_id.strip_prefix("entity_").unwrap_or(source_id);
                        let (mapping_type, format) = match match_info.match_type {
                            MatchType::Exact => ("Exact", &exact_match_format),
                            MatchType::Manual => ("Manual", &manual_mapping_format),
                            MatchType::Prefix => ("Prefix", &prefix_match_format),
                            MatchType::TypeMismatch => ("Type Mismatch", &unmapped_format),
                        };

                        sheet.write_string_with_format(row, 0, &format!("    {}", entity_name), &indent_format)?;
                        sheet.write_string_with_format(row, 1, &usage_count.to_string(), format)?;
                        sheet.write_string_with_format(row, 2, source_name, format)?;
                        sheet.write_string_with_format(row, 3, mapping_type, format)?;
                        row += 1;
                    }
                }
                row += 1;
            }

            // Unmapped Entities
            if !unmapped_entities.is_empty() {
                sheet.write_string_with_format(row, 0, "⚠ UNMAPPED ENTITIES", &header_format)?;
                row += 1;

                for (entity_name, usage_count) in unmapped_entities {
                    sheet.write_string_with_format(row, 0, &format!("    {}", entity_name), &indent_format)?;
                    sheet.write_string_with_format(row, 1, &usage_count.to_string(), &unmapped_format)?;
                    sheet.write_string_with_format(row, 2, "", &unmapped_format)?;
                    sheet.write_string_with_format(row, 3, "Unmapped", &unmapped_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    // Placeholder for examples sheets - will implement separately due to size
    fn create_examples_sheet(_workbook: &mut Workbook, _state: &State) -> Result<()> {
        Ok(())
    }

    fn create_source_examples_sheet(_workbook: &mut Workbook, _state: &State) -> Result<()> {
        Ok(())
    }

    fn create_target_examples_sheet(_workbook: &mut Workbook, _state: &State) -> Result<()> {
        Ok(())
    }

    /// Write a field row with consistent formatting
    fn write_field_row(
        sheet: &mut Worksheet,
        row: u32,
        field: &FieldMetadata,
        target_field_name: &str,
        mapping_type: &str,
        row_format: &Format,
        indent_format: &Format,
    ) -> Result<()> {
        let required_format = Self::create_required_format();

        let name_format = if field.is_required { &required_format } else { indent_format };
        let required_cell_format = if field.is_required { &required_format } else { row_format };

        sheet.write_string_with_format(row, 0, &format!("    {}", field.logical_name), name_format)?;
        sheet.write_string_with_format(row, 1, &format!("{:?}", field.field_type), row_format)?;
        sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
        sheet.write_string_with_format(row, 3, if field.is_primary_key { "Yes" } else { "No" }, row_format)?;
        sheet.write_string_with_format(row, 4, target_field_name, row_format)?;
        sheet.write_string_with_format(row, 5, mapping_type, row_format)?;

        Ok(())
    }

    // Formatting helpers
    fn create_header_format() -> Format {
        Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White)
    }

    fn create_title_format() -> Format {
        Format::new()
            .set_bold()
            .set_font_size(16)
    }

    fn create_exact_match_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0x90EE90))  // Light Green
    }

    fn create_manual_mapping_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0x87CEEB))  // Sky Blue
    }

    fn create_prefix_match_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFE4B5))  // Moccasin/Light Orange
    }

    fn create_type_mismatch_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFD700))  // Gold/Yellow
    }

    fn create_unmapped_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFB6C1))  // Light Pink
    }

    fn create_required_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFA07A))  // Light Salmon
    }

    fn create_relationship_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xDDA0DD))  // Plum/Light Purple
    }

    fn create_custom_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0x20B2AA))  // Light Sea Green
            .set_font_color(Color::White)
    }

    fn create_values_match_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0x90EE90))  // Light Green
    }

    fn create_values_differ_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFB6C1))  // Light Pink
    }

    fn create_missing_data_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xD3D3D3))  // Light Gray
    }

    /// Try to open the Excel file with appropriate application
    fn try_open_file(file_path: &str) {
        let result = if cfg!(target_os = "windows") {
            // Windows: use cmd /c start to open with default Excel application
            Command::new("cmd")
                .args(["/c", "start", "", file_path])  // Empty string after start is for window title
                .spawn()
        } else if cfg!(target_os = "macos") {
            // macOS: use open command
            Command::new("open")
                .arg(file_path)
                .spawn()
        } else if cfg!(target_os = "linux") {
            // Linux: Try LibreOffice Calc first, then onlyoffice-desktopeditors, then fallback to xdg-open
            Command::new("libreoffice")
                .args(["--calc", file_path])
                .spawn()
                .or_else(|_| {
                    Command::new("onlyoffice-desktopeditors")
                        .arg(file_path)
                        .spawn()
                })
                .or_else(|_| {
                    Command::new("xdg-open")
                        .arg(file_path)
                        .spawn()
                })
        } else {
            // Fallback for other platforms
            Command::new("xdg-open")
                .arg(file_path)
                .spawn()
        };

        match result {
            Ok(_) => {
                log::info!("Opening Excel file: {}", file_path);
            }
            Err(e) => {
                log::warn!("Could not auto-open file: {}. File saved at: {}", e, file_path);
            }
        }
    }
}
