use anyhow::Result;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use super::ui::components::fetch_progress::{FetchProgress, FetchStatus};
use crate::config::{AuthConfig, SavedComparison, SavedMigration};
use crate::dynamics::client::DynamicsClient;
use crate::dynamics::metadata::{FieldInfo, FormInfo, ViewInfo, parse_entity_fields};
use super::ui::screens::comparison::data_models::ExamplePair;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ComparisonData {
    pub source_fields: Vec<FieldInfo>,
    pub target_fields: Vec<FieldInfo>,
    pub source_views: Vec<ViewInfo>,
    pub target_views: Vec<ViewInfo>,
    pub source_forms: Vec<FormInfo>,
    pub target_forms: Vec<FormInfo>,
    pub source_entity: String,
    pub target_entity: String,
    pub source_env: String,
    pub target_env: String,
}

impl ComparisonData {
    pub fn new(
        source_fields: Vec<FieldInfo>,
        target_fields: Vec<FieldInfo>,
        source_views: Vec<ViewInfo>,
        target_views: Vec<ViewInfo>,
        source_forms: Vec<FormInfo>,
        target_forms: Vec<FormInfo>,
        source_entity: String,
        target_entity: String,
        source_env: String,
        target_env: String,
    ) -> Self {
        Self {
            source_fields,
            target_fields,
            source_views,
            target_views,
            source_forms,
            target_forms,
            source_entity,
            target_entity,
            source_env,
            target_env,
        }
    }
}

pub async fn fetch_comparison_data(
    source_auth: AuthConfig,
    target_auth: AuthConfig,
    migration: SavedMigration,
    comparison: SavedComparison,
    examples: Vec<ExamplePair>,
    progress: Arc<Mutex<FetchProgress>>,
) -> Result<(ComparisonData, HashMap<String, Value>)> {
    let source_entity = comparison.source_entity.clone();
    let target_entity = comparison.target_entity.clone();
    let source_env = migration.source_env.clone();
    let target_env = migration.target_env.clone();

    // Launch all 6 fetch operations in parallel
    let source_fields_task = spawn_fetch_fields(
        source_auth.clone(),
        source_entity.clone(),
        progress.clone(),
        |p| &mut p.source_fields,
    );

    let target_fields_task = spawn_fetch_fields(
        target_auth.clone(),
        target_entity.clone(),
        progress.clone(),
        |p| &mut p.target_fields,
    );

    let source_views_task = spawn_fetch_views(
        source_auth.clone(),
        source_entity.clone(),
        progress.clone(),
        |p| &mut p.source_views,
    );

    let target_views_task = spawn_fetch_views(
        target_auth.clone(),
        target_entity.clone(),
        progress.clone(),
        |p| &mut p.target_views,
    );

    let source_forms_task = spawn_fetch_forms(
        source_auth.clone(),
        source_entity.clone(),
        progress.clone(),
        |p| &mut p.source_forms,
    );

    let target_forms_task = spawn_fetch_forms(
        target_auth.clone(),
        target_entity.clone(),
        progress.clone(),
        |p| &mut p.target_forms,
    );

    // Spawn examples task if there are examples to fetch
    let examples_task = if !examples.is_empty() {
        log::debug!("Starting example fetch task for {} examples", examples.len());
        for example in &examples {
            log::debug!("  Example: {} -> {}", example.source_uuid, example.target_uuid);
        }

        // Set examples to in progress
        {
            let mut progress_lock = progress.lock().unwrap();
            progress_lock.examples = super::ui::components::fetch_progress::FetchStatus::InProgress;
            log::debug!("Set examples progress to InProgress");
        }

        let progress_clone = progress.clone();
        let examples_clone = examples.clone();
        let source_auth_clone = source_auth.clone();
        let target_auth_clone = target_auth.clone();
        let source_entity_clone = source_entity.clone();
        let target_entity_clone = target_entity.clone();

        Some(tokio::spawn(async move {
            log::debug!("Calling fetch_example_data_best_effort with source_entity={}, target_entity={}", source_entity_clone, target_entity_clone);
            let result = fetch_example_data_best_effort(
                examples_clone,
                source_auth_clone,
                target_auth_clone,
                source_entity_clone,
                target_entity_clone,
            ).await;

            log::debug!("fetch_example_data_best_effort returned {} items", result.len());
            for (uuid, _) in &result {
                log::debug!("  Fetched data for UUID: {}", uuid);
            }

            // Update progress based on result
            {
                let mut progress_lock = progress_clone.lock().unwrap();
                if result.is_empty() && !examples.is_empty() {
                    log::debug!("Setting examples progress to Failed - no data retrieved");
                    progress_lock.examples = super::ui::components::fetch_progress::FetchStatus::Failed("No example data retrieved".to_string());
                } else {
                    log::debug!("Setting examples progress to Completed");
                    progress_lock.examples = super::ui::components::fetch_progress::FetchStatus::Completed;
                }
            }

            result
        }))
    } else {
        log::debug!("No examples to fetch, marking as completed");
        // Mark examples as completed if there are no examples to fetch
        {
            let mut progress_lock = progress.lock().unwrap();
            progress_lock.examples = super::ui::components::fetch_progress::FetchStatus::Completed;
        }
        None
    };

    // Wait for all tasks to complete
    let results = tokio::try_join!(
        source_fields_task,
        target_fields_task,
        source_views_task,
        target_views_task,
        source_forms_task,
        target_forms_task
    )?;

    // Wait for examples task if it exists
    let example_data = if let Some(examples_task) = examples_task {
        log::debug!("Awaiting examples task completion...");
        let data = examples_task.await?;
        log::debug!("Examples task completed with {} items", data.len());
        data
    } else {
        log::debug!("No examples task to wait for");
        HashMap::new()
    };

    // Check if any task failed
    let progress_lock = progress.lock().unwrap();
    if progress_lock.has_any_failures() {
        return Err(anyhow::anyhow!("One or more fetch operations failed"));
    }
    drop(progress_lock);

    // All successful - extract results
    let source_fields = results.0?;
    let target_fields = results.1?;
    let source_views = results.2?;
    let target_views = results.3?;
    let source_forms = results.4?;
    let target_forms = results.5?;

    Ok((
        ComparisonData::new(
            source_fields,
            target_fields,
            source_views,
            target_views,
            source_forms,
            target_forms,
            source_entity,
            target_entity,
            source_env,
            target_env,
        ),
        example_data,
    ))
}

