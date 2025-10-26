use super::super::entity_sets;
/// Step 8: Create conditions

use super::super::super::super::copy::domain::Questionnaire;
use super::super::super::models::{CopyError, CopyPhase};
use super::super::execution::{execute_creation_step, process_creation_results, EntityInfo};
use super::super::helpers::{get_shared_entities, remap_lookup_fields, remap_condition_json, remove_system_fields};
use crate::api::operations::Operations;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn step8_create_conditions(
    questionnaire: Arc<Questionnaire>,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.conditions.is_empty() {
        return Ok((id_map, created_ids));
    }

    let expected_count = questionnaire.conditions.len();
    let mut new_id_map = id_map.clone();

    let (results, entity_info) = execute_creation_step(
        Arc::clone(&questionnaire),
        id_map,
        &mut created_ids,
        CopyPhase::CreatingConditions,
        8,
        expected_count,
        |q, id_map| {
            let shared_entities = get_shared_entities();
            let mut operations = Operations::new();
            let mut entity_info = Vec::new();

            for condition in &q.conditions {
                let mut data = remap_lookup_fields(&condition.raw, &id_map, &shared_entities)
                    .map_err(|e| format!("Failed to remap condition lookup fields: {}", e))?;

                // CRITICAL: Remap condition JSON with embedded question IDs
                if let Some(condition_json_str) = condition.raw.get("nrq_conditionjson").and_then(|v| v.as_str()) {
                    let remapped_json = remap_condition_json(condition_json_str, &id_map)
                        .map_err(|e| format!("Failed to remap condition JSON: {}", e))?;
                    data["nrq_conditionjson"] = json!(remapped_json);
                }

                remove_system_fields(&mut data, "nrq_questionconditionid");

                operations = operations.create(entity_sets::CONDITIONS, data);
                entity_info.push(EntityInfo {
                    old_id: Some(condition.id.clone()),
                    entity_set: entity_sets::CONDITIONS.to_string(),
                });
            }

            Ok((operations, entity_info))
        }
    ).await?;

    process_creation_results(&results, entity_info, &mut new_id_map, &mut created_ids, CopyPhase::CreatingConditions, 8)?;

    Ok((new_id_map, created_ids))
}
