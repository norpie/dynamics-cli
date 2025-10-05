-- Create update_metadata table for storing update-related metadata
CREATE TABLE IF NOT EXISTS update_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_update_metadata_key ON update_metadata(key);
