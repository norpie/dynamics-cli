-- Remove performance indexes

-- Drop indexes for settings
DROP INDEX IF EXISTS idx_settings_type;

-- Drop indexes for example pairs
DROP INDEX IF EXISTS idx_example_pairs_created_at;
DROP INDEX IF EXISTS idx_example_pairs_entity_pair;
DROP INDEX IF EXISTS idx_example_pairs_target_entity;
DROP INDEX IF EXISTS idx_example_pairs_source_entity;

-- Drop indexes for view comparisons
DROP INDEX IF EXISTS idx_view_comparisons_target_view;
DROP INDEX IF EXISTS idx_view_comparisons_source_view;
DROP INDEX IF EXISTS idx_view_comparisons_comparison_id;

-- Drop indexes for comparisons
DROP INDEX IF EXISTS idx_comparisons_created_at;
DROP INDEX IF EXISTS idx_comparisons_last_used;
DROP INDEX IF EXISTS idx_comparisons_target_entity;
DROP INDEX IF EXISTS idx_comparisons_source_entity;
DROP INDEX IF EXISTS idx_comparisons_migration_name;

-- Drop indexes for migrations
DROP INDEX IF EXISTS idx_migrations_created_at;
DROP INDEX IF EXISTS idx_migrations_last_used;
DROP INDEX IF EXISTS idx_migrations_target_env;
DROP INDEX IF EXISTS idx_migrations_source_env;

-- Drop indexes for prefix mappings
DROP INDEX IF EXISTS idx_prefix_mappings_created_at;
DROP INDEX IF EXISTS idx_prefix_mappings_entity_pair;
DROP INDEX IF EXISTS idx_prefix_mappings_target_entity;
DROP INDEX IF EXISTS idx_prefix_mappings_source_entity;

-- Drop indexes for field mappings
DROP INDEX IF EXISTS idx_field_mappings_created_at;
DROP INDEX IF EXISTS idx_field_mappings_entity_pair;
DROP INDEX IF EXISTS idx_field_mappings_target_entity;
DROP INDEX IF EXISTS idx_field_mappings_source_entity;

-- Drop indexes for entity mappings
DROP INDEX IF EXISTS idx_entity_mappings_created_at;
DROP INDEX IF EXISTS idx_entity_mappings_plural;

-- Drop indexes for tokens table
DROP INDEX IF EXISTS idx_tokens_updated_at;
DROP INDEX IF EXISTS idx_tokens_expires_at;

-- Drop indexes for environments table
DROP INDEX IF EXISTS idx_environments_created_at;
DROP INDEX IF EXISTS idx_environments_is_current;
DROP INDEX IF EXISTS idx_environments_credentials_ref;
DROP INDEX IF EXISTS idx_environments_host;

-- Drop indexes for credentials table
DROP INDEX IF EXISTS idx_credentials_created_at;
DROP INDEX IF EXISTS idx_credentials_type;