/// Rollback operations for cleaning up partially created entities

use crate::api::{ResilienceConfig};
use crate::api::operations::{Operation, Operations};
use std::fs::File;
use std::io::Write;

/// Export orphaned entities to CSV for manual cleanup
fn export_orphaned_entities_csv(entities: &[(String, String)]) -> Result<String, String> {
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("orphaned_entities_{}.csv", timestamp);

    // Save to user's Downloads folder - visible and accessible when rollback fails
    let downloads_dir = dirs::download_dir()
        .ok_or_else(|| "Could not determine downloads directory".to_string())?;

    let path = downloads_dir.join(&filename);

    let mut file = File::create(&path)
        .map_err(|e| format!("Failed to create CSV file: {}", e))?;

    // Write CSV header
    writeln!(file, "entity_set,entity_id")
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write entities in REVERSE order (deletion order: children before parents)
    for (entity_set, entity_id) in entities.iter().rev() {
        writeln!(file, "{},{}", entity_set, entity_id)
            .map_err(|e| format!("Failed to write entity to CSV: {}", e))?;
    }

    log::info!("Exported {} orphaned entities to: {}", entities.len(), path.display());

    Ok(path.to_string_lossy().to_string())
}

/// Rollback all created entities in reverse order
/// Returns Ok(()) if rollback succeeded, Err(csv_path) if it failed
pub async fn rollback_created_entities(
    created_ids: Vec<(String, String)>,
) -> Result<(), String> {
    if created_ids.is_empty() {
        return Ok(()); // Nothing to rollback
    }

    log::info!("Starting rollback of {} entities", created_ids.len());

    let client_manager = crate::client_manager();

    // Get client
    let env_name = match client_manager.get_current_environment_name().await {
        Ok(Some(name)) => name,
        _ => {
            log::error!("Rollback failed: Could not get environment name");
            let csv_path = export_orphaned_entities_csv(&created_ids)
                .unwrap_or_else(|e| format!("(CSV export also failed: {})", e));
            return Err(csv_path);
        }
    };

    let client = match client_manager.get_client(&env_name).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("Rollback failed: Could not get client: {}", e);
            let csv_path = export_orphaned_entities_csv(&created_ids)
                .unwrap_or_else(|e| format!("(CSV export also failed: {})", e));
            return Err(csv_path);
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
                Ok(())
            } else {
                log::warn!("Rollback partially failed - some entities may remain");
                let csv_path = export_orphaned_entities_csv(&created_ids)
                    .unwrap_or_else(|e| format!("(CSV export also failed: {})", e));
                Err(csv_path)
            }
        }
        Err(e) => {
            log::error!("Rollback batch operation failed: {}", e);
            let csv_path = export_orphaned_entities_csv(&created_ids)
                .unwrap_or_else(|e| format!("(CSV export also failed: {})", e));
            Err(csv_path)
        }
    }
}
