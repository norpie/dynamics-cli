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

        // If we have cached metadata, extract the requested type
        if let Some(cached) = cached_metadata {
            return match fetch_type {
                FetchType::SourceFields => Ok(FetchedData::SourceFields(cached.fields)),
                FetchType::SourceForms => Ok(FetchedData::SourceForms(cached.forms)),
                FetchType::SourceViews => Ok(FetchedData::SourceViews(cached.views)),
                FetchType::TargetFields => Ok(FetchedData::TargetFields(cached.fields)),
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
            let fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
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
            let fields = client.fetch_entity_fields(entity_name).await.map_err(|e| e.to_string())?;
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
