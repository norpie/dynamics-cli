//! Proper migration framework for configuration database

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::collections::BTreeMap;

pub mod manager;

pub use manager::MigrationManager;

/// Represents a single migration with up and down SQL
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: String,
    pub up_sql: String,
    pub down_sql: String,
}

/// Migration status in the database
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppliedMigration {
    pub version: i64,
    pub name: String,
    pub applied_at: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
}

/// Direction for migration operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// Load all available migrations from the embedded files
pub fn load_migrations() -> Result<BTreeMap<i64, Migration>> {
    let mut migrations = BTreeMap::new();

    // Migration 001: Initial schema
    migrations.insert(1, Migration {
        version: 1,
        name: "initial".to_string(),
        up_sql: include_str!("files/001_initial/up.sql").to_string(),
        down_sql: include_str!("files/001_initial/down.sql").to_string(),
    });

    // Migration 002: Indexes
    migrations.insert(2, Migration {
        version: 2,
        name: "indexes".to_string(),
        up_sql: include_str!("files/002_indexes/up.sql").to_string(),
        down_sql: include_str!("files/002_indexes/down.sql").to_string(),
    });

    Ok(migrations)
}

/// Initialize the migration tracking table
pub async fn init_migration_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            checksum TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create schema_migrations table")?;

    Ok(())
}

/// Get list of applied migrations
pub async fn get_applied_migrations(pool: &SqlitePool) -> Result<Vec<AppliedMigration>> {
    let migrations = sqlx::query_as::<_, AppliedMigration>(
        "SELECT version, name, applied_at, checksum FROM schema_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .context("Failed to get applied migrations")?;

    Ok(migrations)
}

/// Calculate checksum for migration SQL
pub fn calculate_checksum(sql: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    sql.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Validate that applied migrations match available ones
pub async fn validate_migrations(pool: &SqlitePool) -> Result<()> {
    let available = load_migrations()?;
    let applied = get_applied_migrations(pool).await?;

    for applied_migration in applied {
        if let Some(available_migration) = available.get(&applied_migration.version) {
            let expected_checksum = calculate_checksum(&available_migration.up_sql);
            if applied_migration.checksum != expected_checksum {
                anyhow::bail!(
                    "Migration {} checksum mismatch! Applied: {}, Expected: {}. \
                    This indicates the migration file has been modified after being applied.",
                    applied_migration.version,
                    applied_migration.checksum,
                    expected_checksum
                );
            }
        } else {
            anyhow::bail!(
                "Applied migration {} '{}' not found in available migrations",
                applied_migration.version,
                applied_migration.name
            );
        }
    }

    Ok(())
}

/// Get pending migrations (available but not applied)
pub async fn get_pending_migrations(pool: &SqlitePool) -> Result<Vec<Migration>> {
    let available = load_migrations()?;
    let applied = get_applied_migrations(pool).await?;

    let applied_versions: std::collections::HashSet<i64> =
        applied.into_iter().map(|m| m.version).collect();

    let mut pending = Vec::new();
    for (version, migration) in available {
        if !applied_versions.contains(&version) {
            pending.push(migration);
        }
    }

    Ok(pending)
}

/// Get the current schema version (highest applied migration)
pub async fn get_current_version(pool: &SqlitePool) -> Result<Option<i64>> {
    let version: Option<(i64,)> = sqlx::query_as(
        "SELECT MAX(version) FROM schema_migrations",
    )
    .fetch_optional(pool)
    .await
    .context("Failed to get current schema version")?;

    Ok(version.and_then(|(v,)| if v == 0 { None } else { Some(v) }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_migrations() {
        let migrations = load_migrations().unwrap();
        assert!(!migrations.is_empty());
        assert!(migrations.contains_key(&1));
        assert!(migrations.contains_key(&2));
    }

    #[test]
    fn test_calculate_checksum() {
        let sql = "CREATE TABLE test (id INTEGER);";
        let checksum1 = calculate_checksum(sql);
        let checksum2 = calculate_checksum(sql);
        assert_eq!(checksum1, checksum2);

        let different_sql = "CREATE TABLE test2 (id INTEGER);";
        let checksum3 = calculate_checksum(different_sql);
        assert_ne!(checksum1, checksum3);
    }
}