/// Individual step commands for questionnaire copying
/// Each step is executed as a separate Command to allow UI updates between steps

use super::models::{CopyError, CopyPhase};
use super::super::copy::domain::Questionnaire;
use crate::api::{DynamicsClient, ResilienceConfig};
use crate::api::operations::{Operation, Operations};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Helper to get shared entities
fn get_shared_entities() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    set.insert("questiontemplateid");
    set.insert("questiontagid");
    set.insert("categoryid");
    set.insert("domainid");
    set.insert("fundid");
    set.insert("supportid");
    set.insert("typeid");
    set.insert("subcategoryid");
    set.insert("flemishshareid");
    set
}

// Helper to remap lookup fields
fn remap_lookup_fields(
    raw_data: &Value,
    id_map: &HashMap<String, String>,
    shared_entities: &HashSet<&str>,
) -> Result<Value, String> {
    let mut data = raw_data.clone();

    if let Value::Object(ref mut map) = data {
        let mut remapped_fields = Vec::new();

        for (key, value) in map.iter() {
            if key.starts_with('_') && key.ends_with("_value") {
                if let Some(guid) = value.as_str() {
                    let field_name = key.trim_start_matches('_').trim_end_matches("_value");
                    let is_shared = shared_entities.iter().any(|&entity_field| field_name.contains(entity_field));

                    let final_guid = if is_shared {
                        guid.to_string()
                    } else {
                        // No fallback - error if mapping not found
                        id_map.get(guid)
                            .cloned()
                            .ok_or_else(|| format!("ID mapping not found for GUID {} in field {}", guid, field_name))?
                    };

                    let entity_set = infer_entity_set_from_field(field_name)?;

                    remapped_fields.push((
                        key.clone(),
                        format!("{}@odata.bind", field_name),
                        format!("/{}({})", entity_set, final_guid),
                    ));
                }
            }
        }

        for (old_key, new_key, new_value) in remapped_fields {
            map.remove(&old_key);
            map.insert(new_key, json!(new_value));
        }
    }

    Ok(data)
}

fn infer_entity_set_from_field(field_name: &str) -> Result<String, String> {
    if field_name.contains("questionnaireid") {
        Ok("nrq_questionnaires".to_string())
    } else if field_name.contains("questionnairepageid") {
        Ok("nrq_questionnairepages".to_string())
    } else if field_name.contains("questiongroupid") {
        Ok("nrq_questiongroups".to_string())
    } else if field_name.contains("questiontemplateid") {
        Ok("nrq_questiontemplates".to_string())
    } else if field_name.contains("questiontagid") {
        Ok("nrq_questiontags".to_string())
    } else if field_name.contains("questionconditionid") {
        Ok("nrq_questionconditions".to_string())
    } else if field_name.contains("questionid") {
        Ok("nrq_questions".to_string())
    } else if field_name.contains("categoryid") {
        Ok("nrq_categories".to_string())
    } else if field_name.contains("domainid") {
        Ok("nrq_domains".to_string())
    } else if field_name.contains("fundid") {
        Ok("nrq_funds".to_string())
    } else if field_name.contains("supportid") {
        Ok("nrq_supports".to_string())
    } else if field_name.contains("typeid") {
        Ok("nrq_types".to_string())
    } else if field_name.contains("subcategoryid") {
        Ok("nrq_subcategories".to_string())
    } else if field_name.contains("flemishshareid") {
        Ok("nrq_flemishshares".to_string())
    } else {
        Err(format!("Unknown entity field: {} - please add explicit mapping", field_name))
    }
}

