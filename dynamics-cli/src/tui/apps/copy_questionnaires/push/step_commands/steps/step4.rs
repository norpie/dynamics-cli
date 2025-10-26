/// Step 4: Create question groups

use super::super::super::super::copy::domain::Questionnaire;
use super::super::super::models::{CopyError, CopyPhase};
use super::super::execution::{execute_creation_step, process_creation_results, EntityInfo};
use super::super::helpers::{get_shared_entities, remap_lookup_fields, remove_system_fields};
use crate::api::operations::Operations;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn step4_create_groups(
    questionnaire: Arc<Questionnaire>,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    // Count total groups across all pages
    let groups_count: usize = questionnaire.pages.iter().map(|p| p.groups.len()).sum();
    if groups_count == 0 {
        return Ok((id_map, created_ids));
    }

    let mut new_id_map = id_map.clone();

    let (results, entity_info) = execute_creation_step(
        Arc::clone(&questionnaire),
        id_map,
        &mut created_ids,
        CopyPhase::CreatingGroups,
        4,
        groups_count,
        |q, id_map| {
            let shared_entities = get_shared_entities();
            let mut operations = Operations::new();
            let mut entity_info = Vec::new();

            for page in &q.pages {
                for group in &page.groups {
                    let mut data = remap_lookup_fields(&group.raw, &id_map, &shared_entities)
                        .map_err(|e| format!("Failed to remap group lookup fields: {}", e))?;

                    remove_system_fields(&mut data, "nrq_questiongroupid");

                    operations = operations.create("nrq_questiongroups", data);
                    entity_info.push(EntityInfo {
                        old_id: Some(group.id.clone()),
                        entity_set: "nrq_questiongroups".to_string(),
                    });
                }
            }

            Ok((operations, entity_info))
        }
    ).await?;

    process_creation_results(&results, entity_info, &mut new_id_map, &mut created_ids, CopyPhase::CreatingGroups, 4)?;

    Ok((new_id_map, created_ids))
}
