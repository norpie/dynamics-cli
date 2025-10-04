-- Create options table for persistent key-value configuration
CREATE TABLE IF NOT EXISTS options (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_options_key ON options(key);
