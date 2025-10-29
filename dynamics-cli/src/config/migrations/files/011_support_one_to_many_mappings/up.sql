-- Migration to support 1-to-N field mappings (one source → multiple targets)
-- Changes UNIQUE constraint from (source) to (source, target) to allow both N-to-1 and 1-to-N

-- Create new table with updated constraint
CREATE TABLE field_mappings_new (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_field TEXT NOT NULL,
    target_field TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_field, target_field)
);

-- Copy all existing data
INSERT INTO field_mappings_new
SELECT id, source_entity, target_entity, source_field, target_field, created_at
FROM field_mappings;

-- Drop old table
DROP TABLE field_mappings;

-- Rename new table
ALTER TABLE field_mappings_new RENAME TO field_mappings;

-- Create index for query performance (lookup by source)
CREATE INDEX idx_field_mappings_source_lookup
ON field_mappings(source_entity, target_entity, source_field);

-- Create index for reverse lookup (lookup by target)
CREATE INDEX idx_field_mappings_target_lookup
ON field_mappings(source_entity, target_entity, target_field);


-- Apply same changes to prefix_mappings table
CREATE TABLE prefix_mappings_new (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_prefix TEXT NOT NULL,
    target_prefix TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_prefix, target_prefix)
);

INSERT INTO prefix_mappings_new
SELECT id, source_entity, target_entity, source_prefix, target_prefix, created_at
FROM prefix_mappings;

DROP TABLE prefix_mappings;

ALTER TABLE prefix_mappings_new RENAME TO prefix_mappings;

CREATE INDEX idx_prefix_mappings_source_lookup
ON prefix_mappings(source_entity, target_entity, source_prefix);

CREATE INDEX idx_prefix_mappings_target_lookup
ON prefix_mappings(source_entity, target_entity, target_prefix);
