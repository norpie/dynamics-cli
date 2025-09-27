use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use log::debug;

use crate::dynamics::DynamicsClient;
use super::config::{DiscoveredEntity, EntityMapping};

pub struct EntityDiscovery {
    client: DynamicsClient,
}

impl EntityDiscovery {
    pub fn new(client: DynamicsClient) -> Self {
        Self { client }
    }

    /// Discover entities with a given prefix by querying Dynamics metadata
    pub async fn discover_entities_with_prefix(&mut self, prefix: &str) -> Result<Vec<DiscoveredEntity>> {
        debug!("Starting entity discovery for prefix: '{}'", prefix);

        // Use the reliable approach: get all entities and filter client-side
        // The filtered query approach was giving 501 errors, so we skip it entirely
        debug!("Fetching all entities without filter (more reliable than filtered query)");

        let response = self.client.get("EntityDefinitions?$select=LogicalName,DisplayName").await?;
        debug!("Entity discovery response length: {} characters", response.len());

        let entities_data: Value = serde_json::from_str(&response)?;
        debug!("Parsed entities data has {} entities",
            entities_data["value"].as_array().map_or(0, |arr| arr.len()));

        self.process_entities_response(entities_data, prefix).await
    }

    async fn process_entities_response(&mut self, entities_data: Value, prefix: &str) -> Result<Vec<DiscoveredEntity>> {
        let mut discovered_entities = Vec::new();

        if let Some(entities) = entities_data["value"].as_array() {
            debug!("Found {} entities in response", entities.len());
            for entity in entities {
                if let Some(logical_name) = entity["LogicalName"].as_str() {
                    debug!("Checking entity: {}", logical_name);
                    // Include custom entities with prefix and systemuser for user lookups
                    if logical_name.starts_with(&format!("{}_", prefix)) || logical_name == "systemuser" {
                        debug!("Entity '{}' matches prefix '{}', getting details", logical_name, prefix);
                        match self.get_entity_details(logical_name).await {
                            Ok(details) => {
                                debug!("Successfully got details for entity '{}' with {} fields", logical_name, details.fields.len());
                                discovered_entities.push(details);
                            },
                            Err(e) => {
                                debug!("Warning: Failed to get details for entity {}: {}", logical_name, e);
                                continue;
                            }
                        }
                    } else {
                        debug!("Entity '{}' does not match prefix '{}_'", logical_name, prefix);
                    }
                }
            }
        } else {
            debug!("No 'value' array found in response or it's not an array");
        }

        debug!("Entity discovery complete: found {} entities", discovered_entities.len());
        Ok(discovered_entities)
    }

    /// Get detailed information about a specific entity
    async fn get_entity_details(&mut self, entity_name: &str) -> Result<DiscoveredEntity> {
        // Get entity attributes
        let attributes_query = format!(
            "EntityDefinitions(LogicalName='{}')/Attributes?$select=LogicalName,AttributeType",
            entity_name
        );

        let response = self.client.get(&attributes_query).await?;
        let attributes_data: Value = serde_json::from_str(&response)?;

        let mut fields = Vec::new();
        if let Some(attributes) = attributes_data["value"].as_array() {
            for attr in attributes {
                if let Some(logical_name) = attr["LogicalName"].as_str() {
                    fields.push(logical_name.to_string());
                }
            }
        }

        // Get record count by trying to fetch a few records
        let record_count = match self.get_entity_record_count(entity_name).await {
            Ok(count) => count,
            Err(_) => 0, // If we can't get count, assume 0
        };

        Ok(DiscoveredEntity::new(
            entity_name.to_string(),
            record_count,
            fields,
        ))
    }

    /// Get approximate record count for an entity
    async fn get_entity_record_count(&mut self, entity_name: &str) -> Result<usize> {
        // Get the plural form for the endpoint
        let endpoint = self.get_entity_endpoint(entity_name);

        let query = format!(
            "{}?$select={}id&$filter=statecode eq 0&$count=true&$top=1",
            endpoint,
            entity_name.split('_').collect::<Vec<_>>().join("_")
        );

        let response = self.client.get(&query).await?;
        let data: Value = serde_json::from_str(&response)?;

        if let Some(count) = data["@odata.count"].as_u64() {
            Ok(count as usize)
        } else {
            // Fallback: count the returned records (will be limited by $top)
            if let Some(records) = data["value"].as_array() {
                Ok(records.len())
            } else {
                Ok(0)
            }
        }
    }

    /// Convert entity name to endpoint name (simple pluralization)
    fn get_entity_endpoint(&self, entity_name: &str) -> String {
        Self::get_entity_endpoint_static(entity_name)
    }

    /// Static version of get_entity_endpoint for use without instance
    pub fn get_entity_endpoint_static(entity_name: &str) -> String {
        // Follow English pluralization rules
        if entity_name.ends_with('y') {
            format!("{}ies", &entity_name[..entity_name.len()-1])
        } else {
            format!("{}s", entity_name)
        }
    }

