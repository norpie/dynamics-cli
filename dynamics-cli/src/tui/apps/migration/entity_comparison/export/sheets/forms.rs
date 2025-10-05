//! Source and Target Forms sheets

use anyhow::Result;
use rust_xlsxwriter::*;

use crate::api::metadata::FormMetadata;
use crate::tui::Resource;
use super::super::super::app::State;
use super::super::formatting::*;

pub fn create_source_forms_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Source Forms")?;

    let header_format = create_header_format();
    let title_format = create_title_format();
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
pub fn create_target_forms_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Target Forms")?;

    let header_format = create_header_format();
    let title_format = create_title_format();
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

