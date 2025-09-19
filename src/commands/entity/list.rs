use crate::config::Config;
use anyhow::Result;
use log::info;

/// List all entity name mappings
///
/// # Returns
/// * `Ok(())` - List displayed successfully
/// * `Err(anyhow::Error)` - Configuration error
pub async fn list_command() -> Result<()> {
    info!("Listing entity name mappings");

    let config = Config::load()?;
    let mappings = config.list_entity_mappings();

    if mappings.is_empty() {
        println!("No entity mappings configured.");
        return Ok(());
    }

    println!("Entity Name Mappings:");
    println!("{:<20} -> Plural Name", "Entity Name");
    println!("{}", "-".repeat(50));

    let mut mappings_vec: Vec<(&String, &String)> = mappings.iter().collect();
    mappings_vec.sort_by_key(|(entity, _)| entity.as_str());

    for (entity, plural) in mappings_vec {
        println!("{:<20} -> {}", entity, plural);
    }

    println!("\nTotal mappings: {}", mappings.len());
    Ok(())
}
