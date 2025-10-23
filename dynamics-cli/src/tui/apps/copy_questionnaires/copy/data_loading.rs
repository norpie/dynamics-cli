use super::models::QuestionnaireSnapshot;
use serde_json::Value;
use crate::api::query::{Query, Filter, FilterValue};

/// Load complete questionnaire snapshot with all related entities
/// This performs multiple sequential queries to fetch all data
pub async fn load_full_snapshot(questionnaire_id: &str) -> Result<QuestionnaireSnapshot, String> {
    log::info!("Loading full snapshot for questionnaire: {}", questionnaire_id);

    // Get the client
    let manager = crate::client_manager();
    let env_name = manager.get_current_environment_name().await
        .ok()
        .flatten()
        .ok_or_else(|| "No environment selected".to_string())?;

    let client = manager.get_client(&env_name).await
        .map_err(|e| e.to_string())?;

    // Phase 1 - Snapshot (Read Everything) - Load in dependency order

    // 1. Load questionnaire entity by ID
    log::debug!("Loading questionnaire: {}", questionnaire_id);
    let questionnaire = load_questionnaire(&client, questionnaire_id).await?;

    // 2. Load page lines (junction records for page ordering)
    log::debug!("Loading page lines");
    let page_lines = load_page_lines(&client, questionnaire_id).await?;
    let page_ids = extract_ids(&page_lines, "nrq_questionnairepageid");

    // 3. Load pages
    log::debug!("Loading pages ({})", page_ids.len());
    let pages = if !page_ids.is_empty() {
        load_pages(&client, &page_ids).await?
    } else {
        vec![]
    };

    // 4. Load group lines (junction records for group ordering)
    log::debug!("Loading group lines");
    let group_lines = if !page_ids.is_empty() {
        load_group_lines(&client, &page_ids).await?
    } else {
        vec![]
    };
    let group_ids = extract_ids(&group_lines, "nrq_questiongroupid");

    // 5. Load groups
    log::debug!("Loading groups ({})", group_ids.len());
    let groups = if !group_ids.is_empty() {
        load_groups(&client, &group_ids).await?
    } else {
        vec![]
    };

    // 6. Load questions
    log::debug!("Loading questions");
    let questions = load_questions(&client, questionnaire_id).await?;
    let question_ids = extract_ids(&questions, "nrq_questionid");

    // 7. Load template lines
    log::debug!("Loading template lines");
    let template_lines = if !group_ids.is_empty() {
        load_template_lines(&client, &group_ids).await?
    } else {
        vec![]
    };

    // 8. Load conditions
    log::debug!("Loading conditions");
    let conditions = if !question_ids.is_empty() {
        load_conditions(&client, &question_ids).await?
    } else {
        vec![]
    };
    let condition_ids = extract_ids(&conditions, "nrq_questionconditionid");

    // 9. Load condition actions
    log::debug!("Loading condition actions");
    let condition_actions = if !condition_ids.is_empty() {
        load_condition_actions(&client, &condition_ids).await?
    } else {
        vec![]
    };

    // 10. Load N:N relationships (7 junction tables)
    log::debug!("Loading N:N relationships");
    let categories = load_n_n_relationship(&client, "nrq_questionnaire_nrq_category", questionnaire_id, "nrq_categoryid").await?;
    let domains = load_n_n_relationship(&client, "nrq_questionnaire_nrq_domain", questionnaire_id, "nrq_domainid").await?;
    let funds = load_n_n_relationship(&client, "nrq_questionnaire_nrq_fund", questionnaire_id, "nrq_fundid").await?;
    let supports = load_n_n_relationship(&client, "nrq_questionnaire_nrq_support", questionnaire_id, "nrq_supportid").await?;
    let types = load_n_n_relationship(&client, "nrq_questionnaire_nrq_type", questionnaire_id, "nrq_typeid").await?;
    let subcategories = load_n_n_relationship(&client, "nrq_questionnaire_nrq_subcategory", questionnaire_id, "nrq_subcategoryid").await?;
    let flemish_shares = load_n_n_relationship(&client, "nrq_questionnaire_nrq_flemishshare", questionnaire_id, "nrq_flemishshareid").await?;

    log::info!("Successfully loaded complete snapshot with {} total entities",
        1 + pages.len() + page_lines.len() + groups.len() + group_lines.len() +
        questions.len() + template_lines.len() + conditions.len() + condition_actions.len() +
        categories.len() + domains.len() + funds.len() + supports.len() + types.len() +
        subcategories.len() + flemish_shares.len()
    );

    Ok(QuestionnaireSnapshot {
        questionnaire,
        pages,
        page_lines,
        groups,
        group_lines,
        questions,
        template_lines,
        conditions,
        condition_actions,
        categories,
        domains,
        funds,
        supports,
        types,
        subcategories,
        flemish_shares,
    })
}

/// Load a single questionnaire entity by ID
async fn load_questionnaire(client: &crate::api::DynamicsClient, id: &str) -> Result<Value, String> {
    client.fetch_record_by_id("nrq_questionnaire", id)
        .await
        .map_err(|e| format!("Failed to load questionnaire: {}", e))
}