fn spawn_fetch_fields<F>(
    auth: AuthConfig,
    entity_name: String,
    progress: Arc<Mutex<FetchProgress>>,
    status_setter: F,
) -> JoinHandle<Result<Vec<FieldInfo>>>
where
    F: Fn(&mut FetchProgress) -> &mut FetchStatus + Send + 'static,
{
    tokio::spawn(async move {
        // Set status to in progress
        {
            let mut p = progress.lock().unwrap();
            *status_setter(&mut p) = FetchStatus::InProgress;
        }

        let mut client = DynamicsClient::new(auth);

        match fetch_entity_fields(&mut client, &entity_name).await {
            Ok(fields) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Completed;
                Ok(fields)
            }
            Err(e) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Failed(e.to_string());
                Err(e)
            }
        }
    })
}

fn spawn_fetch_views<F>(
    auth: AuthConfig,
    entity_name: String,
    progress: Arc<Mutex<FetchProgress>>,
    status_setter: F,
) -> JoinHandle<Result<Vec<ViewInfo>>>
where
    F: Fn(&mut FetchProgress) -> &mut FetchStatus + Send + 'static,
{
    tokio::spawn(async move {
        // Set status to in progress
        {
            let mut p = progress.lock().unwrap();
            *status_setter(&mut p) = FetchStatus::InProgress;
        }

        let mut client = DynamicsClient::new(auth);

        match client.fetch_views(Some(&entity_name)).await {
            Ok(views) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Completed;
                Ok(views)
            }
            Err(e) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Failed(e.to_string());
                Err(e)
            }
        }
    })
}

fn spawn_fetch_forms<F>(
    auth: AuthConfig,
    entity_name: String,
    progress: Arc<Mutex<FetchProgress>>,
    status_setter: F,
) -> JoinHandle<Result<Vec<FormInfo>>>
where
    F: Fn(&mut FetchProgress) -> &mut FetchStatus + Send + 'static,
{
    tokio::spawn(async move {
        // Set status to in progress
        {
            let mut p = progress.lock().unwrap();
            *status_setter(&mut p) = FetchStatus::InProgress;
        }

        let mut client = DynamicsClient::new(auth);

        match client.fetch_forms(Some(&entity_name)).await {
            Ok(forms) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Completed;
                Ok(forms)
            }
            Err(e) => {
                let mut p = progress.lock().unwrap();
                *status_setter(&mut p) = FetchStatus::Failed(e.to_string());
                Err(e)
            }
        }
    })
}

