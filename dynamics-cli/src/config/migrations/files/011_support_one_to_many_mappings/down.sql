-- Revert migration: restore original UNIQUE(source) constraint
-- WARNING: This will DELETE duplicate source→target mappings if 1-to-N mappings exist

-- Revert field_mappings table
DROP INDEX IF EXISTS idx_field_mappings_source_lookup;
DROP INDEX IF EXISTS idx_field_mappings_target_lookup;

CREATE TABLE field_mappings_old (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_field TEXT NOT NULL,
    target_field TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_field)
);

-- Copy data, keeping only first target for each source (loses 1-to-N mappings)
INSERT INTO field_mappings_old
SELECT MIN(id) as id, source_entity, target_entity, source_field,
       (SELECT target_field FROM field_mappings fm2
        WHERE fm2.source_entity = fm1.source_entity
          AND fm2.target_entity = fm1.target_entity
          AND fm2.source_field = fm1.source_field
        ORDER BY id LIMIT 1) as target_field,
       MIN(created_at) as created_at
FROM field_mappings fm1
GROUP BY source_entity, target_entity, source_field;

DROP TABLE field_mappings;

ALTER TABLE field_mappings_old RENAME TO field_mappings;


-- Revert prefix_mappings table
CREATE TABLE prefix_mappings_old (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_prefix TEXT NOT NULL,
    target_prefix TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_prefix)
);

INSERT INTO prefix_mappings_old
SELECT MIN(id) as id, source_entity, target_entity, source_prefix,
       (SELECT target_prefix FROM prefix_mappings pm2
        WHERE pm2.source_entity = pm1.source_entity
          AND pm2.target_entity = pm1.target_entity
          AND pm2.source_prefix = pm1.source_prefix
        ORDER BY id LIMIT 1) as target_prefix,
       MIN(created_at) as created_at
FROM prefix_mappings pm1
GROUP BY source_entity, target_entity, source_prefix;

DROP TABLE prefix_mappings;

ALTER TABLE prefix_mappings_old RENAME TO prefix_mappings;