/// Load page lines (ordering junctions) for a questionnaire
async fn load_page_lines(client: &crate::api::DynamicsClient, questionnaire_id: &str) -> Result<Vec<Value>, String> {
    let mut query = Query::new("nrq_questionnairepagelines");
    query.filter = Some(Filter::eq("_nrq_questionnaireid_value", FilterValue::Guid(questionnaire_id.to_string())));

    log::debug!("Loading page_lines with questionnaire_id: {}", questionnaire_id);
    if let Some(ref filter) = query.filter {
        log::debug!("Filter OData string: {}", filter.to_odata_string());
    }

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load page lines: {}", e))?;

    log::debug!("Page lines result count: {}", result.data.as_ref().map(|d| d.value.len()).unwrap_or(0));

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load pages by IDs
async fn load_pages(client: &crate::api::DynamicsClient, page_ids: &[String]) -> Result<Vec<Value>, String> {
    if page_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questionnairepages");
    query.filter = Some(build_or_filter("nrq_questionnairepageid", page_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load pages: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load group lines (ordering junctions) for pages
async fn load_group_lines(client: &crate::api::DynamicsClient, page_ids: &[String]) -> Result<Vec<Value>, String> {
    if page_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questiongrouplines");
    query.filter = Some(build_or_filter("_nrq_questionnairepageid_value", page_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load group lines: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load groups by IDs
async fn load_groups(client: &crate::api::DynamicsClient, group_ids: &[String]) -> Result<Vec<Value>, String> {
    if group_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questiongroups");
    query.filter = Some(build_or_filter("nrq_questiongroupid", group_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load groups: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load questions related to a questionnaire
async fn load_questions(client: &crate::api::DynamicsClient, questionnaire_id: &str) -> Result<Vec<Value>, String> {
    let mut query = Query::new("nrq_questions");
    query.filter = Some(Filter::eq("_nrq_questionnaireid_value", FilterValue::Guid(questionnaire_id.to_string())));

    log::debug!("Loading questions with questionnaire_id: {}", questionnaire_id);
    if let Some(ref filter) = query.filter {
        log::debug!("Filter OData string: {}", filter.to_odata_string());
    }

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load questions: {}", e))?;

    log::debug!("Questions result count: {}", result.data.as_ref().map(|d| d.value.len()).unwrap_or(0));

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load template lines related to groups
async fn load_template_lines(client: &crate::api::DynamicsClient, group_ids: &[String]) -> Result<Vec<Value>, String> {
    if group_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questiontemplatelines");
    query.filter = Some(build_or_filter("_nrq_questiongroupid_value", group_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load template lines: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load conditions related to questions
async fn load_conditions(client: &crate::api::DynamicsClient, question_ids: &[String]) -> Result<Vec<Value>, String> {
    if question_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questionconditions");
    query.filter = Some(build_or_filter("_nrq_questionid_value", question_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load conditions: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load condition actions related to conditions
async fn load_condition_actions(client: &crate::api::DynamicsClient, condition_ids: &[String]) -> Result<Vec<Value>, String> {
    if condition_ids.is_empty() {
        return Ok(vec![]);
    }

    let mut query = Query::new("nrq_questionconditionactions");
    query.filter = Some(build_or_filter("_nrq_questionconditionid_value", condition_ids));

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load condition actions: {}", e))?;

    Ok(result.data.map(|d| d.value).unwrap_or_default())
}

/// Load N:N relationship IDs from junction table
/// Returns the IDs of the related entities (e.g., category IDs)
async fn load_n_n_relationship(
    client: &crate::api::DynamicsClient,
    junction_table: &str,
    questionnaire_id: &str,
    related_id_field: &str,
) -> Result<Vec<String>, String> {
    let mut query = Query::new(junction_table);
    query.filter = Some(Filter::eq("nrq_questionnaireid", FilterValue::Guid(questionnaire_id.to_string())));
    query.select = Some(vec![related_id_field.to_string()]);

    let result = client.execute_query(&query)
        .await
        .map_err(|e| format!("Failed to load {} relationship: {}", junction_table, e))?;

    let records = result.data.map(|d| d.value).unwrap_or_default();

    // Extract the related entity IDs from the junction records
    Ok(records.iter()
        .filter_map(|record| {
            record.get(related_id_field)
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect())
}

/// Extract IDs from a Vec<Value> given a field name
fn extract_ids(records: &[Value], id_field: &str) -> Vec<String> {
    records.iter()
        .filter_map(|record| {
            record.get(id_field)
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .collect()
}

/// Build an OR filter for matching multiple IDs
/// Example: field eq id1 or field eq id2 or field eq id3
fn build_or_filter(field: &str, ids: &[String]) -> Filter {
    let filters: Vec<Filter> = ids.iter()
        .map(|id| Filter::eq(field, FilterValue::Guid(id.clone())))
        .collect();

    Filter::or(filters)
}
