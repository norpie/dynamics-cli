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
