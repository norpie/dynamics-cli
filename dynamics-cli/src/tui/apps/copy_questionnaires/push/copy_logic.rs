use super::models::{CopyProgress, CopyResult, CopyError, CopyPhase, Msg};
use super::super::copy::domain::Questionnaire;
use crate::api::{DynamicsClient, ResilienceConfig};
use crate::api::operations::{Operation, Operations};
use crate::api::pluralization::pluralize_entity_name;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Engine for copying questionnaires with progress tracking
pub struct CopyEngine {
    client: Arc<DynamicsClient>,
    resilience: Arc<ResilienceConfig>,
    /// Maps old entity ID to new entity ID
    id_map: HashMap<String, String>,
    /// Tracks created entities for rollback (entity_set, id)
    created_ids: Vec<(String, String)>,
}

impl CopyEngine {
    pub fn new(client: Arc<DynamicsClient>, resilience: Arc<ResilienceConfig>) -> Self {
        Self {
            client,
            resilience,
            id_map: HashMap::new(),
            created_ids: Vec::new(),
        }
    }

    /// Main entry point - copy entire questionnaire
    pub async fn copy_questionnaire(
        &mut self,
        questionnaire: &Questionnaire,
        copy_name: &str,
        copy_code: &str,
        progress_sender: Sender<Msg>,
    ) -> Result<CopyResult, CopyError> {
        let start_time = std::time::Instant::now();
        let mut progress = CopyProgress::new(questionnaire);

        // Step 1: Create Questionnaire
        match self.create_questionnaire(questionnaire, copy_name, copy_code, &mut progress, &progress_sender).await {
            Ok(new_q_id) => {
                log::info!("Created questionnaire with ID: {}", new_q_id);
                self.id_map.insert(questionnaire.id.clone(), new_q_id.clone());
            }
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingQuestionnaire, 1, &progress).await),
        }

        // Step 2: Create Pages
        match self.create_pages(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => log::info!("Created {} pages", questionnaire.pages.len()),
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingPages, 2, &progress).await),
        }

        // Step 3: Create Page Lines
        match self.create_page_lines(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => log::info!("Created {} page lines", questionnaire.page_lines.len()),
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingPageLines, 3, &progress).await),
        }

        // Step 4: Create Groups
        match self.create_groups(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => {
                let groups_count: usize = questionnaire.pages.iter().map(|p| p.groups.len()).sum();
                log::info!("Created {} groups", groups_count);
            }
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingGroups, 4, &progress).await),
        }

        // Step 5: Create Group Lines
        match self.create_group_lines(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => log::info!("Created {} group lines", questionnaire.group_lines.len()),
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingGroupLines, 5, &progress).await),
        }

        // Step 6: Create Questions
        match self.create_questions(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => {
                let questions_count: usize = questionnaire.pages.iter()
                    .flat_map(|p| &p.groups)
                    .map(|g| g.questions.len())
                    .sum();
                log::info!("Created {} questions", questions_count);
            }
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingQuestions, 6, &progress).await),
        }

        // Step 7: Create Template Lines
        match self.create_template_lines(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => log::info!("Created {} template lines", questionnaire.template_lines.len()),
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingTemplateLines, 7, &progress).await),
        }

        // Step 8: Create Conditions
        match self.create_conditions(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => log::info!("Created {} conditions", questionnaire.conditions.len()),
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingConditions, 8, &progress).await),
        }

        // Step 9: Create Condition Actions
        match self.create_condition_actions(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => {
                let actions_count: usize = questionnaire.conditions.iter().map(|c| c.actions.len()).sum();
                log::info!("Created {} condition actions", actions_count);
            }
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingConditionActions, 9, &progress).await),
        }

        // Step 10: Create Classifications (N:N)
        match self.create_classifications(questionnaire, &mut progress, &progress_sender).await {
            Ok(_) => {
                let classifications_count =
                    questionnaire.classifications.categories.len() +
                    questionnaire.classifications.domains.len() +
                    questionnaire.classifications.funds.len() +
                    questionnaire.classifications.supports.len() +
                    questionnaire.classifications.types.len() +
                    questionnaire.classifications.subcategories.len() +
                    questionnaire.classifications.flemish_shares.len();
                log::info!("Created {} classification associations", classifications_count);
            }
            Err(e) => return Err(self.build_error(e, CopyPhase::CreatingClassifications, 10, &progress).await),
        }

        let duration = start_time.elapsed();
        let new_questionnaire_id = self.id_map.get(&questionnaire.id).unwrap().clone();

        Ok(CopyResult {
            new_questionnaire_id,
            new_questionnaire_name: copy_name.to_string(),
            total_entities: progress.total_entities,
            entities_created: self.build_entity_counts(&progress),
            duration,
        })
    }

    // ============================================================================
    // Step 1: Create Questionnaire
    // ============================================================================

    async fn create_questionnaire(
        &mut self,
        questionnaire: &Questionnaire,
        copy_name: &str,
        copy_code: &str,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<String, String> {
        progress.phase = CopyPhase::CreatingQuestionnaire;
        progress.step = 1;

        let mut data = questionnaire.raw.clone();

        // Remove ID and timestamps
        if let Value::Object(ref mut map) = data {
            map.remove("nrq_questionnaireid");
            map.remove("createdon");
            map.remove("modifiedon");
            map.remove("_createdby_value");
            map.remove("_modifiedby_value");
            map.remove("versionnumber");
        }

        // Update name and code
        data["nrq_name"] = json!(copy_name);
        data["nrq_copypostfix"] = json!(copy_code);

        let operations = Operations::new().create("nrq_questionnaires", data);
        let results = self.execute_step(operations, "nrq_questionnaires").await?;

        // Extract new questionnaire ID
        let new_id = self.extract_entity_id(&results[0])?;

        // Update progress
        progress.questionnaire = (1, 1);
        progress.total_created += 1;
        self.send_progress_update(progress, progress_sender).await;

        Ok(new_id)
    }

    // ============================================================================
    // Step 2: Create Pages
    // ============================================================================

    async fn create_pages(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingPages;
        progress.step = 2;

        let new_questionnaire_id = self.id_map.get(&questionnaire.id)
            .ok_or_else(|| "Questionnaire ID not found in map".to_string())?;

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();

        for page in &questionnaire.pages {
            let mut data = self.remap_lookup_fields(&page.raw, &shared_entities);

            // Remap questionnaire reference
            data["nrq_questionnaireid@odata.bind"] = json!(format!("/nrq_questionnaires({})", new_questionnaire_id));

            // Remove ID and timestamps
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

        if questionnaire.pages.is_empty() {
            return Ok(());
        }

        let results = self.execute_step(operations, "nrq_questionnairepages").await?;

        // Store page ID mappings
        for (page, result) in questionnaire.pages.iter().zip(results.iter()) {
            let new_id = self.extract_entity_id(result)?;
            self.id_map.insert(page.id.clone(), new_id);
        }

        // Update progress
        progress.pages = (questionnaire.pages.len(), questionnaire.pages.len());
        progress.total_created += questionnaire.pages.len();
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 3: Create Page Lines
    // ============================================================================

    async fn create_page_lines(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingPageLines;
        progress.step = 3;

        if questionnaire.page_lines.is_empty() {
            return Ok(());
        }

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();

        for page_line in &questionnaire.page_lines {
            let mut data = self.remap_lookup_fields(page_line, &shared_entities);

            // Remove ID and timestamps
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

        let results = self.execute_step(operations, "nrq_questionnairepagelines").await?;

        // Update progress
        progress.page_lines = (questionnaire.page_lines.len(), questionnaire.page_lines.len());
        progress.total_created += questionnaire.page_lines.len();
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 4: Create Groups
    // ============================================================================

    async fn create_groups(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingGroups;
        progress.step = 4;

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();
        let mut all_groups = Vec::new();

        for page in &questionnaire.pages {
            for group in &page.groups {
                let mut data = self.remap_lookup_fields(&group.raw, &shared_entities);

                // Remove ID and timestamps
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
            return Ok(());
        }

        let results = self.execute_step(operations, "nrq_questiongroups").await?;

        // Store group ID mappings
        for (group, result) in all_groups.iter().zip(results.iter()) {
            let new_id = self.extract_entity_id(result)?;
            self.id_map.insert(group.id.clone(), new_id);
        }

        // Update progress
        let groups_count = all_groups.len();
        progress.groups = (groups_count, groups_count);
        progress.total_created += groups_count;
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 5: Create Group Lines
    // ============================================================================

    async fn create_group_lines(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingGroupLines;
        progress.step = 5;

        if questionnaire.group_lines.is_empty() {
            return Ok(());
        }

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();

        for group_line in &questionnaire.group_lines {
            let mut data = self.remap_lookup_fields(group_line, &shared_entities);

            // Remove ID and timestamps
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

        let results = self.execute_step(operations, "nrq_questiongrouplines").await?;

        // Update progress
        progress.group_lines = (questionnaire.group_lines.len(), questionnaire.group_lines.len());
        progress.total_created += questionnaire.group_lines.len();
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 6: Create Questions
    // ============================================================================

    async fn create_questions(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingQuestions;
        progress.step = 6;

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();
        let mut all_questions = Vec::new();

        for page in &questionnaire.pages {
            for group in &page.groups {
                for question in &group.questions {
                    let mut data = self.remap_lookup_fields(&question.raw, &shared_entities);

                    // Remove ID and timestamps
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
            return Ok(());
        }

        let results = self.execute_step(operations, "nrq_questions").await?;

        // Store question ID mappings
        for (question, result) in all_questions.iter().zip(results.iter()) {
            let new_id = self.extract_entity_id(result)?;
            self.id_map.insert(question.id.clone(), new_id);
        }

        // Update progress
        let questions_count = all_questions.len();
        progress.questions = (questions_count, questions_count);
        progress.total_created += questions_count;
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 7: Create Template Lines
    // ============================================================================

    async fn create_template_lines(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingTemplateLines;
        progress.step = 7;

        if questionnaire.template_lines.is_empty() {
            return Ok(());
        }

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();

        for template_line in &questionnaire.template_lines {
            let mut data = self.remap_lookup_fields(&template_line.raw, &shared_entities);

            // Remove ID and timestamps
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

        let results = self.execute_step(operations, "nrq_questiontemplatetogrouplines").await?;

        // Update progress
        progress.template_lines = (questionnaire.template_lines.len(), questionnaire.template_lines.len());
        progress.total_created += questionnaire.template_lines.len();
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 8: Create Conditions
    // ============================================================================

    async fn create_conditions(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingConditions;
        progress.step = 8;

        if questionnaire.conditions.is_empty() {
            return Ok(());
        }

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();

        for condition in &questionnaire.conditions {
            let mut data = self.remap_lookup_fields(&condition.raw, &shared_entities);

            // CRITICAL: Remap condition JSON with embedded question IDs
            if let Some(condition_json_str) = condition.raw.get("nrq_conditionjson").and_then(|v| v.as_str()) {
                match self.remap_condition_json(condition_json_str) {
                    Ok(remapped_json) => {
                        data["nrq_conditionjson"] = json!(remapped_json);
                    }
                    Err(e) => {
                        log::warn!("Failed to remap condition JSON: {}", e);
                        // Keep original if remapping fails
                    }
                }
            }

            // Remove ID and timestamps
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

        let results = self.execute_step(operations, "nrq_questionconditions").await?;

        // Store condition ID mappings
        for (condition, result) in questionnaire.conditions.iter().zip(results.iter()) {
            let new_id = self.extract_entity_id(result)?;
            self.id_map.insert(condition.id.clone(), new_id);
        }

        // Update progress
        progress.conditions = (questionnaire.conditions.len(), questionnaire.conditions.len());
        progress.total_created += questionnaire.conditions.len();
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 9: Create Condition Actions
    // ============================================================================

    async fn create_condition_actions(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingConditionActions;
        progress.step = 9;

        let shared_entities = self.get_shared_entities();
        let mut operations = Operations::new();
        let mut actions_count = 0;

        for condition in &questionnaire.conditions {
            for action in &condition.actions {
                let mut data = self.remap_lookup_fields(&action.raw, &shared_entities);

                // Remove ID and timestamps
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
            return Ok(());
        }

        let results = self.execute_step(operations, "nrq_questionconditionactions").await?;

        // Update progress
        progress.condition_actions = (actions_count, actions_count);
        progress.total_created += actions_count;
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Step 10: Create Classifications (N:N via AssociateRef)
    // ============================================================================

    async fn create_classifications(
        &mut self,
        questionnaire: &Questionnaire,
        progress: &mut CopyProgress,
        progress_sender: &Sender<Msg>,
    ) -> Result<(), String> {
        progress.phase = CopyPhase::CreatingClassifications;
        progress.step = 10;

        let new_questionnaire_id = self.id_map.get(&questionnaire.id)
            .ok_or_else(|| "Questionnaire ID not found in map".to_string())?;

        let mut operations = Operations::new();
        let mut classifications_count = 0;

        // Category associations
        for category_ref in &questionnaire.classifications.categories {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Category_nrq_Category".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_categories({})", category_ref.id),
            });
            classifications_count += 1;
        }

        // Domain associations
        for domain_ref in &questionnaire.classifications.domains {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Domain_nrq_Domain".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_domains({})", domain_ref.id),
            });
            classifications_count += 1;
        }

        // Fund associations
        for fund_ref in &questionnaire.classifications.funds {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Fund_nrq_Fund".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_funds({})", fund_ref.id),
            });
            classifications_count += 1;
        }

        // Support associations
        for support_ref in &questionnaire.classifications.supports {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Support_nrq_Support".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_supports({})", support_ref.id),
            });
            classifications_count += 1;
        }

        // Type associations
        for type_ref in &questionnaire.classifications.types {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Type_nrq_Type".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_types({})", type_ref.id),
            });
            classifications_count += 1;
        }

        // Subcategory associations
        for subcategory_ref in &questionnaire.classifications.subcategories {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_Subcategory_nrq_Subcategory".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_subcategories({})", subcategory_ref.id),
            });
            classifications_count += 1;
        }

        // Flemish share associations
        for flemish_share_ref in &questionnaire.classifications.flemish_shares {
            operations = operations.add(Operation::AssociateRef {
                entity: "nrq_questionnaires".to_string(),
                entity_ref: new_questionnaire_id.clone(),
                navigation_property: "nrq_questionnaire_nrq_FlemishShare_nrq_FlemishShare".to_string(),
                target_ref: format!("/api/data/v9.2/nrq_flemishshares({})", flemish_share_ref.id),
            });
            classifications_count += 1;
        }

        if classifications_count == 0 {
            return Ok(());
        }

        let results = self.execute_step(operations, "associations").await?;

        // Update progress
        progress.classifications = (classifications_count, classifications_count);
        progress.total_created += classifications_count;
        self.send_progress_update(progress, progress_sender).await;

        Ok(())
    }

    // ============================================================================
    // Helper Functions
    // ============================================================================

    /// Remap lookup fields from _fieldname_value to fieldname@odata.bind
    fn remap_lookup_fields(
        &self,
        raw_data: &Value,
        shared_entities: &HashSet<&str>,
    ) -> Value {
        let mut data = raw_data.clone();

        if let Value::Object(ref mut map) = data {
            let mut remapped_fields = Vec::new();

            // Find all _*_value fields
            for (key, value) in map.iter() {
                if key.starts_with('_') && key.ends_with("_value") {
                    if let Some(guid) = value.as_str() {
                        // Extract field name: _nrq_questionnaireid_value -> nrq_questionnaireid
                        let field_name = key.trim_start_matches('_').trim_end_matches("_value");

                        // Determine if this is a shared entity or needs remapping
                        let is_shared = shared_entities.iter().any(|&entity_field| field_name.contains(entity_field));

                        let final_guid = if is_shared {
                            // Shared entity - use original ID
                            guid.to_string()
                        } else {
                            // Check if we have a remapped ID
                            self.id_map.get(guid).cloned().unwrap_or_else(|| guid.to_string())
                        };

                        // Determine entity set name from field name
                        let entity_set = self.infer_entity_set_from_field(field_name);

                        remapped_fields.push((
                            key.clone(),
                            format!("{}@odata.bind", field_name),
                            format!("/{}({})", entity_set, final_guid),
                        ));
                    }
                }
            }

            // Apply remappings
            for (old_key, new_key, new_value) in remapped_fields {
                map.remove(&old_key);
                map.insert(new_key, json!(new_value));
            }
        }

        data
    }

    /// Remap question IDs in condition JSON
    fn remap_condition_json(&self, condition_json_str: &str) -> Result<String, String> {
        let mut json: Value = serde_json::from_str(condition_json_str)
            .map_err(|e| format!("Failed to parse condition JSON: {}", e))?;

        // Remap root questionId
        if let Some(question_id) = json.get("questionId").and_then(|v| v.as_str()) {
            if let Some(new_id) = self.id_map.get(question_id) {
                json["questionId"] = json!(new_id);
            }
        }

        // Remap questions[] array
        if let Some(questions) = json.get_mut("questions").and_then(|v| v.as_array_mut()) {
            for q in questions {
                if let Some(question_id) = q.get("questionId").and_then(|v| v.as_str()) {
                    if let Some(new_id) = self.id_map.get(question_id) {
                        q["questionId"] = json!(new_id);
                    }
                }
            }
        }

        serde_json::to_string(&json)
            .map_err(|e| format!("Failed to serialize condition JSON: {}", e))
    }

    /// Get set of shared entity field patterns (templates, tags, classifications)
    fn get_shared_entities(&self) -> HashSet<&str> {
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

    /// Infer entity set name from field name
    fn infer_entity_set_from_field(&self, field_name: &str) -> String {
        // Map common field patterns to entity sets
        if field_name.contains("questionnaireid") {
            "nrq_questionnaires".to_string()
        } else if field_name.contains("questionnairepageid") {
            "nrq_questionnairepages".to_string()
        } else if field_name.contains("questiongroupid") {
            "nrq_questiongroups".to_string()
        } else if field_name.contains("questiontemplateid") {
            "nrq_questiontemplates".to_string()
        } else if field_name.contains("questiontagid") {
            "nrq_questiontags".to_string()
        } else if field_name.contains("questionconditionid") {
            "nrq_questionconditions".to_string()
        } else if field_name.contains("questionid") {
            "nrq_questions".to_string()
        } else {
            // Fallback: pluralize the field name without 'id'
            let entity = field_name.trim_end_matches("id");
            pluralize_entity_name(entity)
        }
    }

    /// Execute a batch of operations and track created IDs
    async fn execute_step(
        &mut self,
        operations: Operations,
        entity_set: &str,
    ) -> Result<Vec<crate::api::operations::OperationResult>, String> {
        let results = operations.execute(&self.client, &self.resilience).await
            .map_err(|e| format!("Batch execution failed: {}", e))?;

        // Track created IDs for rollback
        for result in &results {
            if result.success {
                if let Some(entity_id) = self.extract_entity_id_from_result(result) {
                    self.created_ids.push((entity_set.to_string(), entity_id));
                }
            }
        }

        // Check for any failures
        let failures: Vec<_> = results.iter().filter(|r| !r.success).collect();
        if !failures.is_empty() {
            let errors: Vec<String> = failures.iter()
                .filter_map(|r| r.error.clone())
                .collect();
            return Err(format!("Batch had {} failures: {}", failures.len(), errors.join("; ")));
        }

        Ok(results)
    }

    /// Extract entity ID from operation result
    fn extract_entity_id(&self, result: &crate::api::operations::OperationResult) -> Result<String, String> {
        self.extract_entity_id_from_result(result)
            .ok_or_else(|| "Failed to extract entity ID from result".to_string())
    }

    /// Extract entity ID from result (if present)
    fn extract_entity_id_from_result(&self, result: &crate::api::operations::OperationResult) -> Option<String> {
        // Try to get from OData-EntityId header first
        if let Some(entity_id_url) = result.headers.get("OData-EntityId") {
            // Extract GUID from URL: https://.../ entities(guid)
            if let Some(start) = entity_id_url.rfind('(') {
                if let Some(end) = entity_id_url.rfind(')') {
                    return Some(entity_id_url[start + 1..end].to_string());
                }
            }
        }

        // Try to get from response data
        if let Some(data) = &result.data {
            // Look for *id field in response
            if let Value::Object(map) = data {
                for (key, value) in map {
                    if key.ends_with("id") {
                        if let Some(id) = value.as_str() {
                            return Some(id.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Send progress update to UI
    async fn send_progress_update(&self, progress: &CopyProgress, sender: &Sender<Msg>) {
        let _ = sender.send(Msg::CopyProgressUpdate(progress.clone())).await;
    }

    /// Build error with rollback attempt
    async fn build_error(
        &mut self,
        error: String,
        phase: CopyPhase,
        step: usize,
        progress: &CopyProgress,
    ) -> CopyError {
        log::error!("Copy failed at step {}: {}", step, error);

        let rollback_complete = self.rollback().await;

        CopyError {
            error_message: error,
            phase,
            step,
            partial_counts: self.build_entity_counts(progress),
            rollback_complete,
        }
    }

    /// Build entity counts map from progress
    fn build_entity_counts(&self, progress: &CopyProgress) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        counts.insert("questionnaire".to_string(), progress.questionnaire.0);
        counts.insert("pages".to_string(), progress.pages.0);
        counts.insert("page_lines".to_string(), progress.page_lines.0);
        counts.insert("groups".to_string(), progress.groups.0);
        counts.insert("group_lines".to_string(), progress.group_lines.0);
        counts.insert("questions".to_string(), progress.questions.0);
        counts.insert("template_lines".to_string(), progress.template_lines.0);
        counts.insert("conditions".to_string(), progress.conditions.0);
        counts.insert("condition_actions".to_string(), progress.condition_actions.0);
        counts.insert("classifications".to_string(), progress.classifications.0);
        counts
    }

    /// Attempt to rollback (delete all created entities in reverse order)
    async fn rollback(&self) -> bool {
        log::info!("Attempting rollback of {} created entities", self.created_ids.len());

        let mut operations = Operations::new();

        // Delete in reverse order
        for (entity_set, entity_id) in self.created_ids.iter().rev() {
            operations = operations.delete(entity_set, entity_id);
        }

        match operations.execute(&self.client, &self.resilience).await {
            Ok(results) => {
                let failures = results.iter().filter(|r| !r.success).count();
                if failures == 0 {
                    log::info!("Rollback completed successfully");
                    true
                } else {
                    log::warn!("Rollback partially failed: {} of {} deletions failed", failures, results.len());
                    false
                }
            }
            Err(e) => {
                log::error!("Rollback failed: {}", e);
                false
            }
        }
    }
}