fn remap_condition_json(
    condition_json_str: &str,
    id_map: &HashMap<String, String>,
) -> Result<String, String> {
    let mut json: Value = serde_json::from_str(condition_json_str)
        .map_err(|e| format!("Failed to parse condition JSON: {}", e))?;

    // Remap root questionId - REQUIRED
    if let Some(question_id) = json.get("questionId").and_then(|v| v.as_str()) {
        let new_id = id_map.get(question_id)
            .ok_or_else(|| format!("Question ID {} not found in mapping (root questionId)", question_id))?;
        json["questionId"] = json!(new_id);
    }

    // Remap questions array - each is REQUIRED
    if let Some(questions) = json.get_mut("questions").and_then(|v| v.as_array_mut()) {
        for (idx, q) in questions.iter_mut().enumerate() {
            if let Some(question_id) = q.get("questionId").and_then(|v| v.as_str()) {
                let new_id = id_map.get(question_id)
                    .ok_or_else(|| format!("Question ID {} not found in mapping (questions[{}])", question_id, idx))?;
                q["questionId"] = json!(new_id);
            }
        }
    }

    serde_json::to_string(&json)
        .map_err(|e| format!("Failed to serialize condition JSON: {}", e))
}

fn extract_entity_id(result: &crate::api::operations::OperationResult) -> Result<String, String> {
    // Primary method: Extract from OData-EntityId header
    if let Some(entity_id_url) = result.headers.get("OData-EntityId") {
        if let Some(start) = entity_id_url.rfind('(') {
            if let Some(end) = entity_id_url.rfind(')') {
                return Ok(entity_id_url[start + 1..end].to_string());
            }
        }
        return Err(format!("Failed to parse OData-EntityId header: {}", entity_id_url));
    }

    // No fallback - OData-EntityId header should always be present for Create operations
    Err("No OData-EntityId header found in response - this should not happen for Create operations".to_string())
}

pub async fn step1_create_questionnaire(
    questionnaire: Questionnaire,
    copy_name: String,
    copy_code: String,
) -> Result<(String, Vec<(String, String)>), CopyError> {
    let client_manager = crate::client_manager();

    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingQuestionnaire, 1))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1))?;

    let resilience = ResilienceConfig::default();

    let mut data = questionnaire.raw.clone();

    if let Value::Object(ref mut map) = data {
        map.remove("nrq_questionnaireid");
        map.remove("createdon");
        map.remove("modifiedon");
        map.remove("_createdby_value");
        map.remove("_modifiedby_value");
        map.remove("versionnumber");
    }

    data["nrq_name"] = json!(copy_name);
    data["nrq_copypostfix"] = json!(copy_code);

    let operations = Operations::new().create("nrq_questionnaires", data);
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1))?;

    if !results[0].success {
        return Err(build_error(
            results[0].error.clone().unwrap_or_else(|| "Unknown error".to_string()),
            CopyPhase::CreatingQuestionnaire,
            1,
        ));
    }

    let new_id = extract_entity_id(&results[0])
        .map_err(|e| build_error(format!("Failed to extract questionnaire ID: {}", e), CopyPhase::CreatingQuestionnaire, 1))?;

    let created_ids = vec![("nrq_questionnaires".to_string(), new_id.clone())];

    Ok((new_id, created_ids))
}

pub async fn step2_create_pages(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.pages.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPages, 2))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingPages, 2))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPages, 2))?;

    let resilience = ResilienceConfig::default();

    let new_questionnaire_id = id_map.get(&questionnaire.id)
        .ok_or_else(|| build_error("Questionnaire ID not found in map".to_string(), CopyPhase::CreatingPages, 2))?;

    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();

    for page in &questionnaire.pages {
        let mut data = remap_lookup_fields(&page.raw, &id_map, &shared_entities)
            .map_err(|e| build_error(
                format!("Failed to remap page lookup fields: {}", e),
                CopyPhase::CreatingPages,
                2,
            ))?;
        data["nrq_questionnaireid@odata.bind"] = json!(format!("/nrq_questionnaires({})", new_questionnaire_id));

        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questionnairepageid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        operations = operations.create("nrq_questionnairepages", data);
    }

    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPages, 2))?;

    // Validate result count matches expected count
    if results.len() != questionnaire.pages.len() {
        return Err(build_error(
            format!("Result count mismatch: expected {} pages, got {} results",
                questionnaire.pages.len(), results.len()),
            CopyPhase::CreatingPages,
            2,
        ));
    }

    let mut new_id_map = id_map;
    let mut first_error = None;

    // Process ALL results, tracking successes even if some fail
    for (page, result) in questionnaire.pages.iter().zip(results.iter()) {
        if result.success {
            // Track successful creations
            match extract_entity_id(result) {
                Ok(new_id) => {
                    new_id_map.insert(page.id.clone(), new_id.clone());
                    created_ids.push(("nrq_questionnairepages".to_string(), new_id));
                }
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(format!("Failed to extract page ID: {}", e));
                    }
                }
            }
        } else if first_error.is_none() {
            first_error = Some(result.error.clone().unwrap_or_else(|| "Unknown error".to_string()));
        }
    }

    // If any errors occurred, return error with ALL successful IDs tracked
    if let Some(error_msg) = first_error {
        return Err(build_error(error_msg, CopyPhase::CreatingPages, 2));
    }

    Ok((new_id_map, created_ids))
}

