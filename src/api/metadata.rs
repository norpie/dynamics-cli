//! Metadata parsing utilities for Dynamics 365

use anyhow::Result;
use roxmltree::Document;

/// Parse Dynamics 365 metadata XML and extract all entity names
pub fn parse_entity_list(metadata_xml: &str) -> Result<Vec<String>> {
    log::info!("Starting metadata XML parsing, XML length: {} bytes", metadata_xml.len());

    let doc = Document::parse(metadata_xml)
        .map_err(|e| anyhow::anyhow!("Failed to parse metadata XML: {}", e))?;

    log::debug!("Metadata XML parsed successfully");

    let mut entities = Vec::new();

    // Find all EntityType elements
    // In EDMX, entities are defined as <EntityType Name="account">
    for entity_type in doc.descendants().filter(|node| node.has_tag_name("EntityType")) {
        if let Some(name) = entity_type.attribute("Name") {
            entities.push(name.to_string());
        }
    }

    // Sort alphabetically
    entities.sort();

    log::info!("Successfully parsed {} entities from metadata", entities.len());
    Ok(entities)
}
