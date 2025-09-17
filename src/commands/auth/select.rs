use anyhow::Result;
use log::info;

use crate::config::Config;
use crate::ui::prompt_environment_selection;

pub async fn select_command(name: Option<String>) -> Result<()> {
    info!("Starting auth select");

    let mut config = Config::load()?;
    let environments = config.list_environments();

    if environments.is_empty() {
        println!("No environments configured. Run 'dynamics-cli auth setup' to create one.");
        return Ok(());
    }

    let selected_env = if let Some(name) = name {
        if !config.environments.contains_key(&name) {
            anyhow::bail!("Environment '{}' not found", name);
        }
        name
    } else {
        let env_names: Vec<String> = environments.iter().map(|s| (*s).clone()).collect();
        let current_env = config.get_current_environment_name();

        prompt_environment_selection(&env_names, current_env)?
    };

    config.set_current_environment(selected_env.clone())?;
    println!("✓ Selected environment: {}", selected_env);

    Ok(())
}