fn build_error(message: String, phase: CopyPhase, step: usize) -> CopyError {
    CopyError {
        error_message: message,
        phase,
        step,
        partial_counts: HashMap::new(),
        rollback_complete: false,
    }
}

/// Rollback all created entities in reverse order
/// Returns true if rollback succeeded, false if it failed
pub async fn rollback_created_entities(
    created_ids: Vec<(String, String)>,
) -> bool {
    if created_ids.is_empty() {
        return true; // Nothing to rollback
    }

    log::info!("Starting rollback of {} entities", created_ids.len());

    let client_manager = crate::client_manager();

    // Get client
    let env_name = match client_manager.get_current_environment_name().await {
        Ok(Some(name)) => name,
        _ => {
            log::error!("Rollback failed: Could not get environment name");
            return false;
        }
    };

    let client = match client_manager.get_client(&env_name).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Rollback failed: Could not get client: {}", e);
            return false;
        }
    };

    let resilience = ResilienceConfig::default();
    let mut operations = Operations::new();

    // Delete in REVERSE order (bottom-up to respect dependencies)
    for (entity_set, entity_id) in created_ids.iter().rev() {
        operations = operations.add(Operation::Delete {
            entity: entity_set.clone(),
            id: entity_id.clone(),
        });
    }

    // Execute batch delete
    match operations.execute(&client, &resilience).await {
        Ok(results) => {
            let mut all_success = true;
            for (idx, result) in results.iter().enumerate() {
                if !result.success {
                    let (entity_set, entity_id) = &created_ids[created_ids.len() - 1 - idx];
                    log::error!(
                        "Failed to delete {} ({}): {:?}",
                        entity_set,
                        entity_id,
                        result.error
                    );
                    all_success = false;
                }
            }

            if all_success {
                log::info!("Rollback completed successfully - deleted {} entities", created_ids.len());
            } else {
                log::warn!("Rollback partially failed - some entities may remain");
            }

            all_success
        }
        Err(e) => {
            log::error!("Rollback batch operation failed: {}", e);
            false
        }
    }
}

pub async fn step3_create_page_lines(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.page_lines.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPageLines, 3))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingPageLines, 3))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPageLines, 3))?;

    let resilience = ResilienceConfig::default();
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();

    for page_line in &questionnaire.page_lines {
        let mut data = remap_lookup_fields(page_line, &id_map, &shared_entities)
            .map_err(|e| build_error(
                format!("Failed to remap page line lookup fields: {}", e),
                CopyPhase::CreatingPageLines,
                3,
            ))?;

        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questionnairepagelineid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        operations = operations.create("nrq_questionnairepagelines", data);
    }

    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingPageLines, 3))?;

    let mut first_error = None;

    // Process ALL results, tracking successes even if some fail
    for result in &results {
        if result.success {
            match extract_entity_id(result) {
                Ok(new_id) => {
                    created_ids.push(("nrq_questionnairepagelines".to_string(), new_id));
                }
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(format!("Failed to extract page line ID: {}", e));
                    }
                }
            }
        } else if first_error.is_none() {
            first_error = Some(result.error.clone().unwrap_or_else(|| "Unknown error".to_string()));
        }
    }

    // If any errors occurred, return error with ALL successful IDs tracked
    if let Some(error_msg) = first_error {
        return Err(build_error(error_msg, CopyPhase::CreatingPageLines, 3));
    }

    Ok((id_map, created_ids))
}

