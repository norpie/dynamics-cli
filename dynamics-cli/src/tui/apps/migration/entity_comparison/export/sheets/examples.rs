//! Examples sheets - showing actual field values from example record pairs

use anyhow::Result;
use rust_xlsxwriter::*;

use crate::api::metadata::FieldMetadata;
use crate::tui::Resource;
use super::super::super::app::State;
use super::super::formatting::*;

/// Create main examples sheet comparing source and target values for mapped fields
pub fn create_examples_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Examples")?;

    let header_format = create_header_format();
    let title_format = create_title_format();

    // Title
    sheet.write_string_with_format(0, 0, "Example Entity Data", &title_format)?;

    if state.examples.pairs.is_empty() {
        sheet.write_string(2, 0, "No examples configured")?;
        sheet.autofit();
        return Ok(());
    }

    // Debug info
    sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}",
        state.examples.pairs.len(),
        state.examples.cache.len()
    ))?;

    let mut row = 2u32;

    // For each example pair
    for (idx, example) in state.examples.pairs.iter().enumerate() {
        row += 1;
        let example_title = example.display_name();
        sheet.write_string_with_format(row, 0, &format!("Example {}: {}", idx + 1, example_title), &header_format)?;
        row += 1;

        // Headers for source and target
        sheet.write_string_with_format(row, 0, "Field", &header_format)?;
        sheet.write_string_with_format(row, 1, &format!("Source Value ({})", state.source_entity), &header_format)?;
        sheet.write_string_with_format(row, 2, &format!("Target Value ({})", state.target_entity), &header_format)?;
        sheet.write_string_with_format(row, 3, "Status", &header_format)?;
        row += 1;

        // Get source fields
        let source_fields = match &state.source_metadata {
            Resource::Success(metadata) => &metadata.fields,
            _ => {
                sheet.write_string(row, 0, "No source metadata loaded")?;
                continue;
            }
        };

        // Show mapped fields with their actual values
        for source_field in source_fields {
            if let Some(match_info) = state.field_matches.get(&source_field.logical_name) {
                let source_value = state.examples.get_field_value(&source_field.logical_name, true, &state.source_entity)
                    .unwrap_or_else(|| "No example data".to_string());

                // For examples sheet, show primary target or all targets comma-separated
                let target_field_names: Vec<&str> = match_info.target_fields.iter()
                    .map(|tf| tf.split('/').last().unwrap_or(tf.as_str()))
                    .collect();
                let target_field_name = target_field_names.join(", ");

                // Get value from first target for comparison
                let first_target_name = match_info.target_fields.first()
                    .map(|tf| tf.split('/').last().unwrap_or(tf.as_str()))
                    .unwrap_or("");
                let target_value = state.examples.get_field_value(first_target_name, false, &state.target_entity)
                    .unwrap_or_else(|| "No example data".to_string());

                let (status, status_format) = if source_value == target_value && source_value != "No example data" {
                    ("Values Match", create_values_match_format())
                } else if source_value == "No example data" || target_value == "No example data" {
                    ("Missing Data", create_missing_data_format())
                } else {
                    ("Values Differ", create_values_differ_format())
                };

                sheet.write_string_with_format(row, 0, &format!("{} → {}", source_field.logical_name, target_field_name), &status_format)?;
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

/// Create source examples sheet showing all source field values across examples
pub fn create_source_examples_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Source Examples")?;

    let header_format = create_header_format();
    let title_format = create_title_format();

    // Title
    sheet.write_string_with_format(0, 0, &format!("Source Entity Examples ({})", state.source_entity), &title_format)?;

    if state.examples.pairs.is_empty() {
        sheet.write_string(2, 0, "No examples configured")?;
        sheet.autofit();
        return Ok(());
    }

    // Debug info
    sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}",
        state.examples.pairs.len(),
        state.examples.cache.len()
    ))?;

    let mut row = 2u32;

    // Headers - Field name + example columns
    sheet.write_string_with_format(row, 0, "Field Name", &header_format)?;
    sheet.write_string_with_format(row, 1, "Type", &header_format)?;
    sheet.write_string_with_format(row, 2, "Required", &header_format)?;
    sheet.write_string_with_format(row, 3, "Primary Key", &header_format)?;

    // Add column for each example
    for (idx, example) in state.examples.pairs.iter().enumerate() {
        let col = 4 + idx as u16;
        let label = if let Some(label) = &example.label {
            format!("Ex{}: {}", idx + 1, label)
        } else {
            format!("Example {} ({}...)", idx + 1, &example.source_record_id[..8.min(example.source_record_id.len())])
        };
        sheet.write_string_with_format(row, col, &label, &header_format)?;
    }
    row += 1;

    let required_format = create_required_format();
    let missing_data_format = create_missing_data_format();

    // Get source fields
    let source_fields = match &state.source_metadata {
        Resource::Success(metadata) => &metadata.fields,
        _ => {
            sheet.write_string(row, 0, "No source metadata loaded")?;
            sheet.autofit();
            return Ok(());
        }
    };

    // Show all fields (mapped and unmapped)
    for field in source_fields {
        let field_format = Format::new();
        let required_cell_format = if field.is_required { &required_format } else { &field_format };

        sheet.write_string_with_format(row, 0, &field.logical_name, &field_format)?;
        sheet.write_string_with_format(row, 1, &format!("{:?}", field.field_type), &field_format)?;
        sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
        sheet.write_string_with_format(row, 3, if field.is_primary_key { "Yes" } else { "No" }, &field_format)?;

        // Show value for each example
        for (idx, _example) in state.examples.pairs.iter().enumerate() {
            let col = 4 + idx as u16;
            let value = state.examples.get_field_value(&field.logical_name, true, &state.source_entity)
                .unwrap_or_else(|| "No example data".to_string());

            let value_format = if value.contains("No example data") || value == "Field not found" {
                &missing_data_format
            } else {
                &field_format
            };

            sheet.write_string_with_format(row, col, &value, value_format)?;
        }
        row += 1;
    }

    sheet.autofit();
    Ok(())
}

