use anyhow::Result;

mod auth_selector;
mod file_browser;
mod excel_parser;
mod sheet_selector;
mod config;
mod entity_discovery;
mod setup_tui;
mod loading_modal;
mod csv_loading_modal;
mod validation;
mod validation_popup;
mod csv_cache;
mod csv_cache_tui;
mod field_mapping_tui;
mod hardcoded_field_mapping;
mod data_transformer;
mod timezone_utils;
mod validation_errors_tui;

use auth_selector::{run_auth_selector, AuthSelectorResult};
use file_browser::run_file_browser;
use sheet_selector::run_sheet_selector;
use excel_parser::ExcelWorkbook;
use config::DeadlineConfig;
use setup_tui::run_deadline_setup;
use validation::validate_excel_entities;
use validation_popup::show_validation_popup;
use csv_cache_tui::run_csv_cache_check;
use hardcoded_field_mapping::create_hardcoded_field_mappings;
use data_transformer::DataTransformer;
use loading_modal::LoadingModal;
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;
use std::time::{Duration, Instant};

async fn show_loading_and_transform(
    transformer: DataTransformer,
    sheet_data: &crate::commands::deadlines::excel_parser::SheetData,
    _validation_result: Option<&crate::commands::deadlines::validation::ValidationResult>,
) -> Result<Vec<crate::commands::deadlines::data_transformer::TransformedRecord>> {

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut loading_modal = LoadingModal::new("Transforming data...".to_string());
    let start_time = Instant::now();

    // Clone sheet_data for the async task
    let sheet_data_clone = sheet_data.clone();

    // Start transformation task immediately in background
    let mut transform_task = tokio::spawn(async move {
        transformer.transform_sheet_data(&sheet_data_clone).await
    });

    let mut transformation_result = None;

    loop {
        // Update UI
        terminal.draw(|f| {
            loading_modal.render(f, f.area());
        })?;

        loading_modal.tick();

        // Check for user input and task completion concurrently
        tokio::select! {
            // Check if transformation is complete
            result = &mut transform_task, if transformation_result.is_none() => {
                transformation_result = Some(result.map_err(|e| anyhow::anyhow!("Task join error: {}", e))?);
            }

            // Short sleep to keep UI responsive
            _ = tokio::time::sleep(Duration::from_millis(50)) => {
                // Continue the loop
            }
        }

        // Exit after transformation is complete and minimum display time
        if transformation_result.is_some() && start_time.elapsed() >= Duration::from_millis(1000) {
            // Check for any key press to skip loading
            if event::poll(Duration::from_millis(0))? {
                if let Event::Key(_) = event::read()? {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;

    transformation_result.unwrap()
}

async fn continue_with_file_selection(selected_env: String) -> Result<()> {
    // Phase 3: Select file
    if let Some(file_path) = run_file_browser(selected_env.clone()).await? {
        // Phase 4: Select Excel sheet and process
        if let Some((file_path, sheet_name)) = run_sheet_selector(file_path).await? {
            // Phase 5: CSV Cache Check & Refresh (self-healing)
            let deadline_config = DeadlineConfig::load()?;
            let env_config = deadline_config.get_environment(&selected_env)
                .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", selected_env))?;

            let config = crate::config::Config::load()?;
            let auth_config = config.environments.get(&selected_env)
                .ok_or_else(|| anyhow::anyhow!("Auth environment '{}' not found", selected_env))?;

            // Check CSV cache and refresh if needed
            if !run_csv_cache_check(selected_env.clone(), env_config, auth_config, false).await? {
                return Ok(());
            }

            // Phase 5.5: Pre-validate Excel structure and show warnings
            match ExcelWorkbook::read_sheet(&file_path, &sheet_name) {
                Ok(sheet_data) => {
                    // Run validation to identify missing entities
                    if let Ok(validation_result) = validate_excel_entities(&sheet_data, &selected_env, &deadline_config) {
                        if !validation_result.unmatched_columns.is_empty() {
                            // Show popup with unmatched entities - user can continue or quit
                            if !show_validation_popup(&validation_result)? {
                                // User chose to quit
                                return Ok(());
                            }
                            // User chose to continue despite unmatched entities
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Could not pre-validate Excel file: {}", e);
                }
            }

            // Phase 6: Parse sheet and validate against entity mappings
            match ExcelWorkbook::read_sheet(&file_path, &sheet_name) {
                Ok(sheet_data) => {
                    // Phase 7: Validate Excel structure against entity mappings
                    match validate_excel_entities(&sheet_data, &selected_env, &deadline_config) {
                        Ok(_validation_result) => {
                            // Validation passed, continue with processing
                        }
                        Err(e) => {
                            log::error!("Validation failed: {}", e);
                            return Ok(());
                        }
                    }

                    // Phase 8: Field Mapping (Hardcoded)
                    let validation_result = validate_excel_entities(&sheet_data, &selected_env, &deadline_config)?;

                    match create_hardcoded_field_mappings(&sheet_data, env_config, &validation_result) {
                        Ok(field_mappings) => {
                            if field_mappings.is_empty() {
                                return Ok(());
                            }

                            let transformer = DataTransformer::new(
                                env_config.clone(),
                                selected_env.clone(),
                                field_mappings,
                            );

                            // Show loading screen and perform transformation
                            match show_loading_and_transform(transformer, &sheet_data, Some(&validation_result)).await {
                                Ok(transformed_records) => {
                                    // Calculate warnings for validation TUI
                                    let total_warnings = transformed_records.iter()
                                        .map(|r| r.validation_warnings.len())
                                        .sum::<usize>();

                                    // Log all validation warnings for debugging
                                    if total_warnings > 0 {
                                        for record in transformed_records.iter() {
                                            if !record.validation_warnings.is_empty() {
                                                for (warning_idx, warning) in record.validation_warnings.iter().enumerate() {
                                                    log::warn!("Row {} Warning {}: {}", record.excel_row_number, warning_idx + 1, warning);
                                                }
                                            }
                                        }

                                        // Show validation errors TUI
                                        match validation_errors_tui::run_validation_errors_tui(&transformed_records, Some(&validation_result)) {
                                            Ok(should_continue) => {
                                                if !should_continue {
                                                    return Ok(());
                                                }
                                            }
                                            Err(e) => {
                                                log::warn!("Error displaying validation errors TUI: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::error!("Data transformation failed: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Field mapping failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading sheet '{}': {}", sheet_name, e);
                }
            }
        }
    }
    Ok(())
}

/// Entry point for deadlines TUI interface
pub async fn start() -> Result<()> {
    // Phase 1: Select authentication environment
    match run_auth_selector().await? {
        AuthSelectorResult::SelectedEnvironment(selected_env) => {
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

        // Continue with file selection
        continue_with_file_selection(selected_env).await?;
        }
        AuthSelectorResult::RerunSetup(selected_env) => {
            // User wants to re-run setup - overwrite existing configuration
            println!("Re-running deadline setup for environment '{}'...", selected_env);

            if let Some(env_config) = run_deadline_setup(selected_env.clone()).await? {
                let mut deadline_config = DeadlineConfig::load()?;
                deadline_config.add_environment(selected_env.clone(), env_config);
                deadline_config.save()?;
                println!("✅ Deadline setup completed for environment '{}'", selected_env);

                // Continue with the normal flow after successful setup
                continue_with_file_selection(selected_env).await?;
            } else {
                println!("Setup was cancelled.");
            }
        }
        AuthSelectorResult::Cancelled => {
            println!("Cancelled.");
        }
    }

    Ok(())
}