pub async fn step4_create_groups(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();
    let mut all_groups = Vec::new();

    for page in &questionnaire.pages {
        for group in &page.groups {
            let mut data = remap_lookup_fields(&group.raw, &id_map, &shared_entities)
                .map_err(|e| build_error(
                    format!("Failed to remap group lookup fields: {}", e),
                    CopyPhase::CreatingGroups,
                    4,
                ))?;

            if let Value::Object(ref mut map) = data {
                map.remove("nrq_questiongroupid");
                map.remove("createdon");
                map.remove("modifiedon");
                map.remove("_createdby_value");
                map.remove("_modifiedby_value");
                map.remove("versionnumber");
            }

            operations = operations.create("nrq_questiongroups", data);
            all_groups.push(group);
        }
    }

    if all_groups.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroups, 4))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingGroups, 4))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroups, 4))?;

    let resilience = ResilienceConfig::default();
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroups, 4))?;

    // Validate result count matches expected count
    if results.len() != all_groups.len() {
        return Err(build_error(
            format!("Result count mismatch: expected {} groups, got {} results",
                all_groups.len(), results.len()),
            CopyPhase::CreatingGroups,
            4,
        ));
    }

    let mut new_id_map = id_map;

    for (group, result) in all_groups.iter().zip(results.iter()) {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingGroups,
                4,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract group ID: {}", e), CopyPhase::CreatingGroups, 4))?;

        new_id_map.insert(group.id.clone(), new_id.clone());
        created_ids.push(("nrq_questiongroups".to_string(), new_id));
    }

    Ok((new_id_map, created_ids))
}

pub async fn step5_create_group_lines(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.group_lines.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroupLines, 5))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingGroupLines, 5))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroupLines, 5))?;

    let resilience = ResilienceConfig::default();
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();

    for group_line in &questionnaire.group_lines {
        let mut data = remap_lookup_fields(group_line, &id_map, &shared_entities)
            .map_err(|e| build_error(
                format!("Failed to remap group line lookup fields: {}", e),
                CopyPhase::CreatingGroupLines,
                5,
            ))?;

        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questiongrouplineid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        operations = operations.create("nrq_questiongrouplines", data);
    }

    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingGroupLines, 5))?;

    for result in &results {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingGroupLines,
                5,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract group line ID: {}", e), CopyPhase::CreatingGroupLines, 5))?;
        created_ids.push(("nrq_questiongrouplines".to_string(), new_id));
    }

    Ok((id_map, created_ids))
}

pub async fn step6_create_questions(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();
    let mut all_questions = Vec::new();

    for page in &questionnaire.pages {
        for group in &page.groups {
            for question in &group.questions {
                let mut data = remap_lookup_fields(&question.raw, &id_map, &shared_entities)
                    .map_err(|e| build_error(
                        format!("Failed to remap question lookup fields: {}", e),
                        CopyPhase::CreatingQuestions,
                        6,
                    ))?;

                if let Value::Object(ref mut map) = data {
                    map.remove("nrq_questionid");
                    map.remove("createdon");
                    map.remove("modifiedon");
                    map.remove("_createdby_value");
                    map.remove("_modifiedby_value");
                    map.remove("versionnumber");
                }

                operations = operations.create("nrq_questions", data);
                all_questions.push(question);
            }
        }
    }

    if all_questions.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestions, 6))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingQuestions, 6))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestions, 6))?;

    let resilience = ResilienceConfig::default();
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestions, 6))?;

    // Validate result count matches expected count
    if results.len() != all_questions.len() {
        return Err(build_error(
            format!("Result count mismatch: expected {} questions, got {} results",
                all_questions.len(), results.len()),
            CopyPhase::CreatingQuestions,
            6,
        ));
    }

    let mut new_id_map = id_map;

    for (question, result) in all_questions.iter().zip(results.iter()) {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingQuestions,
                6,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract question ID: {}", e), CopyPhase::CreatingQuestions, 6))?;

        new_id_map.insert(question.id.clone(), new_id.clone());
        created_ids.push(("nrq_questions".to_string(), new_id));
    }

    Ok((new_id_map, created_ids))
}