/// Create target examples sheet showing all target field values across examples
pub fn create_target_examples_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Target Examples")?;

    let header_format = create_header_format();
    let title_format = create_title_format();

    // Title
    sheet.write_string_with_format(0, 0, &format!("Target Entity Examples ({})", state.target_entity), &title_format)?;

    if state.examples.pairs.is_empty() {
        sheet.write_string(2, 0, "No examples configured")?;
        sheet.autofit();
        return Ok(());
    }

    // Debug info
    sheet.write_string(1, 0, &format!("Examples configured: {}, Data loaded: {}",
        state.examples.pairs.len(),
        state.examples.cache.len()
    ))?;

    let mut row = 2u32;

    // Headers - Field name + example columns
    sheet.write_string_with_format(row, 0, "Field Name", &header_format)?;
    sheet.write_string_with_format(row, 1, "Type", &header_format)?;
    sheet.write_string_with_format(row, 2, "Required", &header_format)?;
    sheet.write_string_with_format(row, 3, "Primary Key", &header_format)?;

    // Add column for each example
    for (idx, example) in state.examples.pairs.iter().enumerate() {
        let col = 4 + idx as u16;
        let label = if let Some(label) = &example.label {
            format!("Ex{}: {}", idx + 1, label)
        } else {
            format!("Example {} ({}...)", idx + 1, &example.target_record_id[..8.min(example.target_record_id.len())])
        };
        sheet.write_string_with_format(row, col, &label, &header_format)?;
    }
    row += 1;

    let required_format = create_required_format();
    let missing_data_format = create_missing_data_format();

    // Get target fields
    let target_fields = match &state.target_metadata {
        Resource::Success(metadata) => &metadata.fields,
        _ => {
            sheet.write_string(row, 0, "No target metadata loaded")?;
            sheet.autofit();
            return Ok(());
        }
    };

    // Show all fields (mapped and unmapped)
    for field in target_fields {
        let field_format = Format::new();
        let required_cell_format = if field.is_required { &required_format } else { &field_format };

        sheet.write_string_with_format(row, 0, &field.logical_name, &field_format)?;
        sheet.write_string_with_format(row, 1, &format!("{:?}", field.field_type), &field_format)?;
        sheet.write_string_with_format(row, 2, if field.is_required { "Yes" } else { "No" }, required_cell_format)?;
        sheet.write_string_with_format(row, 3, if field.is_primary_key { "Yes" } else { "No" }, &field_format)?;

        // Show value for each example
        for (idx, _example) in state.examples.pairs.iter().enumerate() {
            let col = 4 + idx as u16;
            let value = state.examples.get_field_value(&field.logical_name, false, &state.target_entity)
                .unwrap_or_else(|| "No example data".to_string());

            let value_format = if value.contains("No example data") || value == "Field not found" {
                &missing_data_format
            } else {
                &field_format
            };

            sheet.write_string_with_format(row, col, &value, value_format)?;
        }
        row += 1;
    }

    sheet.autofit();
    Ok(())
}