    /// Auto-suggest entity mappings based on common patterns
    pub fn suggest_entity_mappings(&self, entities: &[DiscoveredEntity], logical_types: &[&str]) -> HashMap<String, Option<EntityMapping>> {
        let mut suggestions = HashMap::new();

        for logical_type in logical_types {
            let mut best_match: Option<EntityMapping> = None;

            // Look for entities that contain the logical type name
            for entity in entities {
                if entity.name.contains(logical_type) {
                    if let (Some(id_field), Some(name_field)) = (entity.guess_id_field(), entity.guess_name_field()) {
                        let endpoint = self.get_entity_endpoint(&entity.name);

                        best_match = Some(EntityMapping::new(
                            entity.name.clone(),
                            id_field,
                            name_field,
                            endpoint,
                        ));
                        break; // Take the first match
                    }
                }
            }

            suggestions.insert(logical_type.to_string(), best_match);
        }

        suggestions
    }

    /// Test a single entity mapping by fetching a sample record
    pub async fn test_entity_mapping(&mut self, mapping: &EntityMapping) -> Result<bool> {
        let query = format!(
            "{}?$select={},{}&$filter=statecode eq 0&$top=1",
            mapping.endpoint,
            mapping.id_field,
            mapping.name_field
        );

        match self.client.get(&query).await {
            Ok(response) => {
                let data: Value = serde_json::from_str(&response)?;
                Ok(data["value"].as_array().map_or(false, |arr| !arr.is_empty()))
            }
            Err(e) => {
                debug!("Test failed for entity {}: {}", mapping.entity, e);
                Ok(false)
            }
        }
    }

    /// Fetch all records for an entity mapping with pagination support
    pub async fn fetch_all_records(&mut self, mapping: &EntityMapping) -> Result<Vec<HashMap<String, Value>>> {
        let mut all_records = Vec::new();
        let mut next_link: Option<String> = None;
        let page_size = 5000; // Dynamics 365 max page size

        loop {
            let query = if let Some(next_url) = &next_link {
                // Use the next link provided by OData
                next_url.clone()
            } else {
                // Build initial query
                let filter = if mapping.entity.starts_with("cgk_") || mapping.entity.starts_with("new_") {
                    "&$filter=statecode eq 0"
                } else {
                    "" // No filter for system entities
                };

                // Special handling for systemuser - fetch domainname instead of name field
                let name_field_to_fetch = if mapping.entity == "systemuser" {
                    "domainname"
                } else {
                    &mapping.name_field
                };

                format!(
                    "{}?$select={},{}{}{}",
                    mapping.endpoint,
                    mapping.id_field,
                    name_field_to_fetch,
                    filter,
                    format!("&$top={}", page_size)
                )
            };

            let response = self.client.get(&query).await?;

            let data: Value = serde_json::from_str(&response)?;

            // Process records from this page
            if let Some(value_array) = data["value"].as_array() {
                for record in value_array {
                    if let Some(obj) = record.as_object() {
                        let mut record_map = HashMap::new();
                        for (key, value) in obj {
                            if key != "@odata.etag" {
                                record_map.insert(key.clone(), value.clone());
                            }
                        }
                        all_records.push(record_map);
                    }
                }
            }

            // Check for pagination
            if let Some(next_url) = data["@odata.nextLink"].as_str() {
                next_link = Some(next_url.to_string());
                debug!("Found next page link for {}: {} total records so far", mapping.entity, all_records.len());
            } else {
                // No more pages
                break;
            }
        }

        debug!("Fetched {} total records for entity {}", all_records.len(), mapping.entity);
        Ok(all_records)
    }

    /// Fetch sample records for an entity mapping
    pub async fn fetch_sample_records(&mut self, mapping: &EntityMapping, limit: usize) -> Result<Vec<HashMap<String, Value>>> {
        // System entities like 'systemuser' don't have statecode, custom entities (cgk_*) do
        let filter = if mapping.entity.starts_with("cgk_") || mapping.entity.starts_with("new_") {
            "&$filter=statecode eq 0"
        } else {
            "" // No filter for system entities
        };

        // Special handling for systemuser - fetch domainname instead of name field
        let name_field_to_fetch = if mapping.entity == "systemuser" {
            "domainname"
        } else {
            &mapping.name_field
        };

        let query = format!(
            "{}?$select={},{}{}{}",
            mapping.endpoint,
            mapping.id_field,
            name_field_to_fetch,
            filter,
            format!("&$top={}", limit)
        );

        let response = self.client.get(&query).await?;
        let data: Value = serde_json::from_str(&response)?;

        let mut records = Vec::new();
        if let Some(value_array) = data["value"].as_array() {
            for record in value_array {
                if let Some(obj) = record.as_object() {
                    let mut record_map = HashMap::new();
                    for (key, value) in obj {
                        if key != "@odata.etag" {
                            record_map.insert(key.clone(), value.clone());
                        }
                    }
                    records.push(record_map);
                }
            }
        }

        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;

    #[test]
    fn test_entity_endpoint_generation() {
        let auth_config = AuthConfig {
            host: "dummy".to_string(),
            client_id: "dummy".to_string(),
            client_secret: "dummy".to_string(),
            username: "dummy".to_string(),
            password: "dummy".to_string(),
        };
        let discovery = EntityDiscovery::new(DynamicsClient::new(auth_config));

        assert_eq!(discovery.get_entity_endpoint("cgk_support"), "cgk_supports");
        assert_eq!(discovery.get_entity_endpoint("cgk_category"), "cgk_categories");
        assert_eq!(discovery.get_entity_endpoint("cgk_flemishshare"), "cgk_flemishshares");
    }
}