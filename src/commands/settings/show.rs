use crate::config::Config;
use anyhow::Result;
use log::info;

/// Show all current settings
///
/// # Returns
/// * `Ok(())` - Settings displayed successfully
/// * `Err(anyhow::Error)` - Configuration error
pub async fn show_command() -> Result<()> {
    info!("Showing all settings");

    let config = Config::load()?;
    let settings = config.get_settings();

    println!("Current Settings:");
    println!("{}", "=".repeat(20));
    println!();

    println!("Query Settings:");
    println!("  default-query-limit: {}", settings.default_query_limit);

    println!();
    println!("Use 'settings set <name> <value>' to change a setting");
    println!("Use 'settings reset <name>' to reset a setting to default");

    Ok(())
}
