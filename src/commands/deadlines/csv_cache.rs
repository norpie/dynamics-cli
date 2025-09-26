use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use log::debug;
use tokio::sync::oneshot;

use super::config::{EnvironmentConfig, EntityMapping};
use super::entity_discovery::EntityDiscovery;
use crate::dynamics::DynamicsClient;

#[derive(Debug, Clone)]
pub struct CacheStatus {
    pub entity_name: String,
    pub logical_type: String,
    pub status: CacheState,
    pub record_count: usize,
    pub file_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CacheState {
    NotChecked,
    Fresh,
    Stale,
    Missing,
    Fetching,
    Complete,
    Error(String),
}

#[derive(Debug)]
pub struct CsvCacheManager {
    cache_dir: PathBuf,
    environment_name: String,
}

impl CsvCacheManager {
    pub fn new(environment_name: String) -> Self {
        let cache_dir = dirs::config_dir()
            .map(|config| config.join("dynamics-cli").join("csv-cache"))
            .unwrap_or_else(|| PathBuf::from("csv-cache"));
        Self {
            cache_dir,
            environment_name,
        }
    }

    pub fn ensure_cache_dir(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)?;
            debug!("Created cache directory: {:?}", self.cache_dir);
        }
        Ok(())
    }

    pub fn check_cache_status(&self, env_config: &EnvironmentConfig) -> Vec<CacheStatus> {
        let mut statuses = Vec::new();

        debug!("Checking cache status for {} entities", env_config.entities.len());

        for (logical_type, mapping) in &env_config.entities {
            let entity_name = &mapping.entity;
            let file_path = self.get_csv_path(entity_name);
            debug!("Checking cache for entity '{}' at path: {:?}", entity_name, file_path);

            let status = if !file_path.exists() {
                CacheState::Missing
            } else {
                // Check if file is recent (within last day)
                match fs::metadata(&file_path) {
                    Ok(metadata) => {
                        if let Ok(modified) = metadata.modified() {
                            let age = modified.elapsed().unwrap_or_default();
                            debug!("File age for {}: {} seconds", entity_name, age.as_secs());
                            if age.as_secs() < 86400 { // 24 hours
                                debug!("Cache is fresh for {}", entity_name);
                                CacheState::Fresh
                            } else {
                                debug!("Cache is stale for {}", entity_name);
                                CacheState::Stale
                            }
                        } else {
                            debug!("Could not get modified time for {}", entity_name);
                            CacheState::Stale
                        }
                    }
                    Err(e) => {
                        debug!("Could not get metadata for {}: {}", entity_name, e);
                        CacheState::Missing
                    }
                }
            };

            let record_count = if file_path.exists() {
                self.count_csv_records(&file_path).unwrap_or(0)
            } else {
                0
            };

            statuses.push(CacheStatus {
                entity_name: entity_name.clone(),
                logical_type: logical_type.clone(),
                status,
                record_count,
                file_path: Some(file_path),
            });
        }

        statuses
    }

    pub fn needs_refresh(&self, statuses: &[CacheStatus], force: bool) -> bool {
        if force {
            return true;
        }

        statuses.iter().any(|status| {
            matches!(status.status, CacheState::Missing | CacheState::Stale)
        })
    }

    pub async fn refresh_cache(
        &self,
        env_config: &EnvironmentConfig,
        auth_config: &crate::config::AuthConfig,
        status_sender: oneshot::Sender<Vec<CacheStatus>>,
    ) -> Result<()> {
        self.ensure_cache_dir()?;

        let dynamics_client = DynamicsClient::new(auth_config.clone());
        let mut entity_discovery = EntityDiscovery::new(dynamics_client);
        let mut statuses = Vec::new();

        for (logical_type, mapping) in &env_config.entities {
            let entity_name = &mapping.entity;
            debug!("Fetching cache for entity: {}", entity_name);

            let mut cache_status = CacheStatus {
                entity_name: entity_name.clone(),
                logical_type: logical_type.clone(),
                status: CacheState::Fetching,
                record_count: 0,
                file_path: Some(self.get_csv_path(entity_name)),
            };

            // Send intermediate status update
            let mut current_statuses = statuses.clone();
            current_statuses.push(cache_status.clone());
            // Note: We can't send multiple times on oneshot, so we'll just send final result

            match entity_discovery.fetch_sample_records(mapping, 1000).await {
                Ok(records) => {
                    let csv_path = self.get_csv_path(entity_name);
                    match self.write_csv_file(&csv_path, &records, mapping).await {
                        Ok(count) => {
                            cache_status.status = CacheState::Complete;
                            cache_status.record_count = count;
                            debug!("Successfully cached {} records for {}", count, entity_name);
                        }
                        Err(e) => {
                            cache_status.status = CacheState::Error(e.to_string());
                            debug!("Failed to write CSV for {}: {}", entity_name, e);
                        }
                    }
                }
                Err(e) => {
                    cache_status.status = CacheState::Error(e.to_string());
                    debug!("Failed to fetch records for {}: {}", entity_name, e);
                }
            }

            statuses.push(cache_status);
        }

        let _ = status_sender.send(statuses);
        Ok(())
    }

    async fn write_csv_file(
        &self,
        file_path: &Path,
        records: &[HashMap<String, serde_json::Value>],
        mapping: &EntityMapping,
    ) -> Result<usize> {
        use std::io::Write;

        let mut file = fs::File::create(file_path)?;

        // Write header
        writeln!(file, "{},{}", mapping.id_field, mapping.name_field)?;

        // Write records
        let mut count = 0;
        for record in records {
            if let (Some(id), Some(name)) = (
                record.get(&mapping.id_field),
                record.get(&mapping.name_field)
            ) {
                let id_str = id.as_str().unwrap_or_default();
                let name_str = name.as_str().unwrap_or_default();

                // Escape CSV values
                let name_escaped = if name_str.contains(',') || name_str.contains('"') {
                    format!("\"{}\"", name_str.replace('"', "\"\""))
                } else {
                    name_str.to_string()
                };

                writeln!(file, "{},{}", id_str, name_escaped)?;
                count += 1;
            }
        }

        Ok(count)
    }

    fn get_csv_path(&self, entity_name: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.csv", entity_name))
    }

    fn count_csv_records(&self, file_path: &Path) -> Result<usize> {
        let content = fs::read_to_string(file_path)?;
        let line_count = content.lines().count();
        Ok(if line_count > 0 { line_count - 1 } else { 0 }) // Subtract header
    }

    pub fn load_entity_names(&self, entity_name: &str) -> Result<HashMap<String, String>> {
        let csv_path = self.get_csv_path(entity_name);
        let mut lookup = HashMap::new();

        if !csv_path.exists() {
            return Ok(lookup);
        }

        let content = fs::read_to_string(&csv_path)?;
        let mut lines = content.lines();

        // Skip header
        lines.next();

        for line in lines {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let id = parts[0].trim();
                let name = parts[1].trim().trim_matches('"');
                if !id.is_empty() && !name.is_empty() {
                    lookup.insert(name.to_lowercase(), id.to_string());
                }
            }
        }

        Ok(lookup)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_csv_path_generation() {
        let manager = CsvCacheManager::new("test".to_string());
        let path = manager.get_csv_path("cgk_support");
        assert_eq!(path, PathBuf::from(".csv/cgk_support.csv"));
    }

    #[test]
    fn test_count_csv_records() {
        let temp_dir = TempDir::new().unwrap();
        let csv_path = temp_dir.path().join("test.csv");

        fs::write(&csv_path, "id,name\n1,Test1\n2,Test2\n").unwrap();

        let manager = CsvCacheManager::new("test".to_string());
        let count = manager.count_csv_records(&csv_path).unwrap();
        assert_eq!(count, 2);
    }
}