pub async fn step7_create_template_lines(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.template_lines.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingTemplateLines, 7))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingTemplateLines, 7))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingTemplateLines, 7))?;

    let resilience = ResilienceConfig::default();
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();

    for template_line in &questionnaire.template_lines {
        let mut data = remap_lookup_fields(&template_line.raw, &id_map, &shared_entities)
            .map_err(|e| build_error(
                format!("Failed to remap template line lookup fields: {}", e),
                CopyPhase::CreatingTemplateLines,
                7,
            ))?;

        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questiontemplatetogrouplineid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        operations = operations.create("nrq_questiontemplatetogrouplines", data);
    }

    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingTemplateLines, 7))?;

    for result in &results {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingTemplateLines,
                7,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract template line ID: {}", e), CopyPhase::CreatingTemplateLines, 7))?;
        created_ids.push(("nrq_questiontemplatetogrouplines".to_string(), new_id));
    }

    Ok((id_map, created_ids))
}

pub async fn step8_create_conditions(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    if questionnaire.conditions.is_empty() {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditions, 8))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingConditions, 8))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditions, 8))?;

    let resilience = ResilienceConfig::default();
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();

    for condition in &questionnaire.conditions {
        let mut data = remap_lookup_fields(&condition.raw, &id_map, &shared_entities)
            .map_err(|e| build_error(
                format!("Failed to remap condition lookup fields: {}", e),
                CopyPhase::CreatingConditions,
                8,
            ))?;

        // CRITICAL: Remap condition JSON with embedded question IDs
        if let Some(condition_json_str) = condition.raw.get("nrq_conditionjson").and_then(|v| v.as_str()) {
            let remapped_json = remap_condition_json(condition_json_str, &id_map)
                .map_err(|e| build_error(
                    format!("Failed to remap condition JSON: {}", e),
                    CopyPhase::CreatingConditions,
                    8,
                ))?;
            data["nrq_conditionjson"] = json!(remapped_json);
        }

        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questionconditionid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        operations = operations.create("nrq_questionconditions", data);
    }

    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditions, 8))?;

    // Validate result count matches expected count
    if results.len() != questionnaire.conditions.len() {
        return Err(build_error(
            format!("Result count mismatch: expected {} conditions, got {} results",
                questionnaire.conditions.len(), results.len()),
            CopyPhase::CreatingConditions,
            8,
        ));
    }

    let mut new_id_map = id_map;

    for (condition, result) in questionnaire.conditions.iter().zip(results.iter()) {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingConditions,
                8,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract condition ID: {}", e), CopyPhase::CreatingConditions, 8))?;

        new_id_map.insert(condition.id.clone(), new_id.clone());
        created_ids.push(("nrq_questionconditions".to_string(), new_id));
    }

    Ok((new_id_map, created_ids))
}

