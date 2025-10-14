-- Imported field mappings from C# files
CREATE TABLE imported_mappings (
    id INTEGER PRIMARY KEY,
    source_entity TEXT NOT NULL,
    target_entity TEXT NOT NULL,
    source_field TEXT NOT NULL,
    target_field TEXT NOT NULL,
    source_file TEXT NOT NULL, -- filename of the C# file
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(source_entity, target_entity, source_field)
);
