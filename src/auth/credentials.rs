use anyhow::Result;
use log::info;
use std::path::Path;

#[derive(Debug)]
pub struct Credentials {
    pub host: String,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Credentials {
    pub fn from_env() -> Result<Credentials> {
        info!("Importing from environment variables");

        let host = std::env::var("DYNAMICS_HOST")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_HOST environment variable not set"))?;
        let username = std::env::var("DYNAMICS_USERNAME")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_USERNAME environment variable not set"))?;
        let password = std::env::var("DYNAMICS_PASSWORD")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_PASSWORD environment variable not set"))?;
        let client_id = std::env::var("DYNAMICS_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_CLIENT_ID environment variable not set"))?;
        let client_secret = std::env::var("DYNAMICS_CLIENT_SECRET")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_CLIENT_SECRET environment variable not set"))?;

        println!("✓ Imported credentials from environment variables");

        Ok(Credentials {
            host,
            username,
            password,
            client_id,
            client_secret,
        })
    }

    pub fn from_env_file(path: &str) -> Result<Credentials> {
        info!("Importing from .env file: {}", path);

        // Check if file exists
        if !Path::new(path).exists() {
            anyhow::bail!("Environment file not found: {}", path);
        }

        // Load the specific .env file
        dotenv::from_path(path)
            .map_err(|e| anyhow::anyhow!("Failed to load .env file '{}': {}", path, e))?;

        let host = std::env::var("DYNAMICS_HOST")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_HOST not found in .env file: {}", path))?;
        let username = std::env::var("DYNAMICS_USERNAME")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_USERNAME not found in .env file: {}", path))?;
        let password = std::env::var("DYNAMICS_PASSWORD")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_PASSWORD not found in .env file: {}", path))?;
        let client_id = std::env::var("DYNAMICS_CLIENT_ID")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_CLIENT_ID not found in .env file: {}", path))?;
        let client_secret = std::env::var("DYNAMICS_CLIENT_SECRET")
            .map_err(|_| anyhow::anyhow!("DYNAMICS_CLIENT_SECRET not found in .env file: {}", path))?;

        println!("✓ Imported credentials from .env file: {}", path);

        Ok(Credentials {
            host,
            username,
            password,
            client_id,
            client_secret,
        })
    }

    pub fn from_command_line(
        host: String,
        username: String,
        password: String,
        client_id: String,
        client_secret: String,
    ) -> Credentials {
        info!("Using command line parameters");
        println!("✓ Using credentials from command line parameters");

        Credentials {
            host,
            username,
            password,
            client_id,
            client_secret,
        }
    }
}