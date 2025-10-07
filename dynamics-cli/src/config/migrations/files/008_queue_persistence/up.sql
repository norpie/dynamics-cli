-- Queue persistence: queue items and settings

-- Queue items table
CREATE TABLE queue_items (
    id TEXT PRIMARY KEY,
    environment_name TEXT NOT NULL,
    operations_json TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Pending', 'Running', 'Paused', 'Done', 'Failed')),
    priority INTEGER NOT NULL,
    result_json TEXT,
    was_interrupted BOOLEAN DEFAULT FALSE,
    interrupted_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (environment_name) REFERENCES environments(name) ON DELETE CASCADE
);

CREATE INDEX idx_queue_items_status ON queue_items(status);
CREATE INDEX idx_queue_items_priority ON queue_items(priority);
CREATE INDEX idx_queue_items_environment ON queue_items(environment_name);
CREATE INDEX idx_queue_items_was_interrupted ON queue_items(was_interrupted);

-- Queue settings table (singleton - only one row allowed)
CREATE TABLE queue_settings (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    auto_play BOOLEAN DEFAULT FALSE,
    max_concurrent INTEGER DEFAULT 3,
    filter TEXT DEFAULT 'All',
    sort_mode TEXT DEFAULT 'Priority',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert default settings
INSERT INTO queue_settings (id, auto_play, max_concurrent, filter, sort_mode)
VALUES (1, FALSE, 3, 'All', 'Priority');
