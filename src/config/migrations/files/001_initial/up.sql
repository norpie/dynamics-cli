-- Initial schema for Dynamics CLI configuration database

-- Named credential sets (shared across environments)
CREATE TABLE credentials (
    name TEXT PRIMARY KEY,
    type TEXT NOT NULL CHECK(type IN ('username_password', 'client_credentials', 'device_code', 'certificate')),
    data TEXT NOT NULL, -- JSON blob with type-specific fields
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Environments referencing credentials
CREATE TABLE environments (
    name TEXT PRIMARY KEY,
    host TEXT NOT NULL,
    credentials_ref TEXT NOT NULL,
    is_current BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (credentials_ref) REFERENCES credentials(name) ON UPDATE CASCADE
);

-- Token cache per environment
CREATE TABLE tokens (
    environment_name TEXT PRIMARY KEY,
    access_token TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    refresh_token TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (environment_name) REFERENCES environments(name) ON DELETE CASCADE
);

-- Entity name mappings (singular → plural)
CREATE TABLE entity_mappings (
    singular_name TEXT PRIMARY KEY,
    plural_name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Field mappings for entity comparisons
CREATE TABLE field_mappings (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_field TEXT NOT NULL,
    target_field TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_field)
);

-- Prefix replacement rules
CREATE TABLE prefix_mappings (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_prefix TEXT NOT NULL,
    target_prefix TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_prefix)
);

-- Saved migrations
CREATE TABLE migrations (
    name TEXT PRIMARY KEY,
    source_env TEXT NOT NULL,
    target_env TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Migration comparisons
CREATE TABLE comparisons (
    id INTEGER PRIMARY KEY,
    migration_name TEXT NOT NULL,
    name TEXT NOT NULL,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    entity_comparison TEXT, -- JSON
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (migration_name) REFERENCES migrations(name) ON DELETE CASCADE,
    UNIQUE(migration_name, name)
);

-- View comparisons
CREATE TABLE view_comparisons (
    id INTEGER PRIMARY KEY,
    comparison_id INTEGER NOT NULL,
    source_view_name TEXT NOT NULL,
    target_view_name TEXT NOT NULL,
    column_mappings TEXT, -- JSON
    filter_mappings TEXT, -- JSON
    sort_mappings TEXT,   -- JSON
    FOREIGN KEY (comparison_id) REFERENCES comparisons(id) ON DELETE CASCADE
);

-- Example pairs for testing
CREATE TABLE example_pairs (
    id TEXT PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_uuid TEXT NOT NULL,
    target_uuid TEXT NOT NULL,
    label TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- General settings (key-value)
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('string', 'integer', 'boolean', 'json'))
);

-- Ensure only one current environment
CREATE TRIGGER ensure_single_current_env
    BEFORE UPDATE ON environments
    WHEN NEW.is_current = TRUE
BEGIN
    UPDATE environments SET is_current = FALSE WHERE is_current = TRUE AND name != NEW.name;
END;

CREATE TRIGGER ensure_single_current_env_insert
    BEFORE INSERT ON environments
    WHEN NEW.is_current = TRUE
BEGIN
    UPDATE environments SET is_current = FALSE WHERE is_current = TRUE;
END;