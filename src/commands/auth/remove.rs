use anyhow::Result;
use log::info;

use crate::config::Config;
use crate::ui::prompt_remove_confirmation;

pub async fn remove_command(name: String, force: bool) -> Result<()> {
    info!("Removing environment: {}", name);

    let mut config = Config::load()?;

    // Check if environment exists
    if !config.environments.contains_key(&name) {
        println!("Environment '{}' not found.", name);
        println!("Available environments:");
        for env_name in config.list_environments() {
            if config.get_current_environment_name() == Some(env_name) {
                println!("  ● {} (current)", env_name);
            } else {
                println!("  ○ {}", env_name);
            }
        }
        return Ok(());
    }

    // Confirm removal
    let current_env = config.get_current_environment_name();
    if current_env == Some(&name) {
        println!("⚠ Warning: '{}' is the current environment", name);
    }

    let confirm = if force {
        true
    } else {
        prompt_remove_confirmation(&name)?
    };

    if !confirm {
        println!("Removal cancelled.");
        return Ok(());
    }

    // Remove the environment
    config.remove_environment(&name)?;
    println!("✓ Environment '{}' removed successfully", name);

    // Show current status after removal
    if let Some(current) = config.get_current_environment_name() {
        println!("Current environment: {}", current);
    } else {
        println!("No current environment selected. Run 'dynamics-cli auth select' to choose one.");
    }

    Ok(())
}