//! Source and Target Views sheets

use anyhow::Result;
use rust_xlsxwriter::*;

use crate::api::metadata::ViewMetadata;
use crate::tui::Resource;
use super::super::super::app::State;
use super::super::formatting::*;

pub fn create_source_views_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Source Views")?;

    let header_format = create_header_format();
    let title_format = create_title_format();
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
pub fn create_target_views_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Target Views")?;

    let header_format = create_header_format();
    let title_format = create_title_format();
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

