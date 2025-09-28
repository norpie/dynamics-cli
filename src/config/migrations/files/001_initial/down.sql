-- Rollback initial schema

-- Drop triggers first
DROP TRIGGER IF EXISTS ensure_single_current_env_insert;
DROP TRIGGER IF EXISTS ensure_single_current_env;

-- Drop tables in reverse dependency order
DROP TABLE IF EXISTS view_comparisons;
DROP TABLE IF EXISTS comparisons;
DROP TABLE IF EXISTS migrations;
DROP TABLE IF EXISTS example_pairs;
DROP TABLE IF EXISTS settings;
DROP TABLE IF EXISTS prefix_mappings;
DROP TABLE IF EXISTS field_mappings;
DROP TABLE IF EXISTS entity_mappings;
DROP TABLE IF EXISTS tokens;
DROP TABLE IF EXISTS environments;
DROP TABLE IF EXISTS credentials;