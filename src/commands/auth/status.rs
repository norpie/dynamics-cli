use anyhow::Result;
use log::{info, error};

use crate::config::Config;
use crate::auth::DynamicsAuthClient;

pub async fn status_command() -> Result<()> {
    info!("Executing auth status command");

    let config = Config::load()?;

    println!("Dynamics CLI Authentication Status");
    println!("=================================");

    // Show configured environments
    let environments = config.list_environments();
    if environments.is_empty() {
        println!("No environments configured.");
        println!("Run 'dynamics-cli auth setup' to create one.");
        return Ok(());
    }

    println!("Configured environments:");
    for env_name in &environments {
        if config.get_current_environment_name() == Some(env_name) {
            println!("  ● {} (current)", env_name);
        } else {
            println!("  ○ {}", env_name);
        }
    }

    // Show current environment details and test authentication
    if let Some(current_auth) = config.get_current_auth() {
        let current_env_name = config.get_current_environment_name().unwrap();
        println!("\nCurrent Environment: {}", current_env_name);
        println!("  Host: {}", current_auth.host);
        println!("  Username: {}", current_auth.username);
        println!("  Client ID: {}", current_auth.client_id);

        println!("\nTesting authentication...");
        let auth_client = DynamicsAuthClient::new();
        match auth_client.test_auth(
            &current_auth.host,
            &current_auth.username,
            &current_auth.password,
            &current_auth.client_id,
            &current_auth.client_secret,
        ).await {
            Ok(()) => {
                info!("Authentication test successful");
                println!("✓ Authentication successful");
            },
            Err(e) => {
                error!("Authentication test failed: {}", e);
                println!("✗ Authentication failed: {}", e);
            }
        }
    } else {
        println!("\nNo current environment selected.");
        println!("Run 'dynamics-cli auth select' to choose one.");
    }

    Ok(())
}