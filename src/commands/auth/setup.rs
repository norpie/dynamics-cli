use anyhow::Result;
use log::{error, info, warn};

use crate::auth::{Credentials, DynamicsAuthClient};
use crate::config::{AuthConfig, Config};
use crate::ui::{
    prompt_credentials, prompt_environment_name, prompt_overwrite_confirmation,
    prompt_save_anyway_confirmation,
};

pub struct SetupOptions {
    pub name: Option<String>,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub from_env: bool,
    pub from_env_file: Option<String>,
}

pub async fn setup_command(options: SetupOptions) -> Result<()> {
    info!("Starting auth setup");

    let mut config = Config::load()?;

    // Determine credentials source and get values
    let (env_name, credentials) = if options.from_env {
        let env_name = options.name.unwrap_or_else(|| "from-env".to_string());
        let credentials = Credentials::from_env()?;
        (env_name, credentials)
    } else if let Some(ref env_file_path) = options.from_env_file {
        let env_name = options.name.unwrap_or_else(|| "from-env-file".to_string());
        let credentials = Credentials::from_env_file(env_file_path)?;
        (env_name, credentials)
    } else if let (
        Some(host),
        Some(username),
        Some(password),
        Some(client_id),
        Some(client_secret),
    ) = (
        &options.host,
        &options.username,
        &options.password,
        &options.client_id,
        &options.client_secret,
    ) {
        // All parameters provided via command line
        let env_name = options.name.unwrap_or_else(|| "cli-setup".to_string());
        let credentials = Credentials::from_command_line(
            host.clone(),
            username.clone(),
            password.clone(),
            client_id.clone(),
            client_secret.clone(),
        );
        (env_name, credentials)
    } else {
        // Interactive mode - fallback for missing parameters
        info!("Starting interactive setup");

        // Get environment name
        let env_name = prompt_environment_name(options.name)?;

        // Check if environment exists and confirm overwrite
        if config.environments.contains_key(&env_name) {
            let overwrite = prompt_overwrite_confirmation(&env_name)?;

            if !overwrite {
                println!("Setup cancelled.");
                return Ok(());
            }
        }

        // Collect missing authentication details interactively
        let credentials = prompt_credentials(
            options.host,
            options.username,
            options.password,
            options.client_id,
            options.client_secret,
        )?;

        (env_name, credentials)
    };

    // Check if environment exists and handle overwrite (for non-interactive modes)
    if config.environments.contains_key(&env_name)
        && !options.from_env
        && options.from_env_file.is_none()
        && !env_name.starts_with("cli-setup")
    {
        // Non-interactive mode with explicit parameters, assume overwrite
        warn!("Environment '{}' already exists, overwriting", env_name);
    }
    // Interactive mode already handled above

    let auth_config = AuthConfig {
        host: credentials.host.clone(),
        username: credentials.username.clone(),
        password: credentials.password,
        client_id: credentials.client_id.clone(),
        client_secret: credentials.client_secret,
    };

    // Test authentication before saving
    println!("\nTesting authentication...");
    let auth_client = DynamicsAuthClient::new();
    match auth_client
        .test_auth(
            &auth_config.host,
            &auth_config.username,
            &auth_config.password,
            &auth_config.client_id,
            &auth_config.client_secret,
        )
        .await
    {
        Ok(()) => {
            println!("✓ Authentication test successful");
            config.add_environment(env_name.clone(), auth_config)?;
            println!("✓ Environment '{}' saved successfully", env_name);

            if config.current_environment.as_ref() == Some(&env_name) {
                println!("✓ Set as current environment");
            }
        }
        Err(e) => {
            error!("Authentication test failed: {}", e);
            println!("✗ Authentication test failed: {}", e);

            let save_anyway =
                if options.from_env || options.from_env_file.is_some() || env_name == "cli-setup" {
                    // Non-interactive mode, save anyway
                    true
                } else {
                    prompt_save_anyway_confirmation()?
                };

            if save_anyway {
                config.add_environment(env_name.clone(), auth_config)?;
                println!("⚠ Environment '{}' saved (authentication failed)", env_name);
            } else {
                println!("Setup cancelled.");
            }
        }
    }

    Ok(())
}
