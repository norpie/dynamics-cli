-- Add entity metadata cache table
CREATE TABLE entity_metadata_cache (
    environment_name TEXT NOT NULL,
    entity_name TEXT NOT NULL,
    metadata TEXT NOT NULL, -- JSON serialized EntityMetadata
    cached_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (environment_name, entity_name),
    FOREIGN KEY (environment_name) REFERENCES environments(name) ON DELETE CASCADE
);
