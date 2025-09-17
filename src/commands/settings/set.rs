use anyhow::Result;
use log::info;
use crate::config::Config;

/// Set the value of a specific setting
///
/// # Arguments
/// * `name` - Setting name
/// * `value` - Setting value
///
/// # Returns
/// * `Ok(())` - Setting updated successfully
/// * `Err(anyhow::Error)` - Configuration error or invalid setting/value
pub async fn set_command(name: String, value: String) -> Result<()> {
    info!("Setting {} to {}", name, value);

    let mut config = Config::load()?;

    match name.as_str() {
        "default-query-limit" => {
            let limit: u32 = value.parse()
                .map_err(|_| anyhow::anyhow!("Invalid value for default-query-limit: '{}'. Must be a positive integer.", value))?;

            if limit == 0 {
                anyhow::bail!("default-query-limit must be greater than 0");
            }

            if limit > 50000 {
                println!("Warning: Setting a very high default limit ({}) may impact performance.", limit);
                println!("Consider using explicit limit() clauses for large queries instead.");
            }

            config.update_default_query_limit(limit)?;
            println!("Set default-query-limit to {}", limit);
        },
        _ => {
            anyhow::bail!("Unknown setting: {}", name);
        }
    }

    Ok(())
}