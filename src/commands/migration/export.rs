use anyhow::{Context, Result};
use chrono::Utc;
use rust_xlsxwriter::*;
use std::process::Command;

use crate::{
    commands::migration::ui::screens::comparison::data_models::{ComparisonData, SharedState},
};

/// Excel export functionality for migration analysis
pub struct MigrationExporter;

impl MigrationExporter {
    /// Export migration analysis to Excel file and auto-open
    pub fn export_and_open(
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
        file_path: &str,
    ) -> Result<()> {
        Self::export_to_excel(comparison_data, shared_state, file_path)?;
        Self::try_open_file(file_path);
        Ok(())
    }

    /// Export migration analysis to Excel file
    pub fn export_to_excel(
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
        file_path: &str,
    ) -> Result<()> {
        let mut workbook = Workbook::new();

        // Create all worksheets
        Self::create_source_entity_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_target_entity_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_source_relationships_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_target_relationships_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_source_views_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_target_views_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_source_forms_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_target_forms_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_examples_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_source_examples_sheet(&mut workbook, comparison_data, shared_state)?;
        Self::create_target_examples_sheet(&mut workbook, comparison_data, shared_state)?;

        workbook
            .save(file_path)
            .with_context(|| format!("Failed to save Excel file: {}", file_path))?;

        log::info!("Excel file exported to: {}", file_path);
        Ok(())
    }

    /// Create source entity detail sheet with mapping information
    fn create_source_entity_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Entity")?;

