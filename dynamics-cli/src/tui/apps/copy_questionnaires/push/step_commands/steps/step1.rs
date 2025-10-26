use super::super::entity_sets;
/// Step 1: Create the questionnaire entity

use super::super::super::super::copy::domain::Questionnaire;
use super::super::super::models::{CopyError, CopyPhase};
use super::super::error::build_error;
use super::super::helpers::{extract_entity_id, remove_system_fields};
use crate::api::{ResilienceConfig};
use crate::api::operations::Operations;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn step1_create_questionnaire(
    questionnaire: Arc<Questionnaire>,
    copy_name: String,
    copy_code: String,
) -> Result<(String, Vec<(String, String)>), CopyError> {
    let client_manager = crate::client_manager();

    let env_name = client_manager.get_current_environment_name().await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1, &[]))?
        .ok_or_else(|| build_error("No environment selected".to_string(), CopyPhase::CreatingQuestionnaire, 1, &[]))?;

    let client = client_manager.get_client(&env_name).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1, &[]))?;

    let resilience = ResilienceConfig::default();

    let mut data = questionnaire.raw.clone();
    remove_system_fields(&mut data, "nrq_questionnaireid");

    data["nrq_name"] = json!(copy_name);
    data["nrq_copypostfix"] = json!(copy_code);

    let operations = Operations::new().create(entity_sets::QUESTIONNAIRES, data);
    let results = operations.execute(&client, &resilience).await
        .map_err(|e| build_error(e.to_string(), CopyPhase::CreatingQuestionnaire, 1, &[]))?;

    if !results[0].success {
        return Err(build_error(
            results[0].error.clone().unwrap_or_else(|| "Unknown error".to_string()),
            CopyPhase::CreatingQuestionnaire,
            1,
            &[],
        ));
    }

    let new_id = extract_entity_id(&results[0])
        .map_err(|e| build_error(format!("Failed to extract questionnaire ID: {}", e), CopyPhase::CreatingQuestionnaire, 1, &[]))?;

    let created_ids = vec![(entity_sets::QUESTIONNAIRES.to_string(), new_id.clone())];

    Ok((new_id, created_ids))
}
