//! Field and prefix mappings repository

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Get all field mappings for a source/target entity pair
pub async fn get_field_mappings(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT source_field, target_field FROM field_mappings
         WHERE source_entity = ? AND target_entity = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .fetch_all(pool)
    .await
    .context("Failed to get field mappings")?;

    Ok(rows.into_iter().collect())
}

/// Set a field mapping (insert or update)
pub async fn set_field_mapping(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
    source_field: &str,
    target_field: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO field_mappings (source_entity, target_entity, source_field, target_field)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(source_entity, target_entity, source_field)
         DO UPDATE SET target_field = excluded.target_field",
    )
    .bind(source_entity)
    .bind(target_entity)
    .bind(source_field)
    .bind(target_field)
    .execute(pool)
    .await
    .context("Failed to set field mapping")?;

    Ok(())
}

/// Delete a field mapping
pub async fn delete_field_mapping(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
    source_field: &str,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM field_mappings
         WHERE source_entity = ? AND target_entity = ? AND source_field = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .bind(source_field)
    .execute(pool)
    .await
    .context("Failed to delete field mapping")?;

    Ok(())
}

/// Get all prefix mappings for a source/target entity pair
pub async fn get_prefix_mappings(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
) -> Result<HashMap<String, String>> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT source_prefix, target_prefix FROM prefix_mappings
         WHERE source_entity = ? AND target_entity = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .fetch_all(pool)
    .await
    .context("Failed to get prefix mappings")?;

    Ok(rows.into_iter().collect())
}

/// Set a prefix mapping (insert or update)
pub async fn set_prefix_mapping(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
    source_prefix: &str,
    target_prefix: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO prefix_mappings (source_entity, target_entity, source_prefix, target_prefix)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(source_entity, target_entity, source_prefix)
         DO UPDATE SET target_prefix = excluded.target_prefix",
    )
    .bind(source_entity)
    .bind(target_entity)
    .bind(source_prefix)
    .bind(target_prefix)
    .execute(pool)
    .await
    .context("Failed to set prefix mapping")?;

    Ok(())
}

/// Delete a prefix mapping
pub async fn delete_prefix_mapping(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
    source_prefix: &str,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM prefix_mappings
         WHERE source_entity = ? AND target_entity = ? AND source_prefix = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .bind(source_prefix)
    .execute(pool)
    .await
    .context("Failed to delete prefix mapping")?;

    Ok(())
}

/// Get imported mappings for a source/target entity pair
pub async fn get_imported_mappings(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
) -> Result<(HashMap<String, String>, Option<String>)> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT source_field, target_field, source_file FROM imported_mappings
         WHERE source_entity = ? AND target_entity = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .fetch_all(pool)
    .await
    .context("Failed to get imported mappings")?;

    // Extract mappings and source file
    let mappings: HashMap<String, String> = rows.iter()
        .map(|(src, tgt, _)| (src.clone(), tgt.clone()))
        .collect();

    // Get the source file from the first row (all rows should have the same source_file)
    let source_file = rows.first().map(|(_, _, file)| file.clone());

    Ok((mappings, source_file))
}

/// Set imported mappings (clears existing imports for this entity pair and inserts new ones)
pub async fn set_imported_mappings(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
    mappings: &HashMap<String, String>,
    source_file: &str,
) -> Result<()> {
    // Start transaction
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    // Clear existing imported mappings for this entity pair
    sqlx::query(
        "DELETE FROM imported_mappings
         WHERE source_entity = ? AND target_entity = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .execute(&mut *tx)
    .await
    .context("Failed to clear existing imported mappings")?;

    // Insert new mappings
    for (source_field, target_field) in mappings {
        sqlx::query(
            "INSERT INTO imported_mappings (source_entity, target_entity, source_field, target_field, source_file)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(source_entity)
        .bind(target_entity)
        .bind(source_field)
        .bind(target_field)
        .bind(source_file)
        .execute(&mut *tx)
        .await
        .context("Failed to insert imported mapping")?;
    }

    // Commit transaction
    tx.commit().await.context("Failed to commit transaction")?;

    Ok(())
}

/// Clear all imported mappings for a source/target entity pair
pub async fn clear_imported_mappings(
    pool: &SqlitePool,
    source_entity: &str,
    target_entity: &str,
) -> Result<()> {
    sqlx::query(
        "DELETE FROM imported_mappings
         WHERE source_entity = ? AND target_entity = ?",
    )
    .bind(source_entity)
    .bind(target_entity)
    .execute(pool)
    .await
    .context("Failed to clear imported mappings")?;

    Ok(())
}
