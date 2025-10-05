//! Entity metadata fetching with caching

use super::FetchedData;

/// Type of data to fetch
pub enum FetchType {
    SourceFields,
    SourceForms,
    SourceViews,
    TargetFields,
    TargetForms,
    TargetViews,
}

/// Fetch specific metadata type with optional 12-hour caching
pub async fn fetch_with_cache(
    environment_name: &str,
    entity_name: &str,
    fetch_type: FetchType,
    use_cache: bool,
) -> Result<FetchedData, String> {
    let config = crate::global_config();
    let manager = crate::client_manager();

    // Check cache first (12 hours) - use full metadata cache, only if use_cache is true
    if use_cache {
        let cached_metadata = config.get_entity_metadata_cache(environment_name, entity_name, 12).await
            .ok()
            .flatten();

        // If we have cached metadata, extract the requested type and process it
        if let Some(cached) = cached_metadata {
            return match fetch_type {
                FetchType::SourceFields => {
                    let fields = process_lookup_fields(cached.fields);
                    Ok(FetchedData::SourceFields(fields))
                }
                FetchType::SourceForms => Ok(FetchedData::SourceForms(cached.forms)),
                FetchType::SourceViews => Ok(FetchedData::SourceViews(cached.views)),
                FetchType::TargetFields => {
                    let fields = process_lookup_fields(cached.fields);
                    Ok(FetchedData::TargetFields(fields))
                }
                FetchType::TargetForms => Ok(FetchedData::TargetForms(cached.forms)),
                FetchType::TargetViews => Ok(FetchedData::TargetViews(cached.views)),
            };
        }
    }

    // Fetch from API
    let client = manager.get_client(environment_name).await
        .map_err(|e| e.to_string())?;

    match fetch_type {
        FetchType::SourceFields => {
            let mut fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
            fields = process_lookup_fields(fields);
            Ok(FetchedData::SourceFields(fields))
        }
        FetchType::SourceForms => {
            let forms = client.fetch_entity_forms(entity_name).await.map_err(|e| e.to_string())?;
            Ok(FetchedData::SourceForms(forms))
        }
        FetchType::SourceViews => {
            let views = client.fetch_entity_views(entity_name).await.map_err(|e| e.to_string())?;
            Ok(FetchedData::SourceViews(views))
        }
        FetchType::TargetFields => {
            let mut fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
            fields = process_lookup_fields(fields);
            Ok(FetchedData::TargetFields(fields))
        }
        FetchType::TargetForms => {
            let forms = client.fetch_entity_forms(entity_name).await.map_err(|e| e.to_string())?;
            Ok(FetchedData::TargetForms(forms))
        }
        FetchType::TargetViews => {
            let views = client.fetch_entity_views(entity_name).await.map_err(|e| e.to_string())?;
            Ok(FetchedData::TargetViews(views))
        }
    }
}

/// Extract relationships from field list
pub fn extract_relationships(fields: &[crate::api::metadata::FieldMetadata]) -> Vec<crate::api::metadata::RelationshipMetadata> {
    fields.iter()
        .filter(|f| matches!(f.field_type, crate::api::metadata::FieldType::Lookup))
        .map(|f| crate::api::metadata::RelationshipMetadata {
            name: f.logical_name.clone(),
            relationship_type: crate::api::metadata::RelationshipType::ManyToOne,
            related_entity: f.related_entity.clone().unwrap_or_default(),
            related_attribute: f.logical_name.clone(),
        })
        .collect()
}

/// Process lookup fields by filtering out _*_value virtual fields and ensuring
/// base lookup fields are properly typed
fn process_lookup_fields(fields: Vec<crate::api::metadata::FieldMetadata>) -> Vec<crate::api::metadata::FieldMetadata> {
    use std::collections::HashMap;

    // Build map of _*_value fields to their related entity info
    let mut value_field_map: HashMap<String, Option<String>> = HashMap::new();

    for field in &fields {
        // Detect _*_value pattern
        if field.logical_name.starts_with('_') && field.logical_name.ends_with("_value") {
            // Extract base field name: _cgk_deadlineid_value -> cgk_deadlineid
            if let Some(base_name) = field.logical_name
                .strip_prefix('_')
                .and_then(|s| s.strip_suffix("_value"))
            {
                value_field_map.insert(base_name.to_string(), field.related_entity.clone());
            }
        }
    }

    // Filter and update fields
    fields.into_iter()
        .filter_map(|mut field| {
            // Filter out _*_value fields
            if field.logical_name.starts_with('_') && field.logical_name.ends_with("_value") {
                log::debug!("Filtering out virtual lookup value field: {}", field.logical_name);
                return None;
            }

            // Check if this field has a corresponding _*_value field
            if let Some(related_entity) = value_field_map.get(&field.logical_name) {
                // Ensure field is marked as Lookup
                if !matches!(field.field_type, crate::api::metadata::FieldType::Lookup) {
                    log::debug!("Converting field {} to Lookup type (found _*_value)", field.logical_name);
                    field.field_type = crate::api::metadata::FieldType::Lookup;
                }

                // Update related_entity if we extracted it from the _value field
                if field.related_entity.is_none() && related_entity.is_some() {
                    field.related_entity = related_entity.clone();
                }
            }

            Some(field)
        })
        .collect()
}