        let header_format = Self::create_header_format();
        let mapped_format = Self::create_mapped_format();
        let unmapped_format = Self::create_unmapped_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} ({})", comparison_data.source_entity, comparison_data.source_env),
            &title_format,
        )?;

        // Headers
        let headers = ["Field Name", "Type", "Required", "Custom", "Mapped To", "Mapping Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let indent_format = Format::new().set_indent(1);

        // Group fields by category
        let (mapped_fields, unmapped_fields): (Vec<_>, Vec<_>) = comparison_data.source_fields
            .iter()
            .partition(|field| Self::get_mapped_target_field(&field.name, comparison_data, shared_state).is_some());

        // Mapped Fields Section
        if !mapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "✓ MAPPED FIELDS", &header_format)?;
            row += 1;

            // Group mapped fields by mapping type
            let exact_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                Self::get_mapped_target_field(&f.name, comparison_data, shared_state)
                    .as_ref() == Some(&f.name)
            }).collect();

            let manual_mappings: Vec<_> = mapped_fields.iter().filter(|f| {
                shared_state.field_mappings.contains_key(&f.name)
            }).collect();

            let prefix_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                let mapped_target = Self::get_mapped_target_field(&f.name, comparison_data, shared_state);
                mapped_target.is_some() &&
                mapped_target.as_ref() != Some(&f.name) &&
                !shared_state.field_mappings.contains_key(&f.name)
            }).collect();

            // Exact Matches
            if !exact_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Exact Name Matches", &Format::new().set_bold())?;
                row += 1;

                for field in exact_matches {
                    let target_field_name = Self::get_mapped_target_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_field_row(sheet, row, field, &target_field_name, "Exact", &exact_match_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Manual Mappings
            if !manual_mappings.is_empty() {
                sheet.write_string_with_format(row, 0, "  Manual Mappings", &Format::new().set_bold())?;
                row += 1;

                for field in manual_mappings {
                    let target_field_name = Self::get_mapped_target_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_field_row(sheet, row, field, &target_field_name, "Manual", &manual_mapping_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Prefix Matches
            if !prefix_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Prefix Matches", &Format::new().set_bold())?;
                row += 1;

                for field in prefix_matches {
                    let target_field_name = Self::get_mapped_target_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_field_row(sheet, row, field, &target_field_name, "Prefix", &prefix_match_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }
        }

        // Unmapped Fields Section
        if !unmapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "⚠ UNMAPPED FIELDS", &header_format)?;
            row += 1;

            // Group unmapped by field characteristics
            let relationship_fields: Vec<_> = unmapped_fields.iter().filter(|f| Self::is_relationship_field(f)).collect();
            let required_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_required && !Self::is_relationship_field(f)).collect();
            let custom_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_custom && !f.is_required && !Self::is_relationship_field(f)).collect();
            let standard_fields: Vec<_> = unmapped_fields.iter().filter(|f| !f.is_custom && !f.is_required && !Self::is_relationship_field(f)).collect();

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

            // Relationship Fields
            if !relationship_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Relationship/Lookup Fields", &Format::new().set_bold())?;
                row += 1;
                for field in relationship_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Custom Fields
            if !custom_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Custom Fields", &Format::new().set_bold())?;
                row += 1;
                for field in custom_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &custom_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Standard Fields
            if !standard_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Standard Fields", &Format::new().set_bold())?;
                row += 1;
                for field in standard_fields {
                    Self::write_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target entity detail sheet with mapping information
    fn create_target_entity_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Entity")?;

        let header_format = Self::create_header_format();
        let mapped_format = Self::create_mapped_format();
        let unmapped_format = Self::create_unmapped_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} ({})", comparison_data.target_entity, comparison_data.target_env),
            &title_format,
        )?;

        // Headers
        let headers = ["Field Name", "Type", "Required", "Custom", "Mapped From", "Mapping Type"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let exact_match_format = Self::create_exact_match_format();
        let manual_mapping_format = Self::create_manual_mapping_format();
        let prefix_match_format = Self::create_prefix_match_format();
        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let indent_format = Format::new().set_indent(1);

        // Group fields by category
        let (mapped_fields, unmapped_fields): (Vec<_>, Vec<_>) = comparison_data.target_fields
            .iter()
            .partition(|field| Self::get_mapped_source_field(&field.name, comparison_data, shared_state).is_some());

        // Mapped Fields Section
        if !mapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "✓ MAPPED FIELDS", &header_format)?;
            row += 1;

            // Group mapped fields by mapping type
            let exact_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                Self::get_mapped_source_field(&f.name, comparison_data, shared_state)
                    .as_ref() == Some(&f.name)
            }).collect();

            let manual_mappings: Vec<_> = mapped_fields.iter().filter(|f| {
                shared_state.field_mappings.values().any(|v| v == &f.name)
            }).collect();

            let prefix_matches: Vec<_> = mapped_fields.iter().filter(|f| {
                let mapped_from = Self::get_mapped_source_field(&f.name, comparison_data, shared_state);
                mapped_from.is_some() &&
                mapped_from.as_ref() != Some(&f.name) &&
                !shared_state.field_mappings.values().any(|v| v == &f.name)
            }).collect();

            // Exact Matches
            if !exact_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Exact Name Matches", &Format::new().set_bold())?;
                row += 1;

                for field in exact_matches {
                    let source_field_name = Self::get_mapped_source_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_target_field_row(sheet, row, field, &source_field_name, "Exact", &exact_match_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Manual Mappings
            if !manual_mappings.is_empty() {
                sheet.write_string_with_format(row, 0, "  Manual Mappings", &Format::new().set_bold())?;
                row += 1;

                for field in manual_mappings {
                    let source_field_name = Self::get_mapped_source_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_target_field_row(sheet, row, field, &source_field_name, "Manual", &manual_mapping_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Prefix Matches
            if !prefix_matches.is_empty() {
                sheet.write_string_with_format(row, 0, "  Prefix Matches", &Format::new().set_bold())?;
                row += 1;

                for field in prefix_matches {
                    let source_field_name = Self::get_mapped_source_field(&field.name, comparison_data, shared_state).unwrap_or_default();
                    Self::write_target_field_row(sheet, row, field, &source_field_name, "Prefix", &prefix_match_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }
        }

        // Unmapped Fields Section
        if !unmapped_fields.is_empty() {
            sheet.write_string_with_format(row, 0, "⚠ UNMAPPED FIELDS", &header_format)?;
            row += 1;

            // Group unmapped by field characteristics
            let relationship_fields: Vec<_> = unmapped_fields.iter().filter(|f| Self::is_relationship_field(f)).collect();
            let required_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_required && !Self::is_relationship_field(f)).collect();
            let custom_fields: Vec<_> = unmapped_fields.iter().filter(|f| f.is_custom && !f.is_required && !Self::is_relationship_field(f)).collect();
            let standard_fields: Vec<_> = unmapped_fields.iter().filter(|f| !f.is_custom && !f.is_required && !Self::is_relationship_field(f)).collect();

            // Required Unmapped
            if !required_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Required Fields (Need Attention)", &Format::new().set_bold().set_font_color(Color::Red))?;
                row += 1;
                for field in required_fields {
                    Self::write_target_field_row(sheet, row, field, "", "Unmapped", &required_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Relationship Fields
            if !relationship_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Relationship/Lookup Fields", &Format::new().set_bold())?;
                row += 1;
                for field in relationship_fields {
                    Self::write_target_field_row(sheet, row, field, "", "Unmapped", &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Custom Fields
            if !custom_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Custom Fields", &Format::new().set_bold())?;
                row += 1;
                for field in custom_fields {
                    Self::write_target_field_row(sheet, row, field, "", "Unmapped", &custom_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Standard Fields
            if !standard_fields.is_empty() {
                sheet.write_string_with_format(row, 0, "  Standard Fields", &Format::new().set_bold())?;
                row += 1;
                for field in standard_fields {
                    Self::write_target_field_row(sheet, row, field, "", "Unmapped", &unmapped_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source relationships sheet
    fn create_source_relationships_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Relationships")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Relationships", comparison_data.source_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Relationship Name", "Related Entity", "Type", "Required"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let source_relationships = Self::extract_relationship_fields(&comparison_data.source_fields);

        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let indent_format = Format::new().set_indent(1);

        if source_relationships.is_empty() {
            sheet.write_string(row, 0, "No relationship fields found")?;
        } else {
            // Group by relationship type
            let mut many_to_one: Vec<_> = Vec::new();
            let mut one_to_many: Vec<_> = Vec::new();
            let mut many_to_many: Vec<_> = Vec::new();
            let mut other: Vec<_> = Vec::new();

            for field in &source_relationships {
                let relationship_type = Self::get_relationship_type_from_field(field);
                if relationship_type.contains("Many-to-One") || relationship_type.contains("N:1") {
                    many_to_one.push(field);
                } else if relationship_type.contains("One-to-Many") || relationship_type.contains("1:N") {
                    one_to_many.push(field);
                } else if relationship_type.contains("Many-to-Many") || relationship_type.contains("N:N") {
                    many_to_many.push(field);
                } else {
                    other.push(field);
                }
            }

            // Many-to-One Relationships (most common)
            if !many_to_one.is_empty() {
                sheet.write_string_with_format(row, 0, "🔗 LOOKUP FIELDS (Many-to-One)", &header_format)?;
                row += 1;

                for field in many_to_one {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // One-to-Many Relationships
            if !one_to_many.is_empty() {
                sheet.write_string_with_format(row, 0, "📋 ONE-TO-MANY RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in one_to_many {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Many-to-Many Relationships
            if !many_to_many.is_empty() {
                sheet.write_string_with_format(row, 0, "🔄 MANY-TO-MANY RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in many_to_many {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Other Relationships
            if !other.is_empty() {
                sheet.write_string_with_format(row, 0, "🔗 OTHER RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in other {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target relationships sheet
    fn create_target_relationships_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Relationships")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Relationships", comparison_data.target_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Relationship Name", "Related Entity", "Type", "Required"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        let target_relationships = Self::extract_relationship_fields(&comparison_data.target_fields);

        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let indent_format = Format::new().set_indent(1);

        if target_relationships.is_empty() {
            sheet.write_string(row, 0, "No relationship fields found")?;
        } else {
            // Group by relationship type
            let mut many_to_one: Vec<_> = Vec::new();
            let mut one_to_many: Vec<_> = Vec::new();
            let mut many_to_many: Vec<_> = Vec::new();
            let mut other: Vec<_> = Vec::new();

            for field in &target_relationships {
                let relationship_type = Self::get_relationship_type_from_field(field);
                if relationship_type.contains("Many-to-One") || relationship_type.contains("N:1") {
                    many_to_one.push(field);
                } else if relationship_type.contains("One-to-Many") || relationship_type.contains("1:N") {
                    one_to_many.push(field);
                } else if relationship_type.contains("Many-to-Many") || relationship_type.contains("N:N") {
                    many_to_many.push(field);
                } else {
                    other.push(field);
                }
            }

            // Many-to-One Relationships (most common)
            if !many_to_one.is_empty() {
                sheet.write_string_with_format(row, 0, "🔗 LOOKUP FIELDS (Many-to-One)", &header_format)?;
                row += 1;

                for field in many_to_one {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // One-to-Many Relationships
            if !one_to_many.is_empty() {
                sheet.write_string_with_format(row, 0, "📋 ONE-TO-MANY RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in one_to_many {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Many-to-Many Relationships
            if !many_to_many.is_empty() {
                sheet.write_string_with_format(row, 0, "🔄 MANY-TO-MANY RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in many_to_many {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
                row += 1;
            }

            // Other Relationships
            if !other.is_empty() {
                sheet.write_string_with_format(row, 0, "🔗 OTHER RELATIONSHIPS", &header_format)?;
                row += 1;

                for field in other {
                    let target_entity = Self::extract_target_entity_from_field(field);
                    let relationship_type = Self::get_relationship_type_from_field(field);
                    Self::write_relationship_row(sheet, row, field, &target_entity, &relationship_type, &relationship_format, &indent_format)?;
                    row += 1;
                }
            }
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source views sheet with column structure
    fn create_source_views_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Views")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Views Structure", comparison_data.source_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Item", "Type", "Properties", "Width/Primary"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        for view in &comparison_data.source_views {
            // View header
            sheet.write_string_with_format(row, 0, &view.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, "View")?;
            sheet.write_string(row, 2, &format!("Type: {}, Custom: {}, Columns: {}",
                view.view_type, view.is_custom, view.columns.len()))?;
            row += 1;

            // View columns
            for column in &view.columns {
                sheet.write_string_with_format(row, 0, &column.name, &indent_format)?;
                sheet.write_string(row, 1, "Column")?;
                sheet.write_string(row, 2, "")?; // No additional properties for columns
                let width_info = if let Some(width) = column.width {
                    format!("Width: {}, Primary: {}", width, column.is_primary)
                } else {
                    format!("Width: Auto, Primary: {}", column.is_primary)
                };
                sheet.write_string(row, 3, &width_info)?;
                row += 1;
            }

            row += 1; // Space between views
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target views sheet with column structure
    fn create_target_views_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Views")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Views Structure", comparison_data.target_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Item", "Type", "Properties", "Width/Primary"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        for view in &comparison_data.target_views {
            // View header
            sheet.write_string_with_format(row, 0, &view.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, "View")?;
            sheet.write_string(row, 2, &format!("Type: {}, Custom: {}, Columns: {}",
                view.view_type, view.is_custom, view.columns.len()))?;
            row += 1;

            // View columns
            for column in &view.columns {
                sheet.write_string_with_format(row, 0, &column.name, &indent_format)?;
                sheet.write_string(row, 1, "Column")?;
                sheet.write_string(row, 2, "")?; // No additional properties for columns
                let width_info = if let Some(width) = column.width {
                    format!("Width: {}, Primary: {}", width, column.is_primary)
                } else {
                    format!("Width: Auto, Primary: {}", column.is_primary)
                };
                sheet.write_string(row, 3, &width_info)?;
                row += 1;
            }

            row += 1; // Space between views
        }

        sheet.autofit();
        Ok(())
    }

    /// Create source forms sheet with nested structure
    fn create_source_forms_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Forms")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);
        let indent2_format = Format::new().set_indent(2);
        let indent3_format = Format::new().set_indent(3);

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Forms Structure", comparison_data.source_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Item", "Type", "Properties", "Order/Position"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        for form in &comparison_data.source_forms {
            // Form header
            sheet.write_string_with_format(row, 0, &form.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, &form.form_type)?;
            sheet.write_string(row, 2, &format!("State: {}, Custom: {}", form.state, form.is_custom))?;
            row += 1;

            // Form structure if available
            if let Some(structure) = &form.form_structure {
                for tab in &structure.tabs {
                    // Tab level
                    sheet.write_string_with_format(row, 0, &tab.label, &indent_format)?;
                    sheet.write_string(row, 1, "Tab")?;
                    sheet.write_string(row, 2, &format!("Visible: {}, Expanded: {}", tab.visible, tab.expanded))?;
                    sheet.write_string(row, 3, &tab.order.to_string())?;
                    row += 1;

                    for section in &tab.sections {
                        // Section level
                        sheet.write_string_with_format(row, 0, &section.label, &indent2_format)?;
                        sheet.write_string(row, 1, "Section")?;
                        sheet.write_string(row, 2, &format!("Columns: {}, Visible: {}", section.columns, section.visible))?;
                        sheet.write_string(row, 3, &section.order.to_string())?;
                        row += 1;

                        for field in &section.fields {
                            // Field level
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

            row += 1; // Space between forms
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target forms sheet with nested structure
    fn create_target_forms_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        _shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Forms")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();
        let indent_format = Format::new().set_indent(1);
        let indent2_format = Format::new().set_indent(2);
        let indent3_format = Format::new().set_indent(3);

        // Title
        sheet.write_string_with_format(
            0,
            0,
            &format!("{} - Forms Structure", comparison_data.target_entity),
            &title_format,
        )?;

        // Headers
        let headers = ["Item", "Type", "Properties", "Order/Position"];
        for (col, header) in headers.iter().enumerate() {
            sheet.write_string_with_format(2, col as u16, *header, &header_format)?;
        }

        let mut row = 3u32;
        for form in &comparison_data.target_forms {
            // Form header
            sheet.write_string_with_format(row, 0, &form.name, &Format::new().set_bold())?;
            sheet.write_string(row, 1, &form.form_type)?;
            sheet.write_string(row, 2, &format!("State: {}, Custom: {}", form.state, form.is_custom))?;
            row += 1;

            // Form structure if available
            if let Some(structure) = &form.form_structure {
                for tab in &structure.tabs {
                    // Tab level
                    sheet.write_string_with_format(row, 0, &tab.label, &indent_format)?;
                    sheet.write_string(row, 1, "Tab")?;
                    sheet.write_string(row, 2, &format!("Visible: {}, Expanded: {}", tab.visible, tab.expanded))?;
                    sheet.write_string(row, 3, &tab.order.to_string())?;
                    row += 1;

                    for section in &tab.sections {
                        // Section level
                        sheet.write_string_with_format(row, 0, &section.label, &indent2_format)?;
                        sheet.write_string(row, 1, "Section")?;
                        sheet.write_string(row, 2, &format!("Columns: {}, Visible: {}", section.columns, section.visible))?;
                        sheet.write_string(row, 3, &section.order.to_string())?;
                        row += 1;

                        for field in &section.fields {
                            // Field level
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

            row += 1; // Space between forms
        }

        sheet.autofit();
        Ok(())
    }

    /// Create examples sheet showing actual fetched entity data
    fn create_examples_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Examples")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(0, 0, "Example Entity Data", &title_format)?;

        if shared_state.examples.examples.is_empty() {
            sheet.write_string(2, 0, "No examples configured")?;
            sheet.autofit();
            return Ok(());
        }

        // Debug info
        let data_keys: Vec<String> = shared_state.examples.example_data.keys().cloned().collect();
        sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}, Keys: {:?}",
            shared_state.examples.examples.len(),
            shared_state.examples.example_data.len(),
            data_keys
        ))?;

        let mut row = 2u32;

        // For each example pair
        for (idx, example) in shared_state.examples.examples.iter().enumerate() {
            row += 1;
            let example_title = example.display_name();
            sheet.write_string_with_format(row, 0, &format!("Example {}: {}", idx + 1, example_title), &header_format)?;
            row += 1;

            // Headers for source and target
            sheet.write_string_with_format(row, 0, "Field", &header_format)?;
            sheet.write_string_with_format(row, 1, &format!("Source Value ({})", comparison_data.source_entity), &header_format)?;
            sheet.write_string_with_format(row, 2, &format!("Target Value ({})", comparison_data.target_entity), &header_format)?;
            sheet.write_string_with_format(row, 3, "Status", &header_format)?;
            row += 1;

            // Get example data if available
            let source_data = shared_state.examples.example_data.get(&example.source_uuid);
            let target_data = shared_state.examples.example_data.get(&example.target_uuid);

            // Show mapped fields with their actual values
            for source_field in &comparison_data.source_fields {
                if let Some(target_name) = Self::get_mapped_target_field(&source_field.name, comparison_data, shared_state) {
                    let source_lookup_key = format!("source:{}", example.source_uuid);
                    let target_lookup_key = format!("target:{}", example.target_uuid);

                    let source_value = if let Some(data) = shared_state.examples.example_data.get(&source_lookup_key) {
                        Self::extract_field_value(data, &source_field.name)
                    } else {
                        "No example data".to_string()
                    };

                    let target_value = if let Some(data) = shared_state.examples.example_data.get(&target_lookup_key) {
                        Self::extract_field_value(data, &target_name)
                    } else {
                        "No example data".to_string()
                    };

                    let (status, status_format) = if source_value == target_value && source_value != "No example data" {
                        ("Values Match", Self::create_values_match_format())
                    } else if source_value == "No example data" || target_value == "No example data" {
                        ("Missing Data", Self::create_missing_data_format())
                    } else {
                        ("Values Differ", Self::create_values_differ_format())
                    };

                    sheet.write_string_with_format(row, 0, &format!("{} → {}", source_field.name, target_name), &status_format)?;
                    sheet.write_string_with_format(row, 1, &source_value, &status_format)?;
                    sheet.write_string_with_format(row, 2, &target_value, &status_format)?;
                    sheet.write_string_with_format(row, 3, status, &status_format)?;
                    row += 1;
                }
            }

            row += 1; // Space between examples
        }

        sheet.autofit();
        Ok(())
    }

    /// Extract field value from JSON data
    fn extract_field_value(data: &serde_json::Value, field_name: &str) -> String {
        match data.get(field_name) {
            Some(value) => match value {
                serde_json::Value::Null => "null".to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(_) => "[Array]".to_string(),
                serde_json::Value::Object(_) => "[Object]".to_string(),
            },
            None => "Field not found".to_string(),
        }
    }

    /// Create source examples sheet showing all source field values
    fn create_source_examples_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Source Examples")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(0, 0, &format!("Source Entity Examples ({})", comparison_data.source_entity), &title_format)?;

        if shared_state.examples.examples.is_empty() {
            sheet.write_string(2, 0, "No examples configured")?;
            sheet.autofit();
            return Ok(());
        }

        // Debug info
        sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}",
            shared_state.examples.examples.len(),
            shared_state.examples.example_data.len()
        ))?;

        let mut row = 2u32;

        // Headers - Field name + example columns
        sheet.write_string_with_format(row, 0, "Field Name", &header_format)?;
        sheet.write_string_with_format(row, 1, "Type", &header_format)?;
        sheet.write_string_with_format(row, 2, "Required", &header_format)?;
        sheet.write_string_with_format(row, 3, "Custom", &header_format)?;

        // Add column for each example
        for (idx, example) in shared_state.examples.examples.iter().enumerate() {
            let col = 4 + idx as u16;
            sheet.write_string_with_format(row, col, &format!("Example {} ({}...)", idx + 1, &example.source_uuid[..8]), &header_format)?;
        }
        row += 1;

        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let missing_data_format = Self::create_missing_data_format();

        // Show all fields (mapped and unmapped)
        for field in &comparison_data.source_fields {
            // Choose format based on field properties
            let field_format = if Self::is_relationship_field(field) {
                &relationship_format
            } else if field.is_custom {
                &custom_format
            } else {
                &Format::new()
            };

            let required_cell_format = if field.is_required { &required_format } else { field_format };
            let custom_cell_format = if field.is_custom { &custom_format } else { field_format };

            sheet.write_string_with_format(row, 0, &field.name, field_format)?;
            sheet.write_string_with_format(row, 1, &field.field_type, field_format)?;
            sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
            sheet.write_string_with_format(row, 3, if field.is_custom { "Yes" } else { "No" }, custom_cell_format)?;

            // Show value for each example
            for (idx, example) in shared_state.examples.examples.iter().enumerate() {
                let col = 4 + idx as u16;
                let source_lookup_key = format!("source:{}", example.source_uuid);
                let value = if let Some(data) = shared_state.examples.example_data.get(&source_lookup_key) {
                    Self::extract_field_value(data, &field.name)
                } else {
                    "No example data loaded".to_string()
                };

                let value_format = if value.contains("No example data") || value == "Field not found" {
                    &missing_data_format
                } else {
                    field_format
                };

                sheet.write_string_with_format(row, col, &value, value_format)?;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    /// Create target examples sheet showing all target field values
    fn create_target_examples_sheet(
        workbook: &mut Workbook,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Result<()> {
        let sheet = workbook.add_worksheet();
        sheet.set_name("Target Examples")?;

        let header_format = Self::create_header_format();
        let title_format = Self::create_title_format();

        // Title
        sheet.write_string_with_format(0, 0, &format!("Target Entity Examples ({})", comparison_data.target_entity), &title_format)?;

        if shared_state.examples.examples.is_empty() {
            sheet.write_string(2, 0, "No examples configured")?;
            sheet.autofit();
            return Ok(());
        }

        // Debug info
        sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}",
            shared_state.examples.examples.len(),
            shared_state.examples.example_data.len()
        ))?;

        let mut row = 2u32;

        // Headers - Field name + example columns
        sheet.write_string_with_format(row, 0, "Field Name", &header_format)?;
        sheet.write_string_with_format(row, 1, "Type", &header_format)?;
        sheet.write_string_with_format(row, 2, "Required", &header_format)?;
        sheet.write_string_with_format(row, 3, "Custom", &header_format)?;

        // Add column for each example
        for (idx, example) in shared_state.examples.examples.iter().enumerate() {
            let col = 4 + idx as u16;
            sheet.write_string_with_format(row, col, &format!("Example {} ({}...)", idx + 1, &example.target_uuid[..8]), &header_format)?;
        }
        row += 1;

        let relationship_format = Self::create_relationship_format();
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let missing_data_format = Self::create_missing_data_format();

        // Show all fields (mapped and unmapped)
        for field in &comparison_data.target_fields {
            // Choose format based on field properties
            let field_format = if Self::is_relationship_field(field) {
                &relationship_format
            } else if field.is_custom {
                &custom_format
            } else {
                &Format::new()
            };

            let required_cell_format = if field.is_required { &required_format } else { field_format };
            let custom_cell_format = if field.is_custom { &custom_format } else { field_format };

            sheet.write_string_with_format(row, 0, &field.name, field_format)?;
            sheet.write_string_with_format(row, 1, &field.field_type, field_format)?;
            sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
            sheet.write_string_with_format(row, 3, if field.is_custom { "Yes" } else { "No" }, custom_cell_format)?;

            // Show value for each example
            for (idx, example) in shared_state.examples.examples.iter().enumerate() {
                let col = 4 + idx as u16;
                let target_lookup_key = format!("target:{}", example.target_uuid);
                let value = if let Some(data) = shared_state.examples.example_data.get(&target_lookup_key) {
                    Self::extract_field_value(data, &field.name)
                } else {
                    "No example data loaded".to_string()
                };

                let value_format = if value.contains("No example data") || value == "Field not found" {
                    &missing_data_format
                } else {
                    field_format
                };

                sheet.write_string_with_format(row, col, &value, value_format)?;
            }
            row += 1;
        }

        sheet.autofit();
        Ok(())
    }

    // Helper functions for field mapping
    fn get_mapped_target_field(
        source_field: &str,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Option<String> {
        // Check manual mappings first
        if let Some(target) = shared_state.field_mappings.get(source_field) {
            return Some(target.clone());
        }

        // Check prefix mappings
        for (source_prefix, target_prefix) in &shared_state.prefix_mappings {
            if source_field.starts_with(source_prefix) {
                let suffix = &source_field[source_prefix.len()..];
                let target_field = format!("{}{}", target_prefix, suffix);
                if comparison_data.target_fields.iter().any(|f| f.name == target_field) {
                    return Some(target_field);
                }
            }
        }

        // Check exact matches
        if comparison_data.target_fields.iter().any(|f| f.name == source_field) {
            return Some(source_field.to_string());
        }

        None
    }

    fn get_mapped_source_field(
        target_field: &str,
        comparison_data: &ComparisonData,
        shared_state: &SharedState,
    ) -> Option<String> {
        // Check manual mappings
        for (source, target) in &shared_state.field_mappings {
            if target == target_field {
                return Some(source.clone());
            }
        }

        // Check prefix mappings
        for (source_prefix, target_prefix) in &shared_state.prefix_mappings {
            if target_field.starts_with(target_prefix) {
                let suffix = &target_field[target_prefix.len()..];
                let source_field = format!("{}{}", source_prefix, suffix);
                if comparison_data.source_fields.iter().any(|f| f.name == source_field) {
                    return Some(source_field);
                }
            }
        }

        // Check exact matches
        if comparison_data.source_fields.iter().any(|f| f.name == target_field) {
            return Some(target_field.to_string());
        }

        None
    }

    // Helper methods for enhanced sheet formatting

    /// Write a field row with consistent formatting
    fn write_field_row(
        sheet: &mut Worksheet,
        row: u32,
        field: &crate::dynamics::metadata::FieldInfo,
        target_field_name: &str,
        mapping_type: &str,
        row_format: &Format,
        indent_format: &Format,
    ) -> Result<()> {
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let relationship_format = Self::create_relationship_format();

        // Choose format for field name based on special properties
        let name_format = if Self::is_relationship_field(field) {
            &relationship_format
        } else if field.is_custom {
            &custom_format
        } else {
            indent_format
        };

        let required_cell_format = if field.is_required { &required_format } else { row_format };
        let custom_cell_format = if field.is_custom { &custom_format } else { row_format };

        sheet.write_string_with_format(row, 0, &format!("    {}", field.name), name_format)?;
        sheet.write_string_with_format(row, 1, &field.field_type, row_format)?;
        sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
        sheet.write_string_with_format(row, 3, if field.is_custom { "Yes" } else { "No" }, custom_cell_format)?;
        sheet.write_string_with_format(row, 4, target_field_name, row_format)?;
        sheet.write_string_with_format(row, 5, mapping_type, row_format)?;

        Ok(())
    }

    /// Write a target field row with consistent formatting
    fn write_target_field_row(
        sheet: &mut Worksheet,
        row: u32,
        field: &crate::dynamics::metadata::FieldInfo,
        source_field_name: &str,
        mapping_type: &str,
        row_format: &Format,
        indent_format: &Format,
    ) -> Result<()> {
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();
        let relationship_format = Self::create_relationship_format();

        // Choose format for field name based on special properties
        let name_format = if Self::is_relationship_field(field) {
            &relationship_format
        } else if field.is_custom {
            &custom_format
        } else {
            indent_format
        };

        let required_cell_format = if field.is_required { &required_format } else { row_format };
        let custom_cell_format = if field.is_custom { &custom_format } else { row_format };

        sheet.write_string_with_format(row, 0, &format!("    {}", field.name), name_format)?;
        sheet.write_string_with_format(row, 1, &field.field_type, row_format)?;
        sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
        sheet.write_string_with_format(row, 3, if field.is_custom { "Yes" } else { "No" }, custom_cell_format)?;
        sheet.write_string_with_format(row, 4, source_field_name, row_format)?;
        sheet.write_string_with_format(row, 5, mapping_type, row_format)?;

        Ok(())
    }

    /// Write a relationship field row with consistent formatting
    fn write_relationship_row(
        sheet: &mut Worksheet,
        row: u32,
        field: &crate::dynamics::metadata::FieldInfo,
        target_entity: &str,
        relationship_type: &str,
        relationship_format: &Format,
        indent_format: &Format,
    ) -> Result<()> {
        let required_format = Self::create_required_format();
        let custom_format = Self::create_custom_format();

        let name_format = if field.is_custom { &custom_format } else { indent_format };
        let required_cell_format = if field.is_required { &required_format } else { relationship_format };

        sheet.write_string_with_format(row, 0, &format!("    {}", field.name), name_format)?;
        sheet.write_string_with_format(row, 1, target_entity, relationship_format)?;
        sheet.write_string_with_format(row, 2, relationship_type, relationship_format)?;
        sheet.write_string_with_format(row, 3, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;

        Ok(())
    }

    // Relationship extraction helpers

    /// Extract relationship fields from a list of fields using Dynamics 365 conventions
    fn extract_relationship_fields(fields: &[crate::dynamics::metadata::FieldInfo]) -> Vec<crate::dynamics::metadata::FieldInfo> {
        fields
            .iter()
            .filter(|field| Self::is_relationship_field(field))
            .cloned()
            .collect()
    }

    /// Check if a field is a relationship field using Dynamics 365 conventions
    fn is_relationship_field(field: &crate::dynamics::metadata::FieldInfo) -> bool {
        // Check for Dynamics 365 lookup fields (end with _value and type Edm.Guid)
        if field.name.ends_with("_value") && field.field_type == "Edm.Guid" {
            return true;
        }

        // Check for explicit relationship type strings (though rare in real data)
        if field.field_type.contains("→")
            || field.field_type.contains("N:1")
            || field.field_type.contains("1:N")
        {
            return true;
        }

        false
    }

    /// Extract target entity name from relationship field
    fn extract_target_entity_from_field(field: &crate::dynamics::metadata::FieldInfo) -> String {
        // For Dynamics 365 lookup fields (_value fields), extract entity from field name
        if field.name.ends_with("_value") {
            let base_name = field.name.strip_suffix("_value").unwrap_or(&field.name);

            // Common patterns:
            // _customerid_value -> Customer
            // _parentaccountid_value -> Account
            // _primarycontactid_value -> Contact
            if base_name.ends_with("id") {
                let entity_part = base_name.strip_suffix("id").unwrap_or(base_name);
                // Remove leading underscore if present
                let clean_name = entity_part.strip_prefix('_').unwrap_or(entity_part);

                // Handle common entity mappings
                match clean_name {
                    "customerid" | "customer" => "Customer".to_string(),
                    "parentaccountid" | "parentaccount" => "Account".to_string(),
                    "primarycontactid" | "primarycontact" => "Contact".to_string(),
                    "ownerid" | "owner" => "SystemUser".to_string(),
                    "regardingobjectid" | "regardingobject" => "Various".to_string(),
                    _ => {
                        // Capitalize first letter
                        let mut chars = clean_name.chars();
                        match chars.next() {
                            None => clean_name.to_string(),
                            Some(first) => first.to_uppercase().chain(chars).collect(),
                        }
                    }
                }
            } else {
                base_name.to_string()
            }
        }
        // For explicit relationship type strings, extract entity after →
        else if field.field_type.contains("→") {
            field.field_type
                .split("→")
                .nth(1)
                .unwrap_or("Unknown")
                .trim()
                .to_string()
        }
        else {
            "Unknown".to_string()
        }
    }

    /// Get relationship type string from field
    fn get_relationship_type_from_field(field: &crate::dynamics::metadata::FieldInfo) -> String {
        // For Dynamics 365 lookup fields, they are typically Many-to-One (N:1)
        if field.name.ends_with("_value") && field.field_type == "Edm.Guid" {
            return "Many-to-One (N:1)".to_string();
        }

        // For explicit relationship type strings
        if field.field_type.contains("N:1") {
            "Many-to-One (N:1)".to_string()
        } else if field.field_type.contains("1:N") {
            "One-to-Many (1:N)".to_string()
        } else if field.field_type.contains("N:N") || field.field_type.contains("M:N") {
            "Many-to-Many (N:N)".to_string()
        } else {
            "Lookup".to_string()
        }
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

    fn create_mapped_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0x90EE90))
    }

    fn create_unmapped_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFB6C1))
    }

    // Additional color formats
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

    fn create_relationship_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xDDA0DD))  // Plum/Light Purple
    }

    fn create_required_format() -> Format {
        Format::new()
            .set_background_color(Color::RGB(0xFFA07A))  // Light Salmon
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