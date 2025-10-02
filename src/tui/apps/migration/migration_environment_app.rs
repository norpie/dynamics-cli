use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId};
use crate::tui::widgets::list::{ListItem, ListState};
use crate::tui::widgets::{TextInputState, SelectState};
use crate::config::SavedMigration;
use ratatui::text::{Line, Span};
use ratatui::style::Style;

pub struct MigrationEnvironmentApp;

#[derive(Clone)]
pub struct MigrationEnvironment {
    name: String,
    source: String,
    target: String,
}

impl ListItem for MigrationEnvironment {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        use ratatui::text::{Line, Span};
        use ratatui::style::Style;

        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(format!("  {}", self.name), Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

pub struct State {
    environments: Vec<MigrationEnvironment>,
    list_state: ListState,
    initialized: bool,
    show_create_modal: bool,
    create_form: CreateMigrationForm,
    available_environments: Vec<String>,
    show_delete_confirm: bool,
    delete_migration_name: Option<String>,
    show_rename_modal: bool,
    rename_migration_name: Option<String>,
    rename_form: RenameMigrationForm,
    // Partial loading state for parallel entity fetches
    loading_migration_name: Option<String>,
    loading_source_env: Option<String>,
    loading_target_env: Option<String>,
    source_entities_partial: Option<Result<Vec<String>, String>>,
    target_entities_partial: Option<Result<Vec<String>, String>>,
}

#[derive(Clone, Default)]
pub struct RenameMigrationForm {
    new_name: String,
    name_input_state: TextInputState,
}

#[derive(Clone, Default)]
pub struct CreateMigrationForm {
    name: String,
    name_input_state: TextInputState,
    source_env: Option<String>,
    source_select_state: SelectState,
    target_env: Option<String>,
    target_select_state: SelectState,
    validation_error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub enum Msg {
    Initialize,
    MigrationsLoaded(Result<Vec<SavedMigration>, String>),
    SelectEnvironment(usize),
    SourceEntitiesLoaded(Result<Vec<String>, String>),
    TargetEntitiesLoaded(Result<Vec<String>, String>),
    AllDataLoaded(Result<ComparisonData, String>),
    ListNavigate(KeyCode),
    OpenCreateModal,
    EnvironmentsLoaded(Result<Vec<String>, String>),
    CreateFormNameChanged(KeyCode),
    CreateFormSourceSelected(usize),
    CreateFormSourceToggled,
    CreateFormSourceNavigate(KeyCode),
    CreateFormSourceBlurred,
    CreateFormTargetSelected(usize),
    CreateFormTargetToggled,
    CreateFormTargetNavigate(KeyCode),
    CreateFormTargetBlurred,
    CreateFormSubmit,
    CreateFormCancel,
    MigrationCreated(Result<(), String>),
    RequestDelete,
    ConfirmDelete,
    CancelDelete,
    MigrationDeleted(Result<(), String>),
    RequestRename,
    RenameFormNameChanged(KeyCode),
    RenameFormSubmit,
    RenameFormCancel,
    MigrationRenamed(Result<(), String>),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ComparisonData {
    pub migration_name: String,
    pub source_env: String,
    pub target_env: String,
    pub comparisons: Vec<crate::config::repository::migrations::SavedComparison>,
    pub source_entities: Vec<String>,
    pub target_entities: Vec<String>,
}

impl App for MigrationEnvironmentApp {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize => {
                if !state.initialized {
                    // Load migrations from database
                    Command::perform(
                        async {
                            let config = crate::config();
                            config.list_migrations().await
                                .map_err(|e| e.to_string())
                        },
                        Msg::MigrationsLoaded
                    )
                } else {
                    Command::None
                }
            }
            Msg::MigrationsLoaded(Ok(migrations)) => {
                state.environments = migrations.into_iter().map(|m| MigrationEnvironment {
                    name: m.name,
                    source: m.source_env,
                    target: m.target_env,
                }).collect();
                state.initialized = true;
                Command::set_focus(FocusId::new("migration-list"))
            }
            Msg::MigrationsLoaded(Err(err)) => {
                log::error!("Failed to load migrations: {}", err);
                state.initialized = true;
                Command::batch(vec![
                    Command::publish("error:init", serde_json::json!({
                        "message": format!("Failed to load migrations: {}", err),
                        "target": "MigrationEnvironment",
                    })),
                    Command::navigate_to(AppId::ErrorScreen),
                ])
            }
            Msg::SelectEnvironment(idx) => {
                if let Some(migration) = state.environments.get(idx) {
                    let migration_name = migration.name.clone();
                    let source_env = migration.source.clone();
                    let target_env = migration.target.clone();

                    // Store loading context in state
                    state.loading_migration_name = Some(migration_name.clone());
                    state.loading_source_env = Some(source_env.clone());
                    state.loading_target_env = Some(target_env.clone());
                    state.source_entities_partial = None;
                    state.target_entities_partial = None;

                    let source_task = format!("Loading source entities ({})", source_env);
                    let target_task = format!("Loading target entities ({})", target_env);

                    let source_env_clone = source_env.clone();
                    let target_env_clone = target_env.clone();

                    // Navigate to loading screen and start TWO separate async tasks
                    let loading_init = serde_json::json!({
                        "tasks": [&source_task, &target_task],
                        "target": "MigrationComparisonSelect",
                        "caller": "MigrationEnvironment",
                        "cancellable": false,
                        "auto_complete": false,
                    });

                    Command::batch(vec![
                        Command::publish("loading:init", loading_init),
                        Command::navigate_to(AppId::LoadingScreen),
                        Command::publish("loading:progress", serde_json::json!({
                            "task": &source_task,
                            "status": "InProgress",
                        })),
                        Command::publish("loading:progress", serde_json::json!({
                            "task": &target_task,
                            "status": "InProgress",
                        })),
                        // Task 1: Load source entities
                        Command::perform(
                            async move {
                                use crate::api::metadata::parse_entity_list;
                                let config = crate::config();
                                let manager = crate::client_manager();

                                match config.get_entity_cache(&source_env_clone, 24).await {
                                    Ok(Some(cached)) => {
                                        log::debug!("Using cached entities for source: {}", source_env_clone);
                                        Ok(cached)
                                    }
                                    _ => {
                                        log::debug!("Fetching fresh metadata for source: {}", source_env_clone);
                                        let client = manager.get_client(&source_env_clone).await.map_err(|e| e.to_string())?;
                                        let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                        let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;

                                        // Cache the entities and log result
                                        match config.set_entity_cache(&source_env_clone, entities.clone()).await {
                                            Ok(_) => log::info!("Successfully cached {} entities for {}", entities.len(), source_env_clone),
                                            Err(e) => log::error!("Failed to cache entities for {}: {}", source_env_clone, e),
                                        }

                                        Ok(entities)
                                    }
                                }
                            },
                            Msg::SourceEntitiesLoaded,
                        ),
                        // Task 2: Load target entities
                        Command::perform(
                            async move {
                                use crate::api::metadata::parse_entity_list;
                                let config = crate::config();
                                let manager = crate::client_manager();

                                match config.get_entity_cache(&target_env_clone, 24).await {
                                    Ok(Some(cached)) => {
                                        log::debug!("Using cached entities for target: {}", target_env_clone);
                                        Ok(cached)
                                    }
                                    _ => {
                                        log::debug!("Fetching fresh metadata for target: {}", target_env_clone);
                                        let client = manager.get_client(&target_env_clone).await.map_err(|e| e.to_string())?;
                                        let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                        let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;

                                        // Cache the entities and log result
                                        match config.set_entity_cache(&target_env_clone, entities.clone()).await {
                                            Ok(_) => log::info!("Successfully cached {} entities for {}", entities.len(), target_env_clone),
                                            Err(e) => log::error!("Failed to cache entities for {}: {}", target_env_clone, e),
                                        }

                                        Ok(entities)
                                    }
                                }
                            },
                            Msg::TargetEntitiesLoaded,
                        ),
                    ])
                } else {
                    Command::None
                }
            }
            Msg::SourceEntitiesLoaded(result) => {
                // Store source result and publish progress
                state.source_entities_partial = Some(result.clone());

                let source_env = state.loading_source_env.as_ref().unwrap();
                let source_task = format!("Loading source entities ({})", source_env);

                let mut commands = vec![];

                // Publish progress for this task
                match result {
                    Ok(_) => {
                        commands.push(Command::publish("loading:progress", serde_json::json!({
                            "task": source_task,
                            "status": "Completed",
                        })));
                    }
                    Err(ref e) => {
                        commands.push(Command::publish("loading:progress", serde_json::json!({
                            "task": source_task,
                            "status": "Failed",
                            "error": e.clone(),
                        })));
                    }
                }

                Command::batch(commands)
            }
            Msg::TargetEntitiesLoaded(result) => {
                // Store target result and publish progress
                state.target_entities_partial = Some(result.clone());

                let target_env = state.loading_target_env.as_ref().unwrap();
                let target_task = format!("Loading target entities ({})", target_env);

                let mut commands = vec![];

                // Publish progress for this task
                match result {
                    Ok(_) => {
                        commands.push(Command::publish("loading:progress", serde_json::json!({
                            "task": target_task,
                            "status": "Completed",
                        })));
                    }
                    Err(ref e) => {
                        commands.push(Command::publish("loading:progress", serde_json::json!({
                            "task": target_task,
                            "status": "Failed",
                            "error": e.clone(),
                        })));
                    }
                }

                // Check if both tasks are done
                if state.source_entities_partial.is_some() && state.target_entities_partial.is_some() {
                    let migration_name = state.loading_migration_name.as_ref().unwrap().clone();
                    let source_env = state.loading_source_env.as_ref().unwrap().clone();
                    let target_env = state.loading_target_env.as_ref().unwrap().clone();

                    // Both done - check results
                    let source_result = state.source_entities_partial.as_ref().unwrap().clone();
                    let target_result = state.target_entities_partial.as_ref().unwrap().clone();

                    // Clear loading state
                    state.loading_migration_name = None;
                    state.loading_source_env = None;
                    state.loading_target_env = None;
                    state.source_entities_partial = None;
                    state.target_entities_partial = None;

                    // If either failed, navigate to error screen
                    if source_result.is_err() || target_result.is_err() {
                        let error = if let Err(e) = source_result {
                            e
                        } else {
                            target_result.unwrap_err()
                        };
                        commands.push(Command::publish("error:init", serde_json::json!({
                            "message": format!("Failed to load entities: {}", error),
                            "target": "MigrationEnvironment",
                        })));
                        commands.push(Command::navigate_to(AppId::ErrorScreen));
                    } else {
                        // Both succeeded - wait a moment for UI to show both complete, then load comparisons and navigate
                        let source_entities = source_result.unwrap();
                        let target_entities = target_result.unwrap();

                        commands.push(Command::perform(
                            async move {
                                // Give UI time to render both tasks as complete
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                                let config = crate::config();
                                let comparisons = config.get_comparisons(&migration_name).await.map_err(|e| e.to_string())?;
                                Ok::<_, String>(ComparisonData {
                                    migration_name,
                                    source_env,
                                    target_env,
                                    comparisons,
                                    source_entities,
                                    target_entities,
                                })
                            },
                            Msg::AllDataLoaded,
                        ));
                    }
                }

                Command::batch(commands)
            }
            Msg::AllDataLoaded(result) => {
                match result {
                    Ok(data) => {
                        log::info!("All data loaded successfully");
                        Command::batch(vec![
                            // Publish data to comparison select app
                            Command::publish("comparison_data", serde_json::to_value(&data).unwrap()),
                            // Navigate to comparison select
                            Command::navigate_to(AppId::MigrationComparisonSelect),
                        ])
                    }
                    Err(e) => {
                        log::error!("Failed to load comparison data: {}", e);
                        Command::batch(vec![
                            // Publish error:init so ErrorScreen receives it
                            Command::publish("error:init", serde_json::json!({
                                "message": format!("Failed to load comparison data: {}", e),
                                "target": "MigrationEnvironment",
                            })),
                            // Navigate to ErrorScreen
                            Command::navigate_to(AppId::ErrorScreen),
                        ])
                    }
                }
            }
            Msg::ListNavigate(key) => {
                let visible_height = 20;
                state.list_state.handle_key(key, state.environments.len(), visible_height);
                Command::None
            }
            Msg::OpenCreateModal => {
                state.show_create_modal = true;
                state.create_form = CreateMigrationForm::default();
                // Load available environments
                Command::perform(
                    async {
                        let config = crate::config();
                        config.list_environments().await
                            .map_err(|e| e.to_string())
                    },
                    Msg::EnvironmentsLoaded
                )
            }
            Msg::EnvironmentsLoaded(Ok(envs)) => {
                state.available_environments = envs;
                Command::set_focus(FocusId::new("create-name-input"))
            }
            Msg::EnvironmentsLoaded(Err(err)) => {
                log::error!("Failed to load environments: {}", err);
                state.show_create_modal = false;
                Command::None
            }
            Msg::CreateFormNameChanged(key) => {
                if let Some(new_value) = state.create_form.name_input_state.handle_key(
                    key,
                    &state.create_form.name,
                    Some(50) // Max 50 characters
                ) {
                    state.create_form.name = new_value;
                }
                Command::None
            }
            Msg::CreateFormSourceSelected(idx) => {
                // Index 0 is the placeholder, actual environments start at index 1
                if idx == 0 {
                    state.create_form.source_env = None;
                } else {
                    let filtered_envs = get_filtered_source_envs(&state.available_environments, &state.create_form.target_env);
                    if let Some(env) = filtered_envs.get(idx - 1) {
                        state.create_form.source_env = Some(env.clone());
                    }
                }
                state.create_form.source_select_state.close();
                Command::None
            }
            Msg::CreateFormSourceToggled => {
                state.create_form.source_select_state.toggle();
                Command::None
            }
            Msg::CreateFormSourceNavigate(key) => {
                match key {
                    KeyCode::Up => state.create_form.source_select_state.navigate_prev(),
                    KeyCode::Down => state.create_form.source_select_state.navigate_next(),
                    KeyCode::Enter => {
                        state.create_form.source_select_state.select_highlighted();
                        // Update the source_env based on the selected index
                        let selected_idx = state.create_form.source_select_state.selected();
                        if selected_idx == 0 {
                            state.create_form.source_env = None;
                        } else {
                            let filtered_envs = get_filtered_source_envs(&state.available_environments, &state.create_form.target_env);
                            if let Some(env) = filtered_envs.get(selected_idx - 1) {
                                state.create_form.source_env = Some(env.clone());
                            }
                        }
                    }
                    KeyCode::Esc => state.create_form.source_select_state.close(),
                    _ => {}
                }
                Command::None
            }
            Msg::CreateFormSourceBlurred => {
                state.create_form.source_select_state.close();
                Command::None
            }
            Msg::CreateFormTargetSelected(idx) => {
                // Index 0 is the placeholder, actual environments start at index 1
                if idx == 0 {
                    state.create_form.target_env = None;
                } else {
                    let filtered_envs = get_filtered_target_envs(&state.available_environments, &state.create_form.source_env);
                    if let Some(env) = filtered_envs.get(idx - 1) {
                        state.create_form.target_env = Some(env.clone());
                    }
                }
                state.create_form.target_select_state.close();
                Command::None
            }
            Msg::CreateFormTargetToggled => {
                state.create_form.target_select_state.toggle();
                Command::None
            }
            Msg::CreateFormTargetNavigate(key) => {
                match key {
                    KeyCode::Up => state.create_form.target_select_state.navigate_prev(),
                    KeyCode::Down => state.create_form.target_select_state.navigate_next(),
                    KeyCode::Enter => {
                        state.create_form.target_select_state.select_highlighted();
                        // Update the target_env based on the selected index
                        let selected_idx = state.create_form.target_select_state.selected();
                        if selected_idx == 0 {
                            state.create_form.target_env = None;
                        } else {
                            let filtered_envs = get_filtered_target_envs(&state.available_environments, &state.create_form.source_env);
                            if let Some(env) = filtered_envs.get(selected_idx - 1) {
                                state.create_form.target_env = Some(env.clone());
                            }
                        }
                    }
                    KeyCode::Esc => state.create_form.target_select_state.close(),
                    _ => {}
                }
                Command::None
            }
            Msg::CreateFormTargetBlurred => {
                state.create_form.target_select_state.close();
                Command::None
            }
            Msg::CreateFormSubmit => {
                let name = state.create_form.name.trim().to_string();
                let source = state.create_form.source_env.clone();
                let target = state.create_form.target_env.clone();

                if name.is_empty() {
                    state.create_form.validation_error = Some("Migration name is required".to_string());
                    return Command::None;
                }

                if source.is_none() {
                    state.create_form.validation_error = Some("Source environment is required".to_string());
                    return Command::None;
                }

                if target.is_none() {
                    state.create_form.validation_error = Some("Target environment is required".to_string());
                    return Command::None;
                }

                let source = source.unwrap();
                let target = target.unwrap();

                if source == target {
                    state.create_form.validation_error = Some("Source and target environments must be different".to_string());
                    return Command::None;
                }

                state.show_create_modal = false;
                state.create_form.validation_error = None;

                Command::perform(
                    async move {
                        let config = crate::config();
                        let migration = SavedMigration {
                            name,
                            source_env: source,
                            target_env: target,
                            created_at: chrono::Utc::now(),
                            last_used: chrono::Utc::now(),
                        };
                        config.add_migration(migration).await
                            .map_err(|e| e.to_string())
                    },
                    Msg::MigrationCreated
                )
            }
            Msg::CreateFormCancel => {
                state.show_create_modal = false;
                state.create_form.validation_error = None;
                Command::None
            }
            Msg::RequestDelete => {
                // Get selected migration name
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(migration) = state.environments.get(selected_idx) {
                        state.delete_migration_name = Some(migration.name.clone());
                        state.show_delete_confirm = true;
                    }
                }
                Command::None
            }
            Msg::ConfirmDelete => {
                if let Some(name) = state.delete_migration_name.clone() {
                    state.show_delete_confirm = false;
                    // Async delete from database
                    Command::perform(
                        async move {
                            let config = crate::config();
                            config.delete_migration(&name).await.map_err(|e| e.to_string())
                        },
                        Msg::MigrationDeleted
                    )
                } else {
                    Command::None
                }
            }
            Msg::CancelDelete => {
                state.show_delete_confirm = false;
                state.delete_migration_name = None;
                Command::None
            }
            Msg::MigrationDeleted(result) => {
                match result {
                    Ok(_) => {
                        state.delete_migration_name = None;
                        // Reload migration list
                        Command::perform(
                            async {
                                let config = crate::config();
                                config.list_migrations().await.map_err(|e| e.to_string())
                            },
                            Msg::MigrationsLoaded
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to delete migration: {}", e);
                        Command::None
                    }
                }
            }
            Msg::RequestRename => {
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(migration) = state.environments.get(selected_idx) {
                        state.rename_migration_name = Some(migration.name.clone());
                        state.rename_form.new_name = migration.name.clone();  // Pre-fill with current name
                        state.show_rename_modal = true;
                    }
                }
                Command::None
            }
            Msg::RenameFormNameChanged(key) => {
                if let Some(new_value) = state.rename_form.name_input_state.handle_key(
                    key,
                    &state.rename_form.new_name,
                    Some(50)
                ) {
                    state.rename_form.new_name = new_value;
                }
                Command::None
            }
            Msg::RenameFormSubmit => {
                let old_name = state.rename_migration_name.clone();
                let new_name = state.rename_form.new_name.trim().to_string();

                if new_name.is_empty() || old_name.is_none() {
                    return Command::None;
                }

                state.show_rename_modal = false;
                let old_name = old_name.unwrap();

                Command::perform(
                    async move {
                        let config = crate::config();
                        // Get existing migration
                        let migration = config.get_migration(&old_name).await
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| "Migration not found".to_string())?;

                        // Delete old
                        config.delete_migration(&old_name).await
                            .map_err(|e| e.to_string())?;

                        // Insert with new name
                        let renamed = SavedMigration {
                            name: new_name,
                            source_env: migration.source_env,
                            target_env: migration.target_env,
                            created_at: migration.created_at,
                            last_used: chrono::Utc::now(),
                        };
                        config.add_migration(renamed).await
                            .map_err(|e| e.to_string())
                    },
                    Msg::MigrationRenamed
                )
            }
            Msg::RenameFormCancel => {
                state.show_rename_modal = false;
                state.rename_migration_name = None;
                state.rename_form = RenameMigrationForm::default();
                Command::None
            }
            Msg::MigrationRenamed(result) => {
                match result {
                    Ok(_) => {
                        state.rename_migration_name = None;
                        state.rename_form = RenameMigrationForm::default();
                        // Reload list
                        Command::perform(
                            async {
                                let config = crate::config();
                                config.list_migrations().await.map_err(|e| e.to_string())
                            },
                            Msg::MigrationsLoaded
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to rename migration: {}", e);
                        Command::None
                    }
                }
            }
            Msg::MigrationCreated(Ok(())) => {
                // Reload migrations list
                Command::perform(
                    async {
                        let config = crate::config();
                        config.list_migrations().await
                            .map_err(|e| e.to_string())
                    },
                    Msg::MigrationsLoaded
                )
            }
            Msg::MigrationCreated(Err(err)) => {
                log::error!("Failed to create migration: {}", err);
                state.show_create_modal = false;
                Command::batch(vec![
                    Command::publish("error:init", serde_json::json!({
                        "message": format!("Failed to create migration: {}", err),
                        "target": "MigrationEnvironment",
                    })),
                    Command::navigate_to(AppId::ErrorScreen),
                ])
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let list = Element::list(FocusId::new("migration-list"), &state.environments, &state.list_state, theme)
            .on_activate(Msg::SelectEnvironment)
            .on_navigate(Msg::ListNavigate)
            .build();

        let main_ui = Element::panel(list)
            .title("Select Migration Environment")
            .build();

        if state.show_delete_confirm {
            // Render delete confirmation modal
            let migration_name = state.delete_migration_name.as_deref().unwrap_or("Unknown");

            Element::modal_confirm(
                main_ui,
                "Delete Migration",
                format!("Delete migration '{}'?", migration_name),
                FocusId::new("delete-cancel"),
                Msg::CancelDelete,
                FocusId::new("delete-confirm"),
                Msg::ConfirmDelete,
            )
        } else if state.show_rename_modal {
            use crate::tui::element::{ColumnBuilder, RowBuilder};
            use crate::tui::{LayoutConstraint, Layer};

            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    FocusId::new("rename-name-input"),
                    &state.rename_form.new_name,
                    &state.rename_form.name_input_state
                )
                .placeholder("Migration name")
                .on_change(Msg::RenameFormNameChanged)
                .build()
            )
            .title("New Name")
            .build();

            // Buttons
            let buttons = RowBuilder::new()
                .add(
                    Element::button(FocusId::new("rename-cancel"), "Cancel")
                        .on_press(Msg::RenameFormCancel)
                        .build(),
                    LayoutConstraint::Fill(1),
                )
                .add(Element::text("  "), LayoutConstraint::Length(2))
                .add(
                    Element::button(FocusId::new("rename-confirm"), "Rename")
                        .on_press(Msg::RenameFormSubmit)
                        .build(),
                    LayoutConstraint::Fill(1),
                )
                .spacing(0)
                .build();

            // Modal content
            let modal_content = Element::panel(
                Element::container(
                    ColumnBuilder::new()
                        .add(name_input, LayoutConstraint::Length(3))
                        .add(Element::text(""), LayoutConstraint::Length(1))
                        .add(buttons, LayoutConstraint::Length(3))
                        .spacing(0)
                        .build()
                )
                .padding(2)
                .build()
            )
            .title("Rename Migration")
            .width(60)
            .height(13)
            .build();

            Element::stack(vec![
                Layer::new(main_ui).dim(true),
                Layer::new(modal_content).center(),
            ])
        } else if state.show_create_modal {
            use crate::tui::element::{ColumnBuilder, RowBuilder, Alignment};
            use crate::tui::LayoutConstraint;

            // If environments haven't loaded yet, show loading message
            if state.available_environments.is_empty() {
                let loading_content = Element::panel(
                    Element::container(
                        Element::column(vec![
                            Element::text("Loading environments..."),
                        ]).build()
                    )
                    .padding(2)
                    .build()
                )
                .title("Create New Migration")
                .build();

                return Element::stack(vec![
                    crate::tui::Layer {
                        element: main_ui,
                        alignment: Alignment::Center,
                        dim_below: false,
                    },
                    crate::tui::Layer {
                        element: loading_content,
                        alignment: Alignment::Center,
                        dim_below: true,
                    },
                ]);
            }

            // Get filtered environment options
            let mut source_options = vec!["(Select source environment)".to_string()];
            source_options.extend(get_filtered_source_envs(&state.available_environments, &state.create_form.target_env));

            let mut target_options = vec!["(Select target environment)".to_string()];
            target_options.extend(get_filtered_target_envs(&state.available_environments, &state.create_form.source_env));

            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    FocusId::new("create-name-input"),
                    &state.create_form.name,
                    &state.create_form.name_input_state
                )
                .placeholder("Migration name")
                .on_change(Msg::CreateFormNameChanged)
                .build()
            )
            .title("Name")
            .build();

            // Source environment select
            let source_select = Element::panel(
                Element::select(
                    FocusId::new("create-source-select"),
                    source_options,
                    &mut state.create_form.source_select_state
                )
                .on_select(Msg::CreateFormSourceSelected)
                .on_toggle(Msg::CreateFormSourceToggled)
                .on_navigate(Msg::CreateFormSourceNavigate)
                .on_blur(Msg::CreateFormSourceBlurred)
                .build()
            )
            .title("Source Environment")
            .build();

            // Target environment select
            let target_select = Element::panel(
                Element::select(
                    FocusId::new("create-target-select"),
                    target_options,
                    &mut state.create_form.target_select_state
                )
                .on_select(Msg::CreateFormTargetSelected)
                .on_toggle(Msg::CreateFormTargetToggled)
                .on_navigate(Msg::CreateFormTargetNavigate)
                .on_blur(Msg::CreateFormTargetBlurred)
                .build()
            )
            .title("Target Environment")
            .build();

            // Validation error display
            let error_section = if let Some(ref error) = state.create_form.validation_error {
                ColumnBuilder::new()
                    .add(
                        Element::styled_text(Line::from(vec![
                            Span::styled(format!("⚠ {}", error), Style::default().fg(theme.red))
                        ])).build(),
                        LayoutConstraint::Length(1)
                    )
                    .add(Element::text(""), LayoutConstraint::Length(1))
                    .spacing(0)
                    .build()
            } else {
                Element::text("")
            };

            // Buttons
            let buttons = RowBuilder::new()
                .add(
                    Element::button(FocusId::new("create-cancel"), "Cancel")
                        .on_press(Msg::CreateFormCancel)
                        .build(),
                    LayoutConstraint::Fill(1),
                )
                .add(Element::text("  "), LayoutConstraint::Length(2))
                .add(
                    Element::button(FocusId::new("create-confirm"), "Confirm")
                        .on_press(Msg::CreateFormSubmit)
                        .build(),
                    LayoutConstraint::Fill(1),
                )
                .spacing(0)
                .build();

            // Modal content - use explicit sizing for proper display
            let mut modal_builder = ColumnBuilder::new()
                .add(name_input, LayoutConstraint::Length(3))
                .add(Element::text(""), LayoutConstraint::Length(1))
                .add(source_select, LayoutConstraint::Length(10))
                .add(Element::text(""), LayoutConstraint::Length(1))
                .add(target_select, LayoutConstraint::Length(10))
                .add(Element::text(""), LayoutConstraint::Length(1));

            if state.create_form.validation_error.is_some() {
                modal_builder = modal_builder.add(error_section, LayoutConstraint::Length(2));
            }

            modal_builder = modal_builder
                .add(buttons, LayoutConstraint::Length(3))
                .spacing(0);

            let modal_content = Element::panel(
                Element::container(modal_builder.build())
                .padding(2)
                .build()
            )
            .title("Create New Migration")
            .width(80)
            .height(if state.create_form.validation_error.is_some() { 37 } else { 35 })
            .build();

            Element::stack(vec![
                crate::tui::Layer {
                    element: main_ui,
                    alignment: Alignment::TopLeft,
                    dim_below: false,
                },
                crate::tui::Layer {
                    element: modal_content,
                    alignment: Alignment::Center,
                    dim_below: true,
                },
            ])
        } else {
            main_ui
        }
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        let mut subs = vec![];

