use anyhow::Result;
use log::info;
use crate::config::{Config, Settings};
use crate::ui::prompts::confirm;

/// Reset a setting to its default value
///
/// # Arguments
/// * `name` - Setting name
///
/// # Returns
/// * `Ok(())` - Setting reset successfully
/// * `Err(anyhow::Error)` - Configuration error or unknown setting
pub async fn reset_command(name: String) -> Result<()> {
    info!("Resetting setting: {}", name);

    let mut config = Config::load()?;
    let defaults = Settings::default();

    match name.as_str() {
        "default-query-limit" => {
            config.update_default_query_limit(defaults.default_query_limit)?;
            println!("Reset default-query-limit to {}", defaults.default_query_limit);
        },
        _ => {
            anyhow::bail!("Unknown setting: {}", name);
        }
    }

    Ok(())
}

/// Reset all settings to default values
///
/// # Arguments
/// * `force` - Skip confirmation prompt
///
/// # Returns
/// * `Ok(())` - Settings reset successfully
/// * `Err(anyhow::Error)` - Configuration error or user cancelled
pub async fn reset_all_command(force: bool) -> Result<()> {
    info!("Resetting all settings to defaults");

    if !force {
        if !confirm("Reset all settings to their default values?", false)? {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    let mut config = Config::load()?;
    config.settings = Settings::default();
    config.save()?;

    println!("All settings have been reset to default values:");
    println!("  default-query-limit: {}", config.settings.default_query_limit);

    Ok(())
}