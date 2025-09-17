use anyhow::Result;
use log::info;
use crate::config::Config;

/// Update an existing entity name mapping
///
/// # Arguments
/// * `entity_name` - Entity name to update
/// * `plural_name` - New plural name
///
/// # Returns
/// * `Ok(())` - Mapping updated successfully
/// * `Err(anyhow::Error)` - Configuration error
pub async fn update_command(entity_name: String, plural_name: String) -> Result<()> {
    info!("Updating entity mapping: {} -> {}", entity_name, plural_name);

    let mut config = Config::load()?;

    // Check if mapping exists
    let old_plural = match config.get_entity_mapping(&entity_name) {
        Some(plural) => plural.clone(),
        None => {
            println!("Entity mapping '{}' not found.", entity_name);
            println!("Use 'entity add' to create a new mapping.");
            anyhow::bail!("Entity mapping not found");
        }
    };

    // Check if the new value is the same as the existing one
    if old_plural == plural_name {
        println!("Entity mapping '{}' already has value '{}'.", entity_name, plural_name);
        return Ok(());
    }

    config.add_entity_mapping(entity_name.clone(), plural_name.clone())?;

    println!("Updated entity mapping: {} -> {} (was: {})", entity_name, plural_name, old_plural);
    Ok(())
}