pub async fn step9_create_condition_actions(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    let shared_entities = get_shared_entities();
    let mut operations = Operations::new();
    let mut actions_count = 0;

    for condition in &questionnaire.conditions {
        for action in &condition.actions {
            let mut data = remap_lookup_fields(&action.raw, &id_map, &shared_entities)
                .map_err(|e| build_error(
                    format!("Failed to remap condition action lookup fields: {}", e),
                    CopyPhase::CreatingConditionActions,
                    9,
                ))?;

            if let Value::Object(ref mut map) = data {
                map.remove("nrq_questionconditionactionid");
                map.remove("createdon");
                map.remove("modifiedon");
                map.remove("_createdby_value");
                map.remove("_modifiedby_value");
                map.remove("versionnumber");
            }

            operations = operations.create("nrq_questionconditionactions", data);
            actions_count += 1;
        }
    }

    if actions_count == 0 {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditionActions, 9))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingConditionActions, 9))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditionActions, 9))?;

    let resilience = ResilienceConfig::default();
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingConditionActions, 9))?;

    for result in &results {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingConditionActions,
                9,
            ));
        }

        let new_id = extract_entity_id(result)
            .map_err(|e| build_error(format!("Failed to extract condition action ID: {}", e), CopyPhase::CreatingConditionActions, 9))?;
        created_ids.push(("nrq_questionconditionactions".to_string(), new_id));
    }

    Ok((id_map, created_ids))
}

pub async fn step10_create_classifications(
    questionnaire: Questionnaire,
    id_map: HashMap<String, String>,
    mut created_ids: Vec<(String, String)>,
) -> Result<(HashMap<String, String>, Vec<(String, String)>), CopyError> {
    let new_questionnaire_id = id_map.get(&questionnaire.id)
        .ok_or_else(|| build_error("Questionnaire ID not found in map".to_string(), CopyPhase::CreatingClassifications, 10))?;

    let mut operations = Operations::new();
    let mut classifications_count = 0;

    // Category associations
    for category_ref in &questionnaire.classifications.categories {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Category_nrq_Category".to_string(),
            target_ref: format!("/nrq_categories({})", category_ref.id),
        });
        classifications_count += 1;
    }

    // Domain associations
    for domain_ref in &questionnaire.classifications.domains {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Domain_nrq_Domain".to_string(),
            target_ref: format!("/nrq_domains({})", domain_ref.id),
        });
        classifications_count += 1;
    }

    // Fund associations
    for fund_ref in &questionnaire.classifications.funds {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Fund_nrq_Fund".to_string(),
            target_ref: format!("/nrq_funds({})", fund_ref.id),
        });
        classifications_count += 1;
    }

    // Support associations
    for support_ref in &questionnaire.classifications.supports {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Support_nrq_Support".to_string(),
            target_ref: format!("/nrq_supports({})", support_ref.id),
        });
        classifications_count += 1;
    }

    // Type associations
    for type_ref in &questionnaire.classifications.types {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Type_nrq_Type".to_string(),
            target_ref: format!("/nrq_types({})", type_ref.id),
        });
        classifications_count += 1;
    }

    // Subcategory associations
    for subcategory_ref in &questionnaire.classifications.subcategories {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_Subcategory_nrq_Subcategory".to_string(),
            target_ref: format!("/nrq_subcategories({})", subcategory_ref.id),
        });
        classifications_count += 1;
    }

    // Flemish share associations
    for flemish_share_ref in &questionnaire.classifications.flemish_shares {
        operations = operations.add(Operation::AssociateRef {
            entity: "nrq_questionnaires".to_string(),
            entity_ref: new_questionnaire_id.clone(),
            navigation_property: "nrq_questionnaire_nrq_FlemishShare_nrq_FlemishShare".to_string(),
            target_ref: format!("/nrq_flemishshares({})", flemish_share_ref.id),
        });
        classifications_count += 1;
    }

    if classifications_count == 0 {
        return Ok((id_map, created_ids));
    }

    let client_manager = crate::client_manager();
    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingClassifications, 10))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingClassifications, 10))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingClassifications, 10))?;

    let resilience = ResilienceConfig::default();
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingClassifications, 10))?;

    for result in &results {
        if !result.success {
            return Err(build_error(
                result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
                CopyPhase::CreatingClassifications,
                10,
            ));
        }
    }

    // Note: AssociateRef operations don't create new entities, they just link existing ones
    // So we don't add anything to created_ids here

    Ok((id_map, created_ids))
}
