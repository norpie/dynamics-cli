-- Performance indexes for the configuration database

-- Indexes for credentials table
CREATE INDEX idx_credentials_type ON credentials(type);
CREATE INDEX idx_credentials_created_at ON credentials(created_at);

-- Indexes for environments table
CREATE INDEX idx_environments_host ON environments(host);
CREATE INDEX idx_environments_credentials_ref ON environments(credentials_ref);
CREATE INDEX idx_environments_is_current ON environments(is_current) WHERE is_current = TRUE;
CREATE INDEX idx_environments_created_at ON environments(created_at);

-- Indexes for tokens table
CREATE INDEX idx_tokens_expires_at ON tokens(expires_at);
CREATE INDEX idx_tokens_updated_at ON tokens(updated_at);

-- Indexes for entity mappings
CREATE INDEX idx_entity_mappings_plural ON entity_mappings(plural_name);
CREATE INDEX idx_entity_mappings_created_at ON entity_mappings(created_at);

-- Indexes for field mappings
CREATE INDEX idx_field_mappings_source_entity ON field_mappings(source_entity);
CREATE INDEX idx_field_mappings_target_entity ON field_mappings(target_entity);
CREATE INDEX idx_field_mappings_entity_pair ON field_mappings(source_entity, target_entity);
CREATE INDEX idx_field_mappings_created_at ON field_mappings(created_at);

-- Indexes for prefix mappings
CREATE INDEX idx_prefix_mappings_source_entity ON prefix_mappings(source_entity);
CREATE INDEX idx_prefix_mappings_target_entity ON prefix_mappings(target_entity);
CREATE INDEX idx_prefix_mappings_entity_pair ON prefix_mappings(source_entity, target_entity);
CREATE INDEX idx_prefix_mappings_created_at ON prefix_mappings(created_at);

-- Indexes for migrations
CREATE INDEX idx_migrations_source_env ON migrations(source_env);
CREATE INDEX idx_migrations_target_env ON migrations(target_env);
CREATE INDEX idx_migrations_last_used ON migrations(last_used);
CREATE INDEX idx_migrations_created_at ON migrations(created_at);

-- Indexes for comparisons
CREATE INDEX idx_comparisons_migration_name ON comparisons(migration_name);
CREATE INDEX idx_comparisons_source_entity ON comparisons(source_entity);
CREATE INDEX idx_comparisons_target_entity ON comparisons(target_entity);
CREATE INDEX idx_comparisons_last_used ON comparisons(last_used);
CREATE INDEX idx_comparisons_created_at ON comparisons(created_at);

-- Indexes for view comparisons
CREATE INDEX idx_view_comparisons_comparison_id ON view_comparisons(comparison_id);
CREATE INDEX idx_view_comparisons_source_view ON view_comparisons(source_view_name);
CREATE INDEX idx_view_comparisons_target_view ON view_comparisons(target_view_name);

-- Indexes for example pairs
CREATE INDEX idx_example_pairs_source_entity ON example_pairs(source_entity);
CREATE INDEX idx_example_pairs_target_entity ON example_pairs(target_entity);
CREATE INDEX idx_example_pairs_entity_pair ON example_pairs(source_entity, target_entity);
CREATE INDEX idx_example_pairs_created_at ON example_pairs(created_at);

-- Indexes for settings
CREATE INDEX idx_settings_type ON settings(type);