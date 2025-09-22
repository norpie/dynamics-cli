use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{DisableMouseCapture, EnableMouseCapture},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use crate::config::{Config, SavedMigration, ComparisonType};
use crate::dynamics::client::DynamicsClient;
use crate::dynamics::metadata::{parse_entity_fields, FieldInfo};
use super::app::CompareApp;
use super::menus::{MigrationSelectAction, EnvironmentSelectAction, ComparisonSelectAction, ViewSelectAction};
use super::spinner::show_loading_while_with_terminal;

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationLevel {
    MigrationSelect,
    EnvironmentSelect,
    ComparisonSelect,
    FieldCompare,
    ViewCompare,
}


pub struct NavigationState {
    pub current_level: NavigationLevel,
    pub selected_migration: Option<SavedMigration>,
    pub temp_source_env: Option<String>,
    pub temp_target_env: Option<String>,
    pub temp_source_entity: Option<String>,
    pub temp_target_entity: Option<String>,
}

impl NavigationState {
    pub fn new() -> Self {
        Self {
            current_level: NavigationLevel::MigrationSelect,
            selected_migration: None,
            temp_source_env: None,
            temp_target_env: None,
            temp_source_entity: None,
            temp_target_entity: None,
        }
    }

    pub fn go_back(&mut self) {
        match self.current_level {
            NavigationLevel::FieldCompare => {
                self.current_level = NavigationLevel::ComparisonSelect;
            }
            NavigationLevel::ViewCompare => {
                self.current_level = NavigationLevel::ComparisonSelect;
            }
            NavigationLevel::ComparisonSelect => {
                // Skip EnvironmentSelect and go directly to MigrationSelect
                self.current_level = NavigationLevel::MigrationSelect;
                self.selected_migration = None;
                self.temp_source_env = None;
                self.temp_target_env = None;
            }
            NavigationLevel::EnvironmentSelect => {
                self.current_level = NavigationLevel::MigrationSelect;
                self.temp_source_env = None;
                self.temp_target_env = None;
            }
            NavigationLevel::MigrationSelect => {
                // Exit the application
            }
        }
    }

    pub fn advance_to_environment_select(&mut self) {
        self.current_level = NavigationLevel::EnvironmentSelect;
    }

    pub fn advance_to_view_select(&mut self, source_env: String, target_env: String) {
        self.temp_source_env = Some(source_env.clone());
        self.temp_target_env = Some(target_env.clone());

        // Create a temporary migration config
        self.selected_migration = Some(SavedMigration {
            name: format!("{} → {}", source_env, target_env),
            source_env,
            target_env,
            comparisons: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_used: chrono::Utc::now().to_rfc3339(),
        });

        self.current_level = NavigationLevel::ComparisonSelect;
    }

    pub fn advance_to_field_compare(&mut self, source_entity: String, target_entity: String) {
        self.temp_source_entity = Some(source_entity);
        self.temp_target_entity = Some(target_entity);
        self.current_level = NavigationLevel::FieldCompare;
    }

    pub fn advance_to_view_compare(&mut self, source_entity: String, target_entity: String) {
        self.temp_source_entity = Some(source_entity);
        self.temp_target_entity = Some(target_entity);
        self.current_level = NavigationLevel::ViewCompare;
    }

    pub fn get_current_migration(&self) -> Option<&SavedMigration> {
        self.selected_migration.as_ref()
    }
}