async fn fetch_entity_fields(
    client: &mut DynamicsClient,
    entity_name: &str,
) -> Result<Vec<FieldInfo>> {
    let metadata_xml = client.fetch_metadata().await?;
    parse_entity_fields(&metadata_xml, entity_name)
}

/// Fetch example record data for a list of examples (non-blocking, best effort)
/// This function never fails the overall comparison data fetch - it returns
/// what it can get and logs errors for individual failures.
pub async fn fetch_example_data_best_effort(
    examples: Vec<ExamplePair>,
    source_auth: AuthConfig,
    target_auth: AuthConfig,
    source_entity: String,
    target_entity: String,
) -> HashMap<String, Value> {
    log::debug!("fetch_example_data_best_effort called with {} examples", examples.len());
    log::debug!("  source_entity: {}", source_entity);
    log::debug!("  target_entity: {}", target_entity);
    log::debug!("  source_auth host: {}", source_auth.host);
    log::debug!("  target_auth host: {}", target_auth.host);

    if examples.is_empty() {
        log::debug!("No examples provided, returning empty map");
        return HashMap::new();
    }

    log::info!("Fetching example data for {} example pairs", examples.len());

    let mut tasks = Vec::new();

    // Create tasks for each UUID (both source and target)
    for example in &examples {
        log::debug!("Creating fetch tasks for example pair:");
        log::debug!("  source_uuid: {} (entity: {})", example.source_uuid, source_entity);
        log::debug!("  target_uuid: {} (entity: {})", example.target_uuid, target_entity);

        // Source record fetch
        let source_task = spawn_fetch_example_record(
            source_auth.clone(),
            source_entity.to_string(),
            example.source_uuid.clone(),
        );
        tasks.push((format!("source:{}", example.source_uuid), source_task));

        // Target record fetch
        let target_task = spawn_fetch_example_record(
            target_auth.clone(),
            target_entity.to_string(),
            example.target_uuid.clone(),
        );
        tasks.push((format!("target:{}", example.target_uuid), target_task));
    }

    // Collect results (ignore failures)
    let mut example_data = HashMap::new();
    log::debug!("Processing {} fetch tasks...", tasks.len());

    for (uuid, task) in tasks {
        log::debug!("Awaiting task for UUID: {}", uuid);
        match task.await {
            Ok(Ok(record_data)) => {
                log::info!("Successfully fetched example data for UUID: {}", uuid);
                log::debug!("  Record data keys: {:?}", record_data.as_object().map(|obj| obj.keys().collect::<Vec<_>>()));
                example_data.insert(uuid, record_data);
            }
            Ok(Err(e)) => {
                log::warn!("Failed to fetch example record {}: {}", uuid, e);
            }
            Err(e) => {
                log::error!("Task failed for example record {}: {}", uuid, e);
            }
        }
    }

    log::info!(
        "Fetched example data for {}/{} records",
        example_data.len(),
        examples.len() * 2
    );

    for (uuid, data) in &example_data {
        log::debug!("Final result - UUID {}: {} fields", uuid, data.as_object().map_or(0, |obj| obj.len()));
    }

    example_data
}

/// Spawn a task to fetch a single example record
fn spawn_fetch_example_record(
    auth: AuthConfig,
    entity_name: String,
    record_id: String,
) -> JoinHandle<Result<Value>> {
    tokio::spawn(async move {
        log::debug!("spawn_fetch_example_record: Starting fetch for record {} from entity {}", record_id, entity_name);
        log::debug!("  Auth host: {}", auth.host);
        log::debug!("  Auth username: {}", auth.username);

        let mut client = DynamicsClient::new(auth);

        match client.fetch_record_by_id_silent(&entity_name, &record_id).await {
            Ok(record_data) => {
                log::info!("Fetched record {} from entity {} successfully", record_id, entity_name);
                log::debug!("  Record has {} fields", record_data.as_object().map_or(0, |obj| obj.len()));
                Ok(record_data)
            }
            Err(e) => {
                log::error!("Failed to fetch record {} from entity {}: {}", record_id, entity_name, e);
                Err(e)
            }
        }
    })
}
