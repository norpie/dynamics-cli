use crate::config::Config;
use anyhow::Result;
use log::info;

/// Get the value of a specific setting
///
/// # Arguments
/// * `name` - Setting name
///
/// # Returns
/// * `Ok(())` - Setting value displayed successfully
/// * `Err(anyhow::Error)` - Configuration error or unknown setting
pub async fn get_command(name: String) -> Result<()> {
    info!("Getting setting: {}", name);

    let config = Config::load()?;
    let settings = config.get_settings();

    match name.as_str() {
        "default-query-limit" => {
            println!("{}", settings.default_query_limit);
        }
        _ => {
            anyhow::bail!("Unknown setting: {}", name);
        }
    }

    Ok(())
}
