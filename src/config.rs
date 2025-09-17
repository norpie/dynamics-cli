use anyhow::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub current_environment: Option<String>,
    pub environments: HashMap<String, AuthConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            current_environment: None,
            environments: HashMap::new(),
        }
    }
}

impl Config {
    pub fn get_config_path() -> Result<PathBuf> {
        let config_dir = if cfg!(target_os = "linux") {
            // Use XDG config directory on Linux
            dirs::config_dir()
                .context("Failed to get XDG config directory")?
                .join("dynamics-cli")
        } else {
            // Use home directory with dot prefix on Windows/Mac
            dirs::home_dir()
                .context("Failed to get home directory")?
                .join(".dynamics-cli")
        };

        // Ensure the directory exists
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
            info!("Created config directory: {:?}", config_dir);
        }

        Ok(config_dir.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        debug!("Loading config from: {:?}", config_path);

        if !config_path.exists() {
            info!("Config file doesn't exist, creating default config");
            return Ok(Self::default());
        }

        let config_content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Config = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        debug!("Loaded config with {} environments", config.environments.len());
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        debug!("Saving config to: {:?}", config_path);

        let config_content = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        fs::write(&config_path, config_content)
            .with_context(|| format!("Failed to write config file: {:?}", config_path))?;

        info!("Config saved successfully");
        Ok(())
    }

    pub fn add_environment(&mut self, name: String, auth_config: AuthConfig) -> Result<()> {
        info!("Adding environment: {}", name);
        self.environments.insert(name.clone(), auth_config);

        // Set as current environment if it's the first one
        if self.current_environment.is_none() {
            self.current_environment = Some(name.clone());
            info!("Set {} as current environment", name);
        }

        self.save()
    }

    pub fn get_current_auth(&self) -> Option<&AuthConfig> {
        let current_env = self.current_environment.as_ref()?;
        self.environments.get(current_env)
    }

    pub fn get_current_environment_name(&self) -> Option<&String> {
        self.current_environment.as_ref()
    }

    pub fn set_current_environment(&mut self, name: String) -> Result<()> {
        if !self.environments.contains_key(&name) {
            anyhow::bail!("Environment '{}' not found", name);
        }

        info!("Setting current environment to: {}", name);
        self.current_environment = Some(name);
        self.save()
    }

    pub fn list_environments(&self) -> Vec<&String> {
        self.environments.keys().collect()
    }

    pub fn remove_environment(&mut self, name: &str) -> Result<()> {
        if !self.environments.contains_key(name) {
            anyhow::bail!("Environment '{}' not found", name);
        }

        info!("Removing environment: {}", name);
        self.environments.remove(name);

        // If this was the current environment, clear it
        if self.current_environment.as_ref() == Some(&name.to_string()) {
            warn!("Removed current environment, clearing current selection");
            self.current_environment = None;
        }

        self.save()
    }
}