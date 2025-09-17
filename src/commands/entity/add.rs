use anyhow::Result;
use log::info;
use crate::config::Config;

/// Add a new entity name mapping
///
/// # Arguments
/// * `entity_name` - Entity name (singular form used in FetchXML)
/// * `plural_name` - Plural name (used in Dynamics Web API)
///
/// # Returns
/// * `Ok(())` - Mapping added successfully
/// * `Err(anyhow::Error)` - Configuration error
pub async fn add_command(entity_name: String, plural_name: String) -> Result<()> {
    info!("Adding entity mapping: {} -> {}", entity_name, plural_name);

    let mut config = Config::load()?;

    // Check if mapping already exists
    if config.get_entity_mapping(&entity_name).is_some() {
        let existing = config.get_entity_mapping(&entity_name).unwrap();
        if existing == &plural_name {
            println!("Entity mapping '{}' -> '{}' already exists.", entity_name, plural_name);
            return Ok(());
        } else {
            println!("Entity mapping '{}' already exists with value '{}'.", entity_name, existing);
            println!("Use 'entity update' to change the mapping.");
            anyhow::bail!("Entity mapping already exists");
        }
    }

    config.add_entity_mapping(entity_name.clone(), plural_name.clone())?;

    println!("Added entity mapping: {} -> {}", entity_name, plural_name);
    Ok(())
}