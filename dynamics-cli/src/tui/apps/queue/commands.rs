//! Command helpers for queue execution

use crate::tui::command::Command;
use super::app::{State, Msg};
use super::models::{OperationStatus, QueueResult};
use std::collections::HashSet;

/// Helper function to save queue settings to database
/// Note: auto_play is NOT persisted (always starts paused)
pub fn save_settings_command(state: &State) -> Command<Msg> {
    let settings = crate::config::repository::queue::QueueSettings {
        auto_play: false, // Never persist auto_play - always start paused
        max_concurrent: state.max_concurrent,
        filter: state.filter,
        sort_mode: state.sort_mode,
    };

    Command::perform(
        async move {
            crate::global_config().save_queue_settings(&settings).await
                .map_err(|e| format!("Failed to save queue settings: {}", e))
        },
        |result| {
            if let Err(err) = result {
                Msg::PersistenceError(err)
            } else {
                Msg::PersistenceError("".to_string())
            }
        }
    )
}

/// Helper function to execute the next available operation
pub fn execute_next_if_available(state: &mut State) -> Command<Msg> {
    // Check if we can run more
    if state.currently_running.len() >= state.max_concurrent {
        return Command::None;
    }

    // Find next pending (not paused) item by priority
    let next = state
        .queue_items
        .iter()
        .filter(|item| item.status == OperationStatus::Pending)
        .min_by_key(|item| item.priority)
        .map(|item| item.id.clone());

    if let Some(id) = next {
        // Mark as running immediately and set start time
        if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
            item.status = OperationStatus::Running;
            item.started_at = Some(std::time::Instant::now());
            state.currently_running.insert(id.clone());
        }

        // Persist Running status to database
        let item_id_for_persist = id.clone();
        let persist_cmd = Command::perform(
            async move {
                log::info!("Persisting Running status for item: {}", item_id_for_persist);
                let result = crate::global_config().update_queue_item_status(&item_id_for_persist, OperationStatus::Running).await;
                match &result {
                    Ok(_) => log::info!("Successfully persisted Running status for item: {}", item_id_for_persist),
                    Err(e) => log::error!("Failed to persist Running status: {}", e),
                }
                result.map_err(|e| format!("Failed to persist Running status: {}", e))
            },
            |result| {
                if let Err(err) = result {
                    Msg::PersistenceError(err)
                } else {
                    Msg::PersistenceError("".to_string())
                }
            }
        );

        // Get item for execution
        let item = state.queue_items.iter().find(|i| i.id == id).cloned();

        if let Some(item) = item {
            let exec_cmd = Command::perform(
                async move {
                    use crate::api::resilience::ResilienceConfig;
                    let start = std::time::Instant::now();

                    // Get client for this environment from global client manager
                    let client = match crate::client_manager().get_client(&item.metadata.environment_name).await {
                        Ok(client) => client,
                        Err(e) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            return (item.id.clone(), QueueResult {
                                success: false,
                                operation_results: vec![],
                                error: Some(format!("Failed to get client: {}", e)),
                                duration_ms,
                            });
                        }
                    };

                    let resilience = ResilienceConfig::default();
                    let result = item.operations.execute(&client, &resilience).await;
                    let duration_ms = start.elapsed().as_millis() as u64;

                    let queue_result = match result {
                        Ok(operation_results) => QueueResult {
                            success: operation_results.iter().all(|r| r.success),
                            operation_results,
                            error: None,
                            duration_ms,
                        },
                        Err(e) => QueueResult {
                            success: false,
                            operation_results: vec![],
                            error: Some(e.to_string()),
                            duration_ms,
                        },
                    };

                    (item.id.clone(), queue_result)
                },
                |(id, result)| Msg::ExecutionCompleted(id, result),
            );

            Command::Batch(vec![persist_cmd, exec_cmd])
        } else {
            persist_cmd
        }
    } else {
        Command::None
    }
}
