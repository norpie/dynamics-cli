/// Step 9: Create condition actions

use super::super::super::super::copy::domain::Questionnaire;
use super::super::super::models::{CopyError, CopyPhase};
use super::super::execution::{execute_creation_step, process_creation_results, EntityInfo};
use super::super::helpers::{get_shared_entities, remap_lookup_fields, remove_system_fields};
use crate::api::operations::Operations;
use std::collections::HashMap;
use std::sync::Arc;

pub async fn step9_create_condition_actions(
    questionnaire: Arc<Questionnaire>,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    // Count total actions across all conditions
    let actions_count: usize = questionnaire.conditions.iter().map(|c| c.actions.len()).sum();
    if actions_count == 0 {
        return Ok((id_map, created_ids));
    }

    let mut new_id_map = id_map.clone();

    let (results, entity_info) = execute_creation_step(
        Arc::clone(&questionnaire),
        id_map,
        &mut created_ids,
        CopyPhase::CreatingConditionActions,
        9,
        actions_count,
        |q, id_map| {
            let shared_entities = get_shared_entities();
            let mut operations = Operations::new();
            let mut entity_info = Vec::new();

            for condition in &q.conditions {
                for action in &condition.actions {
                    let mut data = remap_lookup_fields(&action.raw, &id_map, &shared_entities)
                        .map_err(|e| format!("Failed to remap condition action lookup fields: {}", e))?;

                    remove_system_fields(&mut data, "nrq_questionconditionactionid");

                    operations = operations.create("nrq_questionconditionactions", data);
                    entity_info.push(EntityInfo {
                        old_id: None,  // No ID mapping needed for condition actions
                        entity_set: "nrq_questionconditionactions".to_string(),
                    });
                }
            }

            Ok((operations, entity_info))
        }
    ).await?;

    process_creation_results(&results, entity_info, &mut new_id_map, &mut created_ids, CopyPhase::CreatingConditionActions, 9)?;

    Ok((new_id_map, created_ids))
}
