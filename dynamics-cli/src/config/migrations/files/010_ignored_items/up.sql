-- Ignored items for entity comparison
CREATE TABLE ignored_items (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    item_id TEXT NOT NULL, -- Format: "tab:side:node_id" (e.g., "fields:source:cr123_fieldname")
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, item_id)
);