        if !state.initialized {
            subs.push(Subscription::timer(std::time::Duration::from_millis(1), Msg::Initialize));
        }

        if !state.show_create_modal && !state.show_delete_confirm && !state.show_rename_modal {
            subs.push(Subscription::keyboard(KeyCode::Char('n'), "Create new migration", Msg::OpenCreateModal));
            subs.push(Subscription::keyboard(KeyCode::Char('N'), "Create new migration", Msg::OpenCreateModal));
            subs.push(Subscription::keyboard(KeyCode::Char('d'), "Delete migration", Msg::RequestDelete));
            subs.push(Subscription::keyboard(KeyCode::Char('D'), "Delete migration", Msg::RequestDelete));
            subs.push(Subscription::keyboard(KeyCode::Char('r'), "Rename migration", Msg::RequestRename));
            subs.push(Subscription::keyboard(KeyCode::Char('R'), "Rename migration", Msg::RequestRename));
        }

        subs
    }

    fn title() -> &'static str {
        "Migration Environments"
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            environments: vec![],
            list_state: ListState::with_selection(),
            initialized: false,
            show_create_modal: false,
            create_form: CreateMigrationForm::default(),
            available_environments: vec![],
            show_delete_confirm: false,
            delete_migration_name: None,
            show_rename_modal: false,
            rename_migration_name: None,
            rename_form: RenameMigrationForm::default(),
            loading_migration_name: None,
            loading_source_env: None,
            loading_target_env: None,
            source_entities_partial: None,
            target_entities_partial: None,
        }
    }
}

fn get_filtered_source_envs(all_envs: &[String], exclude: &Option<String>) -> Vec<String> {
    all_envs.iter()
        .filter(|e| {
            if let Some(target) = exclude {
                *e != target
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

fn get_filtered_target_envs(all_envs: &[String], exclude: &Option<String>) -> Vec<String> {
    all_envs.iter()
        .filter(|e| {
            if let Some(source) = exclude {
                *e != source
            } else {
                true
            }
        })
        .cloned()
        .collect()
}
