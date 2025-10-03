use anyhow::{Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Serializable example pair for configuration persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigExamplePair {
    pub id: String,
    pub source_uuid: String,
    pub target_uuid: String,
    pub label: Option<String>,
}

impl ConfigExamplePair {
    /// Create a new ConfigExamplePair
    pub fn new(source_uuid: String, target_uuid: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_uuid,
            target_uuid,
            label: None,
        }
    }

    /// Create a ConfigExamplePair with a label
    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub host: String,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedMigration {
    pub name: String,
    pub source_env: String,
    pub target_env: String,
    pub comparisons: Vec<SavedComparison>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub last_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedComparison {
    pub name: String,
    pub source_entity: String,
    pub target_entity: String,
    #[serde(default)]
    pub entity_comparison: EntityComparison,
    #[serde(default)]
    pub view_comparisons: Vec<ViewComparison>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub last_used: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityComparison {
    #[serde(default)]
    pub field_mappings: HashMap<String, String>, // manual field mappings
    #[serde(default)]
    pub prefix_mappings: HashMap<String, String>, // bulk prefix rules
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewComparison {
    pub source_view_name: String,
    pub target_view_name: String,
    #[serde(default)]
    pub column_mappings: HashMap<String, String>, // derived from field_mappings
    #[serde(default)]
    pub filter_mappings: HashMap<String, String>, // conditions mapping
    #[serde(default)]
    pub sort_mappings: HashMap<String, String>, // order-by mapping
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub current_environment: Option<String>,
    pub environments: HashMap<String, AuthConfig>,
    #[serde(default)]
    pub entity_mappings: HashMap<String, String>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub migrations: HashMap<String, SavedMigration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_query_limit")]
    pub default_query_limit: u32,
    #[serde(default)]
    pub field_mappings: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub prefix_mappings: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub examples: HashMap<String, Vec<ConfigExamplePair>>, // key: "source_entity:target_entity"
}

fn default_query_limit() -> u32 {
    100
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_query_limit: default_query_limit(),
            field_mappings: HashMap::new(),
            prefix_mappings: HashMap::new(),
            examples: HashMap::new(),
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

        debug!(
            "Loaded config with {} environments",
            config.environments.len()
        );
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        debug!("Saving config to: {:?}", config_path);

        let config_content =
            toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

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

    pub fn get_auth(&self, env_name: &str) -> Option<&AuthConfig> {
        self.environments.get(env_name)
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

    pub fn add_entity_mapping(&mut self, entity_name: String, plural_name: String) -> Result<()> {
        info!("Adding entity mapping: {} -> {}", entity_name, plural_name);
        self.entity_mappings.insert(entity_name, plural_name);
        self.save()
    }

    pub fn get_entity_mapping(&self, entity_name: &str) -> Option<&String> {
        self.entity_mappings.get(entity_name)
    }

    pub fn remove_entity_mapping(&mut self, entity_name: &str) -> Result<()> {
        if self.entity_mappings.remove(entity_name).is_some() {
            info!("Removed entity mapping: {}", entity_name);
            self.save()
        } else {
            anyhow::bail!("Entity mapping '{}' not found", entity_name);
        }
    }

    pub fn list_entity_mappings(&self) -> &HashMap<String, String> {
        &self.entity_mappings
    }

    pub fn get_settings(&self) -> &Settings {
        &self.settings
    }

    pub fn update_default_query_limit(&mut self, limit: u32) -> Result<()> {
        info!("Updating default query limit to: {}", limit);
        self.settings.default_query_limit = limit;
        self.save()
    }

    /// Add a manual field mapping for specific entity comparison
    /// Key format: "source_entity:target_entity" -> {"source_field": "target_field"}
    pub fn add_field_mapping(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        source_field: &str,
        target_field: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        info!(
            "Adding field mapping for {}: {} -> {}",
            mapping_key, source_field, target_field
        );

        self.settings
            .field_mappings
            .entry(mapping_key)
            .or_default()
            .insert(source_field.to_string(), target_field.to_string());

        self.save()
    }

    /// Get field mappings for a specific entity comparison
    pub fn get_field_mappings(
        &self,
        source_entity: &str,
        target_entity: &str,
    ) -> Option<&HashMap<String, String>> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        self.settings.field_mappings.get(&mapping_key)
    }

    /// Remove a specific field mapping
    pub fn remove_field_mapping(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        source_field: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);

        if let Some(entity_mappings) = self.settings.field_mappings.get_mut(&mapping_key) {
            if entity_mappings.remove(source_field).is_some() {
                info!(
                    "Removed field mapping for {}: {}",
                    mapping_key, source_field
                );
                // Remove the entity mapping if it's empty
                if entity_mappings.is_empty() {
                    self.settings.field_mappings.remove(&mapping_key);
                }
                self.save()
            } else {
                anyhow::bail!(
                    "Field mapping '{}' not found for entity comparison '{}'",
                    source_field,
                    mapping_key
                );
            }
        } else {
            anyhow::bail!(
                "No field mappings found for entity comparison '{}'",
                mapping_key
            );
        }
    }

    /// List all field mappings
    pub fn list_field_mappings(&self) -> &HashMap<String, HashMap<String, String>> {
        &self.settings.field_mappings
    }

    /// Add a prefix mapping for entity field comparisons
    pub fn add_prefix_mapping(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        source_prefix: &str,
        target_prefix: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        info!(
            "Adding prefix mapping for {}: {} -> {}",
            mapping_key, source_prefix, target_prefix
        );

        self.settings
            .prefix_mappings
            .entry(mapping_key)
            .or_default()
            .insert(source_prefix.to_string(), target_prefix.to_string());

        self.save()
    }

    /// Get prefix mappings for a specific entity comparison
    pub fn get_prefix_mappings(
        &self,
        source_entity: &str,
        target_entity: &str,
    ) -> Option<&HashMap<String, String>> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        self.settings.prefix_mappings.get(&mapping_key)
    }

    /// Remove a specific prefix mapping
    pub fn remove_prefix_mapping(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        source_prefix: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);

        if let Some(entity_mappings) = self.settings.prefix_mappings.get_mut(&mapping_key) {
            if entity_mappings.remove(source_prefix).is_some() {
                info!(
                    "Removed prefix mapping for {}: {}",
                    mapping_key, source_prefix
                );
                // Remove the entity mapping if it's empty
                if entity_mappings.is_empty() {
                    self.settings.prefix_mappings.remove(&mapping_key);
                }
                self.save()
            } else {
                anyhow::bail!(
                    "Prefix mapping '{}' not found for entity comparison '{}'",
                    source_prefix,
                    mapping_key
                );
            }
        } else {
            anyhow::bail!(
                "No prefix mappings found for entity comparison '{}'",
                mapping_key
            );
        }
    }

    /// List all prefix mappings
    pub fn list_prefix_mappings(&self) -> &HashMap<String, HashMap<String, String>> {
        &self.settings.prefix_mappings
    }

    // Migration management methods

    /// Save a migration configuration
    pub fn save_migration(&mut self, migration: SavedMigration) -> Result<()> {
        info!("Saving migration: {}", migration.name);
        self.migrations.insert(migration.name.clone(), migration);
        self.save()
    }

    /// Get a migration by name
    pub fn get_migration(&self, name: &str) -> Option<&SavedMigration> {
        self.migrations.get(name)
    }

    /// List all saved migrations
    pub fn list_migrations(&self) -> Vec<&SavedMigration> {
        let mut migrations: Vec<&SavedMigration> = self.migrations.values().collect();
        // Sort by last_used, then by name
        migrations.sort_by(|a, b| {
            b.last_used
                .cmp(&a.last_used)
                .then_with(|| a.name.cmp(&b.name))
        });
        migrations
    }

    /// Remove a migration
    pub fn remove_migration(&mut self, name: &str) -> Result<()> {
        if self.migrations.remove(name).is_some() {
            info!("Removed migration: {}", name);
            self.save()
        } else {
            anyhow::bail!("Migration '{}' not found", name);
        }
    }

    /// Update last used timestamp for a migration
    pub fn touch_migration(&mut self, name: &str) -> Result<()> {
        if let Some(migration) = self.migrations.get_mut(name) {
            migration.last_used = chrono::Utc::now().to_rfc3339();
            self.save()
        } else {
            anyhow::bail!("Migration '{}' not found", name);
        }
    }

    /// Add a comparison to a migration
    pub fn add_comparison_to_migration(
        &mut self,
        migration_name: &str,
        comparison: SavedComparison,
    ) -> Result<()> {
        if let Some(migration) = self.migrations.get_mut(migration_name) {
            info!(
                "Adding comparison '{}' to migration '{}'",
                comparison.name, migration_name
            );
            migration.comparisons.push(comparison);
            migration.last_used = chrono::Utc::now().to_rfc3339();
            self.save()
        } else {
            anyhow::bail!("Migration '{}' not found", migration_name);
        }
    }

    /// Remove a comparison from a migration
    pub fn remove_comparison_from_migration(
        &mut self,
        migration_name: &str,
        comparison_name: &str,
    ) -> Result<()> {
        if let Some(migration) = self.migrations.get_mut(migration_name) {
            let original_len = migration.comparisons.len();
            migration.comparisons.retain(|c| c.name != comparison_name);

            if migration.comparisons.len() < original_len {
                info!(
                    "Removed comparison '{}' from migration '{}'",
                    comparison_name, migration_name
                );
                migration.last_used = chrono::Utc::now().to_rfc3339();
                self.save()
            } else {
                anyhow::bail!(
                    "Comparison '{}' not found in migration '{}'",
                    comparison_name,
                    migration_name
                );
            }
        } else {
            anyhow::bail!("Migration '{}' not found", migration_name);
        }
    }

    // Examples management methods

    /// Add an example pair for a specific entity comparison
    pub fn add_example(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        example: ConfigExamplePair,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        info!(
            "Adding example for {}: {} -> {}",
            mapping_key, example.source_uuid, example.target_uuid
        );

        self.settings
            .examples
            .entry(mapping_key)
            .or_default()
            .push(example);

        self.save()
    }

    /// Get all examples for a specific entity comparison
    pub fn get_examples(
        &self,
        source_entity: &str,
        target_entity: &str,
    ) -> Option<&Vec<ConfigExamplePair>> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        self.settings.examples.get(&mapping_key)
    }

    /// Remove an example by ID for a specific entity comparison
    pub fn remove_example(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        example_id: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);

        if let Some(examples) = self.settings.examples.get_mut(&mapping_key) {
            let original_len = examples.len();
            examples.retain(|e| e.id != example_id);

            if examples.len() < original_len {
                info!(
                    "Removed example {} for {}",
                    example_id, mapping_key
                );
                // Remove the entity examples if it's empty
                if examples.is_empty() {
                    self.settings.examples.remove(&mapping_key);
                }
                self.save()
            } else {
                anyhow::bail!(
                    "Example '{}' not found for entity comparison '{}'",
                    example_id,
                    mapping_key
                );
            }
        } else {
            anyhow::bail!(
                "No examples found for entity comparison '{}'",
                mapping_key
            );
        }
    }

    /// Update all examples for a specific entity comparison
    pub fn update_examples(
        &mut self,
        source_entity: &str,
        target_entity: &str,
        examples: Vec<ConfigExamplePair>,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);
        info!(
            "Updating {} examples for {}",
            examples.len(),
            mapping_key
        );

        if examples.is_empty() {
            // Remove the entry if no examples
            self.settings.examples.remove(&mapping_key);
        } else {
            self.settings.examples.insert(mapping_key, examples);
        }

        self.save()
    }

    /// List all examples for all entity comparisons
    pub fn list_all_examples(&self) -> &HashMap<String, Vec<ConfigExamplePair>> {
        &self.settings.examples
    }

    /// Remove all examples for a specific entity comparison
    pub fn clear_examples(
        &mut self,
        source_entity: &str,
        target_entity: &str,
    ) -> Result<()> {
        let mapping_key = format!("{}:{}", source_entity, target_entity);

        if self.settings.examples.remove(&mapping_key).is_some() {
            info!("Cleared all examples for {}", mapping_key);
            self.save()
        } else {
            anyhow::bail!(
                "No examples found for entity comparison '{}'",
                mapping_key
            );
        }
    }
}