pub async fn start_navigation() -> Result<()> {
    let mut nav_state = NavigationState::new();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_navigation_loop(&mut terminal, &mut nav_state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_navigation_loop<B>(
    terminal: &mut Terminal<B>,
    nav_state: &mut NavigationState
) -> Result<()>
where
    B: ratatui::backend::Backend,
{
    loop {
        match nav_state.current_level {
            NavigationLevel::MigrationSelect => {
                let action = run_migration_select_menu(terminal).await?;
                match action {
                    MigrationSelectAction::CreateNew => {
                        nav_state.advance_to_environment_select();
                    }
                    MigrationSelectAction::Exit => break,
                    MigrationSelectAction::LoadExisting(migration_name) => {
                        // Load the existing migration
                        let config = Config::load()?;
                        if let Some(migration) = config.get_migration(&migration_name) {
                            nav_state.selected_migration = Some(migration.clone());
                            nav_state.current_level = NavigationLevel::ComparisonSelect;
                        } else {
                            // Migration not found, fallback to creating new
                            nav_state.advance_to_environment_select();
                        }
                    }
                }
            }
            NavigationLevel::EnvironmentSelect => {
                let action = run_environment_select_menu(terminal).await?;
                match action {
                    EnvironmentSelectAction::Selected { source, target } => {
                        nav_state.advance_to_view_select(source, target);
                    }
                    EnvironmentSelectAction::Back => {
                        nav_state.go_back();
                    }
                }
            }
            NavigationLevel::ComparisonSelect => {
                let migration = nav_state.get_current_migration().unwrap();
                let action = run_comparison_select_menu(terminal, migration).await?;
                match action {
                    ComparisonSelectAction::CreateNew { source_entity, target_entity, comparison_type } => {
                        match comparison_type {
                            ComparisonType::Entity => {
                                nav_state.advance_to_field_compare(source_entity, target_entity);
                            }
                            ComparisonType::View => {
                                nav_state.advance_to_view_compare(source_entity, target_entity);
                            }
                        }
                    }
                    ComparisonSelectAction::OpenExisting { source_entity, target_entity, comparison_type } => {
                        match comparison_type {
                            ComparisonType::Entity => {
                                nav_state.advance_to_field_compare(source_entity, target_entity);
                            }
                            ComparisonType::View => {
                                nav_state.advance_to_view_compare(source_entity, target_entity);
                            }
                        }
                    }
                    ComparisonSelectAction::Back => {
                        nav_state.go_back();
                    }
                }
            }
            NavigationLevel::FieldCompare => {
                let migration = nav_state.get_current_migration().unwrap();
                let source_entity = nav_state.temp_source_entity.as_ref().unwrap();
                let target_entity = nav_state.temp_target_entity.as_ref().unwrap();

                let action = run_field_compare(
                    terminal,
                    &migration.source_env,
                    &migration.target_env,
                    source_entity,
                    target_entity
                ).await?;

                match action {
                    FieldCompareAction::Back => {
                        nav_state.go_back();
                    }
                }
            }
            NavigationLevel::ViewCompare => {
                let migration = nav_state.get_current_migration().unwrap();
                let source_entity = nav_state.temp_source_entity.as_ref().unwrap();
                let target_entity = nav_state.temp_target_entity.as_ref().unwrap();

                let action = run_view_compare(
                    terminal,
                    &migration.source_env,
                    &migration.target_env,
                    source_entity,
                    target_entity
                ).await?;

                match action {
                    ViewCompareAction::Back => {
                        nav_state.go_back();
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
enum FieldCompareAction {
    Back,
}

#[derive(Debug)]
enum ViewCompareAction {
    Back,
}

use super::menus::{MigrationSelectMenu, EnvironmentSelectMenu, ComparisonSelectMenu, ViewSelectMenu};

async fn run_migration_select_menu<B>(terminal: &mut Terminal<B>) -> Result<MigrationSelectAction>
where
    B: ratatui::backend::Backend,
{
    let mut menu = MigrationSelectMenu::new()?;
    menu.run(terminal).await
}

async fn run_environment_select_menu<B>(terminal: &mut Terminal<B>) -> Result<EnvironmentSelectAction>
where
    B: ratatui::backend::Backend,
{
    let mut menu = EnvironmentSelectMenu::new()?;
    menu.run(terminal).await
}

async fn run_comparison_select_menu<B>(terminal: &mut Terminal<B>, migration: &SavedMigration) -> Result<ComparisonSelectAction>
where
    B: ratatui::backend::Backend,
{
    let mut menu = ComparisonSelectMenu::new(migration.clone())?;
    menu.run(terminal).await
}

async fn run_field_compare<B>(
    terminal: &mut Terminal<B>,
    source_env: &str,
    target_env: &str,
    source_entity: &str,
    target_entity: &str,
) -> Result<FieldCompareAction>
where
    B: ratatui::backend::Backend,
{
    // Load configuration
    let config = Config::load()?;

    // Validate environments exist
    if !config.environments.contains_key(source_env) {
        anyhow::bail!("Source environment '{}' not found", source_env);
    }
    if !config.environments.contains_key(target_env) {
        anyhow::bail!("Target environment '{}' not found", target_env);
    }

    // Get authentication configs for both environments
    let source_auth = config.environments.get(source_env)
        .ok_or_else(|| anyhow::anyhow!("Source environment '{}' not found", source_env))?;
    let target_auth = config.environments.get(target_env)
        .ok_or_else(|| anyhow::anyhow!("Target environment '{}' not found", target_env))?;

    // Prepare loading message
    let loading_message = if source_entity == target_entity {
        format!("Fetching entity metadata for '{}' from both environments...", source_entity)
    } else {
        format!("Fetching entity metadata for '{}' from source and '{}' from target...", source_entity, target_entity)
    };

    // Fetch entity metadata from both environments with spinner
    let (source_fields, target_fields) = show_loading_while_with_terminal(
        terminal,
        loading_message,
        || {
            let source_auth = source_auth.clone();
            let target_auth = target_auth.clone();
            let source_entity = source_entity.to_string();
            let target_entity = target_entity.to_string();

            async move {
                let mut source_client = DynamicsClient::new(source_auth);
                let mut target_client = DynamicsClient::new(target_auth);

                let source_fields = fetch_entity_fields(&mut source_client, &source_entity).await?;
                let target_fields = fetch_entity_fields(&mut target_client, &target_entity).await?;
                Ok::<(Vec<FieldInfo>, Vec<FieldInfo>), anyhow::Error>((source_fields, target_fields))
            }
        }
    ).await?;

    // Create app and run it (this is the existing compare functionality)
    let mut app = CompareApp::new(
        source_fields,
        target_fields,
        source_entity.to_string(),
        target_entity.to_string(),
        source_env.to_string(),
        target_env.to_string(),
    );

    // Load existing field mappings
    app.load_field_mappings(&config);

    // Load existing prefix mappings
    app.load_prefix_mappings(&config);

    // Load available comparisons for copying mappings
    app.load_available_comparisons(&config);

    // Run the comparison TUI
    app.run(terminal)?;

    Ok(FieldCompareAction::Back)
}

async fn fetch_entity_fields(client: &mut DynamicsClient, entity_name: &str) -> Result<Vec<FieldInfo>> {
    // Fetch metadata from Dynamics 365
    let metadata_xml = client.fetch_metadata().await?;

    // Parse the metadata to extract field information for the specific entity
    parse_entity_fields(&metadata_xml, entity_name)
}

async fn run_view_compare<B>(
    terminal: &mut Terminal<B>,
    source_env: &str,
    target_env: &str,
    source_entity: &str,
    target_entity: &str,
) -> Result<ViewCompareAction>
where
    B: ratatui::backend::Backend,
{
    // Load configuration
    let config = Config::load()?;

    // Validate environments exist
    if !config.environments.contains_key(source_env) {
        anyhow::bail!("Source environment '{}' not found", source_env);
    }
    if !config.environments.contains_key(target_env) {
        anyhow::bail!("Target environment '{}' not found", target_env);
    }

    // Get authentication configs for both environments
    let source_auth = config.environments.get(source_env)
        .ok_or_else(|| anyhow::anyhow!("Source environment '{}' not found", source_env))?;
    let target_auth = config.environments.get(target_env)
        .ok_or_else(|| anyhow::anyhow!("Target environment '{}' not found", target_env))?;

    // Prepare loading message
    let loading_message = if source_entity == target_entity {
        format!("Fetching views for '{}' from both environments...", source_entity)
    } else {
        format!("Fetching views for '{}' from source and '{}' from target...", source_entity, target_entity)
    };

    // Fetch views from both environments with spinner
    let (source_views, target_views) = show_loading_while_with_terminal(
        terminal,
        loading_message,
        || {
            let source_auth = source_auth.clone();
            let target_auth = target_auth.clone();
            let source_entity = source_entity.to_string();
            let target_entity = target_entity.to_string();

            async move {
                let mut source_client = DynamicsClient::new(source_auth);
                let mut target_client = DynamicsClient::new(target_auth);

                let source_views = fetch_entity_views(&mut source_client, &source_entity).await?;
                let target_views = fetch_entity_views(&mut target_client, &target_entity).await?;
                Ok::<(Vec<crate::dynamics::metadata::ViewInfo>, Vec<crate::dynamics::metadata::ViewInfo>), anyhow::Error>((source_views, target_views))
            }
        }
    ).await?;

    // Main loop for view selection and comparison
    loop {
        // Let user select which views to compare
        let mut view_menu = ViewSelectMenu::new(source_views.clone(), target_views.clone());
        let view_action = view_menu.run(terminal).await?;

        let (source_view, target_view) = match view_action {
            ViewSelectAction::Selected { source_view, target_view } => (source_view, target_view),
            ViewSelectAction::Back => return Ok(ViewCompareAction::Back),
        };

        // Create view comparison app and run it
        let mut app = super::app::ViewCompareApp::new(
            source_view,
            target_view,
            source_env.to_string(),
            target_env.to_string(),
        )?;

        // Run the view comparison TUI
        let comparison_result = app.run(terminal)?;

        // Handle the result from the view comparison
        match comparison_result {
            // If user wants to go back to view selection, continue the loop
            super::app::ViewCompareResult::BackToViewSelection => continue,
            // If user wants to exit completely, break the loop
            super::app::ViewCompareResult::Exit => return Ok(ViewCompareAction::Back),
        }
    }
}

async fn fetch_entity_views(client: &mut DynamicsClient, entity_name: &str) -> Result<Vec<crate::dynamics::metadata::ViewInfo>> {
    // Fetch views from Dynamics 365 for the specific entity
    client.fetch_views(Some(entity_name)).await
}