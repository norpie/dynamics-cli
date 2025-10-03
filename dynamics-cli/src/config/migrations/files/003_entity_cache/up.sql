-- Entity cache table for storing metadata entity lists per environment
CREATE TABLE entity_cache (
    environment_name TEXT PRIMARY KEY,
    entities TEXT NOT NULL, -- JSON array of entity names
    cached_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (environment_name) REFERENCES environments(name) ON DELETE CASCADE
);
