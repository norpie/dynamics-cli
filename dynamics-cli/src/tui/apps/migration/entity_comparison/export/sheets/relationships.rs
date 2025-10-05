//! Source and Target Relationships sheets

use anyhow::Result;
use rust_xlsxwriter::*;

use crate::api::metadata::RelationshipMetadata;
use crate::tui::Resource;
use super::super::super::app::State;
use super::super::formatting::*;

pub fn create_source_relationships_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Source Relationships")?;

    let header_format = create_header_format();
    let title_format = create_title_format();

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
    let relationship_format = create_relationship_format();
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
pub fn create_target_relationships_sheet(workbook: &mut Workbook, state: &State) -> Result<()> {
    let sheet = workbook.add_worksheet();
    sheet.set_name("Target Relationships")?;

    let header_format = create_header_format();
    let title_format = create_title_format();

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
    let relationship_format = create_relationship_format();
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

