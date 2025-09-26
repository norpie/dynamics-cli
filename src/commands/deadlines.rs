use anyhow::Result;

mod auth_selector;
mod file_browser;
mod excel_parser;
mod sheet_selector;
mod config;
mod entity_discovery;
mod setup_tui;
mod loading_modal;

use auth_selector::run_auth_selector;
use file_browser::run_file_browser;
use sheet_selector::run_sheet_selector;
use excel_parser::ExcelWorkbook;
use config::DeadlineConfig;
use setup_tui::run_deadline_setup;

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
                // Phase 5: Parse sheet and output as CSV (only print after TUI is done)
                match ExcelWorkbook::read_sheet(&file_path, &sheet_name) {
                    Ok(sheet_data) => {
                        println!("Environment: {}", selected_env);
                        println!("File: {}", file_path);
                        println!("Sheet: {}", sheet_name);
                        println!("Rows: {}, Columns: {}", sheet_data.row_count(), sheet_data.column_count());
                        println!("\nCSV Output:");
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