use anyhow::Result;
use log::info;
use crate::config::Config;
use crate::ui::prompts::confirm;

/// Remove an entity name mapping
///
/// # Arguments
/// * `entity_name` - Entity name to remove
/// * `force` - Skip confirmation prompt
///
/// # Returns
/// * `Ok(())` - Mapping removed successfully
/// * `Err(anyhow::Error)` - Configuration error or user cancelled
pub async fn remove_command(entity_name: String, force: bool) -> Result<()> {
    info!("Removing entity mapping: {}", entity_name);

    let mut config = Config::load()?;

    // Check if mapping exists
    let plural_name = match config.get_entity_mapping(&entity_name) {
        Some(plural) => plural.clone(),
        None => {
            println!("Entity mapping '{}' not found.", entity_name);
            return Ok(());
        }
    };

    // Confirm removal unless force flag is used
    if !force {
        let message = format!("Remove entity mapping '{}' -> '{}'?", entity_name, plural_name);
        if !confirm(&message, false)? {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    config.remove_entity_mapping(&entity_name)?;

    println!("Removed entity mapping: {} -> {}", entity_name, plural_name);
    Ok(())
}