-- Entity data cache for storing fetched entity records (for lookups/checkboxes)
CREATE TABLE entity_data_cache (
    environment_name TEXT NOT NULL,
    entity_name TEXT NOT NULL,
    data TEXT NOT NULL, -- JSON array of entity records
    cached_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (environment_name, entity_name),
    FOREIGN KEY (environment_name) REFERENCES environments(name) ON DELETE CASCADE
);
