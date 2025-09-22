mod app;
mod events;
mod popups;
mod ui;
mod navigation;
mod menus;
mod spinner;
mod export;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{DisableMouseCapture, EnableMouseCapture},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::cli::commands::migration::{CompareArgs, ExportArgs};
use crate::config::Config;
use crate::dynamics::client::DynamicsClient;
use crate::dynamics::metadata::{parse_entity_fields, FieldInfo};
use app::CompareApp;
use navigation::start_navigation;
use spinner::show_loading_while;

pub async fn start() -> Result<()> {
    start_navigation().await
}

pub async fn execute(args: CompareArgs) -> Result<()> {
    // If no arguments provided, start the navigation interface
    if args.source_entity.is_none() || args.source.is_none() || args.target.is_none() {
        return start_navigation().await;
    }

    // Extract required arguments
    let source_entity = args.source_entity.unwrap();
    let source_env = args.source.unwrap();
    let target_env = args.target.unwrap();
    let target_entity = args.target_entity.unwrap_or_else(|| source_entity.clone());
    // Load configuration
    let config = Config::load()?;

    // Validate environments exist
    if !config.environments.contains_key(&source_env) {
        anyhow::bail!("Source environment '{}' not found", source_env);
    }
    if !config.environments.contains_key(&target_env) {
        anyhow::bail!("Target environment '{}' not found", target_env);
    }

    // Get authentication configs for both environments
    let source_auth = config.environments.get(&source_env)
        .ok_or_else(|| anyhow::anyhow!("Source environment '{}' not found", source_env))?;
    let target_auth = config.environments.get(&target_env)
        .ok_or_else(|| anyhow::anyhow!("Target environment '{}' not found", target_env))?;

    // Prepare loading message
    let loading_message = if source_entity == target_entity {
        format!("Fetching entity metadata for '{}' from both environments...", source_entity)
    } else {
        format!("Fetching entity metadata for '{}' from source and '{}' from target...", source_entity, target_entity)
    };

    // Fetch entity metadata from both environments with spinner
    let (source_fields, target_fields) = show_loading_while(
        loading_message,
        || {
            let source_auth = source_auth.clone();
            let target_auth = target_auth.clone();
            let source_entity = source_entity.clone();
            let target_entity = target_entity.clone();

            async move {
                let mut source_client = DynamicsClient::new(source_auth);
                let mut target_client = DynamicsClient::new(target_auth);

                let source_fields = fetch_entity_fields(&mut source_client, &source_entity).await?;
                let target_fields = fetch_entity_fields(&mut target_client, &target_entity).await?;
                Ok::<(Vec<FieldInfo>, Vec<FieldInfo>), anyhow::Error>((source_fields, target_fields))
            }
        }
    ).await?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = CompareApp::new(
        source_fields,
        target_fields,
        source_entity,
        target_entity,
        source_env,
        target_env,
    );

    // Load existing field mappings
    app.load_field_mappings(&config);

    // Load existing prefix mappings
    app.load_prefix_mappings(&config);

    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

pub async fn export(args: ExportArgs) -> Result<()> {
    let source_entity = args.source_entity;
    let source_env = args.source;
    let target_env = args.target;
    let target_entity = args.target_entity.unwrap_or_else(|| source_entity.clone());
    let output_path = args.output;

    // Load configuration
    let config = Config::load()?;

    // Validate environments exist
    if !config.environments.contains_key(&source_env) {
        anyhow::bail!("Source environment '{}' not found", source_env);
    }
    if !config.environments.contains_key(&target_env) {
        anyhow::bail!("Target environment '{}' not found", target_env);
    }

    // Get authentication configs for both environments
    let source_auth = config.environments.get(&source_env)
        .ok_or_else(|| anyhow::anyhow!("Source environment '{}' not found", source_env))?;
    let target_auth = config.environments.get(&target_env)
        .ok_or_else(|| anyhow::anyhow!("Target environment '{}' not found", target_env))?;

    // Prepare loading message
    let loading_message = if source_entity == target_entity {
        format!("Fetching entity metadata for '{}' from both environments...", source_entity)
    } else {
        format!("Fetching entity metadata for '{}' from source and '{}' from target...", source_entity, target_entity)
    };

    // Fetch entity metadata from both environments with spinner
    let (source_fields, target_fields) = show_loading_while(
        loading_message,
        || {
            let source_auth = source_auth.clone();
            let target_auth = target_auth.clone();
            let source_entity = source_entity.clone();
            let target_entity = target_entity.clone();

            async move {
                let mut source_client = DynamicsClient::new(source_auth);
                let mut target_client = DynamicsClient::new(target_auth);

                let source_fields = fetch_entity_fields(&mut source_client, &source_entity).await?;
                let target_fields = fetch_entity_fields(&mut target_client, &target_entity).await?;
                Ok::<(Vec<FieldInfo>, Vec<FieldInfo>), anyhow::Error>((source_fields, target_fields))
            }
        }
    ).await?;

    // Create app with the data
    let mut app = CompareApp::new(
        source_fields,
        target_fields,
        source_entity,
        target_entity,
        source_env,
        target_env,
    );

    // Load existing field mappings
    app.load_field_mappings(&config);

    // Load existing prefix mappings
    app.load_prefix_mappings(&config);

    // Export to Excel
    println!("Exporting migration analysis to '{}'...", output_path);
    app.export_to_excel(&output_path)?;
    println!("Export completed successfully!");

    Ok(())
}

async fn fetch_entity_fields(client: &mut DynamicsClient, entity_name: &str) -> Result<Vec<FieldInfo>> {
    // Fetch metadata from Dynamics 365
    let metadata_xml = client.fetch_metadata().await?;

    // Parse the metadata to extract field information for the specific entity
    parse_entity_fields(&metadata_xml, entity_name)
}