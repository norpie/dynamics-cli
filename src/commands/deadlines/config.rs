use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use dirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineConfig {
    #[serde(flatten)]
    pub environments: HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub prefix: String,
    pub main_entity: String,
    pub entities: HashMap<String, EntityMapping>,
    pub board_meeting: Option<BoardMeetingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardMeetingConfig {
    pub entity_type: String,        // "cgk_deadline" or "nrq_boardofdirectorsmeeting"
    pub csv_name: String,           // "bestuur_deadlines.csv" or "boardofdirectorsmeeting.csv"
    pub relationship_field: String, // Field name for the relationship
    pub lookup_by: String,          // "date" or "name"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMapping {
    pub entity: String,
    pub id_field: String,
    pub name_field: String,
    pub endpoint: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredEntity {
    pub name: String,
    pub record_count: usize,
    pub fields: Vec<String>,
}

impl DeadlineConfig {
    pub fn new() -> Self {
        Self {
            environments: HashMap::new(),
        }
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;

        if !config_path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: DeadlineConfig = toml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse deadline.toml: {}", e))?;

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| anyhow!("Failed to serialize deadline config: {}", e))?;

        std::fs::write(&config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?;

        Ok(config_dir.join("dynamics-cli").join("deadline.toml"))
    }

    pub fn get_environment(&self, env_name: &str) -> Option<&EnvironmentConfig> {
        self.environments.get(env_name)
    }

    pub fn has_environment(&self, env_name: &str) -> bool {
        self.environments.contains_key(env_name)
    }

    pub fn add_environment(&mut self, env_name: String, config: EnvironmentConfig) {
        self.environments.insert(env_name, config);
    }

    pub fn remove_environment(&mut self, env_name: &str) {
        self.environments.remove(env_name);
    }

    pub fn list_environments(&self) -> Vec<&String> {
        self.environments.keys().collect()
    }
}

impl EnvironmentConfig {
    pub fn new(prefix: String, main_entity: String) -> Self {
        // Auto-detect board meeting configuration based on prefix
        let board_meeting = Self::detect_board_meeting_config(&prefix);

        Self {
            prefix,
            main_entity,
            entities: HashMap::new(),
            board_meeting,
        }
    }

    fn detect_board_meeting_config(prefix: &str) -> Option<BoardMeetingConfig> {
        match prefix {
            p if p.starts_with("cgk") => Some(BoardMeetingConfig {
                entity_type: "cgk_deadline".to_string(),
                csv_name: "bestuur_deadlines.csv".to_string(),
                relationship_field: "cgk_raadvanbestuur_cgk_deadline".to_string(),
                lookup_by: "date".to_string(),
            }),
            p if p.starts_with("nrq") => Some(BoardMeetingConfig {
                entity_type: "nrq_boardofdirectorsmeeting".to_string(),
                csv_name: "boardofdirectorsmeeting.csv".to_string(),
                relationship_field: "nrq_boardmeetingid".to_string(),
                lookup_by: "date".to_string(),
            }),
            _ => None, // Unknown prefix, no board meeting support
        }
    }

    pub fn add_entity_mapping(&mut self, logical_name: String, mapping: EntityMapping) {
        self.entities.insert(logical_name, mapping);
    }

    pub fn get_entity_mapping(&self, logical_name: &str) -> Option<&EntityMapping> {
        self.entities.get(logical_name)
    }

    pub fn get_csv_filename(&self, logical_name: &str) -> Option<String> {
        self.entities.get(logical_name)
            .map(|mapping| format!("{}.csv", mapping.entity))
    }

    pub fn get_all_csv_filenames(&self) -> Vec<String> {
        let mut filenames: Vec<String> = self.entities.values()
            .map(|mapping| format!("{}.csv", mapping.entity))
            .collect();

        // Add board meeting CSV if configured
        if let Some(board_config) = &self.board_meeting {
            filenames.push(board_config.csv_name.clone());
        }

        filenames
    }

    pub fn has_board_meeting_support(&self) -> bool {
        self.board_meeting.is_some()
    }

    pub fn get_board_meeting_config(&self) -> Option<&BoardMeetingConfig> {
        self.board_meeting.as_ref()
    }

    pub fn is_cgk_environment(&self) -> bool {
        self.prefix.starts_with("cgk_")
    }

    pub fn is_nrq_environment(&self) -> bool {
        self.prefix.starts_with("nrq_")
    }
}

impl EntityMapping {
    pub fn new(entity: String, id_field: String, name_field: String, endpoint: String) -> Self {
        Self {
            entity,
            id_field,
            name_field,
            endpoint,
        }
    }

    pub fn generate_fetchxml(&self) -> String {
        format!(
            r#"<fetch>
  <entity name="{}">
    <attribute name="{}" />
    <attribute name="{}" />
    <filter type="and">
      <condition attribute="statecode" operator="eq" value="0" />
    </filter>
  </entity>
</fetch>"#,
            self.entity, self.name_field, self.id_field
        )
    }
}

impl DiscoveredEntity {
    pub fn new(name: String, record_count: usize, fields: Vec<String>) -> Self {
        Self {
            name,
            record_count,
            fields,
        }
    }

    pub fn has_required_fields(&self, id_suffix: &str, name_field: &str) -> bool {
        let expected_id_field = format!("{}id", &self.name);
        self.fields.contains(&expected_id_field) && self.fields.contains(&name_field.to_string())
    }

    pub fn guess_id_field(&self) -> Option<String> {
        self.fields.iter()
            .find(|field| field.ends_with("id") && field.starts_with(&self.name))
            .cloned()
    }

    pub fn guess_name_field(&self) -> Option<String> {
        // Special handling for systemuser - always use domainname for email lookups
        if self.name == "systemuser" {
            return Some("domainname".to_string());
        }

        // Priority order: exact matches first, then broader patterns
        let exact_patterns = if self.name.starts_with("nrq_") {
            // For NRQ entities, try multiple naming patterns
            // Extract the entity name part after the prefix
            let entity_part = self.name.strip_prefix("nrq_").unwrap_or(&self.name);
            vec![
                format!("nrq_{}name", entity_part),  // e.g., nrq_deadlinename, nrq_categoryname
                "nrq_name".to_string(),              // Generic nrq_name
                format!("{}_name", self.name),       // e.g., nrq_deadline_name
                "name".to_string(),                  // Generic name
            ]
        } else if self.name.starts_with("cgk_") {
            // For CGK entities, use the existing pattern
            vec![
                format!("{}_name", self.name.replace("cgk_", "cgk_")),
                "cgk_name".to_string(),
                "name".to_string(),
            ]
        } else {
            // For other entities, use generic pattern
            vec![
                format!("{}_name", self.name),
                "name".to_string(),
            ]
        };

        let partial_patterns = ["title", "displayname"];

        // First try exact name patterns
        for pattern in &exact_patterns {
            if let Some(field) = self.fields.iter()
                .find(|field| *field == pattern)
                .cloned()
            {
                return Some(field);
            }
        }

        // Then try partial patterns (but exclude fields ending in "idname" which are relationships)
        for pattern in &partial_patterns {
            if let Some(field) = self.fields.iter()
                .find(|field| field.contains(pattern) && !field.ends_with("idname"))
                .cloned()
            {
                return Some(field);
            }
        }

        // Finally, fallback to any field containing "name" but exclude system fields
        self.fields.iter()
            .find(|field| {
                field.contains("name")
                && !field.ends_with("idname")
                && *field != "createdbyname"   // Exclude system field
                && *field != "modifiedbyname"  // Exclude system field
                && *field != "owneridname"     // Exclude system field
            })
            .cloned()
    }
}

// Common logical entity types for deadline management with aliases
pub const COMMON_ENTITY_TYPES: &[(&str, &[&str])] = &[
    ("category", &["category"]),
    ("commission", &["commission"]),
    ("support", &["support"]),
    ("length", &["length"]), // sub-categories
    ("fund", &["fund"]),
    ("pillar", &["pillar", "domain"]), // pillar aka domain
    ("flemish_share", &["flemish_share", "flemishshare"]),
    ("systemuser", &["systemuser"]),
];

