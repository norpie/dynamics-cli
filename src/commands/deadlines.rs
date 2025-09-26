use anyhow::Result;

mod auth_selector;
mod file_browser;
mod excel_parser;
mod sheet_selector;
mod config;
mod entity_discovery;
mod setup_tui;
mod loading_modal;
mod validation;

use auth_selector::run_auth_selector;
use file_browser::run_file_browser;
use sheet_selector::run_sheet_selector;
use excel_parser::ExcelWorkbook;
use config::DeadlineConfig;
use setup_tui::run_deadline_setup;
use validation::validate_excel_entities;

/// Entry point for deadlines TUI interface
pub async fn start() -> Result<()> {
    // Phase 1: Select authentication environment
    if let Some(selected_env) = run_auth_selector().await? {
        // Phase 2: Check deadline configuration, run setup if missing
        let mut deadline_config = DeadlineConfig::load()?;

        if !deadline_config.has_environment(&selected_env) {
            // Run setup TUI seamlessly
            if let Some(env_config) = run_deadline_setup(selected_env.clone()).await? {
                deadline_config.add_environment(selected_env.clone(), env_config);
                deadline_config.save()?;
            } else {
                // Setup was cancelled, exit gracefully
                return Ok(());
            }
        }

        // Phase 3: Select file
        if let Some(file_path) = run_file_browser(selected_env.clone()).await? {
            // Phase 4: Select Excel sheet and process
            if let Some((file_path, sheet_name)) = run_sheet_selector(file_path).await? {
                // Phase 5: Parse sheet and validate against entity mappings
                match ExcelWorkbook::read_sheet(&file_path, &sheet_name) {
                    Ok(sheet_data) => {
                        println!("Environment: {}", selected_env);
                        println!("File: {}", file_path);
                        println!("Sheet: {}", sheet_name);
                        println!("Rows: {}, Columns: {}", sheet_data.row_count(), sheet_data.column_count());

                        // Phase 6: Validate Excel structure against entity mappings
                        println!("\n{}", "=".repeat(60));
                        println!("ENTITY VALIDATION");
                        println!("{}", "=".repeat(60));

                        let deadline_config = DeadlineConfig::load()?;
                        match validate_excel_entities(&sheet_data, &selected_env, &deadline_config) {
                            Ok(validation_result) => {
                                println!("{}", validation_result.summary());

                                if !validation_result.matched_entities.is_empty() {
                                    println!("\n✅ Matched Entities:");
                                    for entity_match in &validation_result.matched_entities {
                                        println!("  • '{}' → {} ({})",
                                                entity_match.column_header,
                                                entity_match.entity_name,
                                                entity_match.logical_type);
                                    }
                                }

                                if !validation_result.unmatched_columns.is_empty() {
                                    println!("\n❌ Unmatched Columns (no entity mapping found):");
                                    for column in &validation_result.unmatched_columns {
                                        println!("  • '{}'", column);
                                    }
                                }

                                if !validation_result.missing_entities.is_empty() {
                                    println!("\n⚠️  Missing Entities (configured but not in Excel):");
                                    for entity in &validation_result.missing_entities {
                                        println!("  • {}", entity);
                                    }
                                }

                                if validation_result.is_valid() {
                                    println!("\n🎉 All Excel columns successfully matched to configured entities!");
                                } else {
                                    println!("\n⚠️  Some Excel columns could not be matched. Please check your setup or Excel headers.");
                                }
                            }
                            Err(e) => {
                                println!("❌ Validation failed: {}", e);
                            }
                        }

                        println!("\n{}", "=".repeat(60));
                        println!("CSV OUTPUT");
                        println!("{}", "=".repeat(60));
                        println!("{}", sheet_data.to_csv());
                    }
                    Err(e) => {
                        println!("Error reading sheet '{}': {}", sheet_name, e);
                    }
                }
            }
        }
    }

    Ok(())
}