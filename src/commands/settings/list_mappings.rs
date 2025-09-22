use crate::config::Config;
use anyhow::Result;

/// List all field mappings
pub async fn list_mappings_command() -> Result<()> {
    let config = Config::load()?;
    let mappings = config.list_field_mappings();

    if mappings.is_empty() {
        println!("No field mappings found.");
        return Ok(());
    }

    println!("Field Mappings:");
    println!("=============");

    for (entity_comparison, field_mappings) in mappings {
        println!("\n{}", entity_comparison);
        for (source_field, target_field) in field_mappings {
            println!("  {} → {}", source_field, target_field);
        }
    }

    println!("\nTotal entity comparisons: {}", mappings.len());
    let total_mappings: usize = mappings.values().map(|m| m.len()).sum();
    println!("Total field mappings: {}", total_mappings);

    Ok(())
}