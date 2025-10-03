use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId, Resource};
use crate::tui::widgets::list::{ListItem, ListState};
use crate::tui::widgets::{TextInputState, SelectState, SelectEvent};
use crate::config::SavedMigration;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use crate::{col, row, spacer, button_row, modal, use_constraints};

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
    show_create_modal: bool,
    create_form: CreateMigrationForm,
    available_environments: Vec<String>,
    show_delete_confirm: bool,
    delete_migration_name: Option<String>,
    show_rename_modal: bool,
    rename_migration_name: Option<String>,
    rename_form: RenameMigrationForm,
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
    MigrationsLoaded(Result<Vec<SavedMigration>, String>),
    SelectEnvironment(usize),
    ListNavigate(KeyCode),
    OpenCreateModal,
    EnvironmentsLoaded(Result<Vec<String>, String>),
    CreateFormNameChanged(KeyCode),
    CreateFormSourceEvent(SelectEvent),
    CreateFormTargetEvent(SelectEvent),
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

impl crate::tui::AppState for State {}

impl App for MigrationEnvironmentApp {
    type State = State;
    type Msg = Msg;

    fn init() -> (State, Command<Msg>) {
        let state = State::default();
        let cmd = Command::perform(
            async {
                let config = crate::config();
                config.list_migrations().await
                    .map_err(|e| e.to_string())
            },
            Msg::MigrationsLoaded
        );
        (state, cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::MigrationsLoaded(Ok(migrations)) => {
                state.environments = migrations.into_iter().map(|m| MigrationEnvironment {
                    name: m.name,
                    source: m.source_env,
                    target: m.target_env,
                }).collect();
                Command::set_focus(FocusId::new("migration-list"))
            }
            Msg::MigrationsLoaded(Err(err)) => {
                log::error!("Failed to load migrations: {}", err);
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
                    // Publish metadata first - comparison app will receive it and start loading
                    Command::batch(vec![
                        Command::navigate_to(AppId::MigrationComparisonSelect),
                        Command::publish("migration:selected", serde_json::json!({
                            "migration_name": migration.name,
                            "source_env": migration.source,
                            "target_env": migration.target,
                        })),
                    ])
                } else {
                    Command::None
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
            Msg::CreateFormSourceEvent(event) => {
                use SelectEvent::*;
                match event {
                    Navigate(key) => {
                        state.create_form.source_select_state.handle_event(event);
                    }
                    Select(idx) => {
                        state.create_form.source_select_state.handle_event(event);
                        // Index 0 is the placeholder, actual environments start at index 1
                        if idx == 0 {
                            state.create_form.source_env = None;
                        } else {
                            let filtered_envs = get_filtered_source_envs(&state.available_environments, &state.create_form.target_env);
                            if let Some(env) = filtered_envs.get(idx - 1) {
                                state.create_form.source_env = Some(env.clone());
                            }
                        }
                    }
                }
                Command::None
            }
            Msg::CreateFormTargetEvent(event) => {
                use SelectEvent::*;
                match event {
                    Navigate(key) => {
                        state.create_form.target_select_state.handle_event(event);
                    }
                    Select(idx) => {
                        state.create_form.target_select_state.handle_event(event);
                        // Index 0 is the placeholder, actual environments start at index 1
                        if idx == 0 {
                            state.create_form.target_env = None;
                        } else {
                            let filtered_envs = get_filtered_target_envs(&state.available_environments, &state.create_form.source_env);
                            if let Some(env) = filtered_envs.get(idx - 1) {
                                state.create_form.target_env = Some(env.clone());
                            }
                        }
                    }
                }
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
        use_constraints!();

        let list = Element::list("migration-list", &state.environments, &state.list_state, theme)
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
                "delete-cancel",
                Msg::CancelDelete,
                "delete-confirm",
                Msg::ConfirmDelete,
            )
        } else if state.show_rename_modal {
            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    "rename-name-input",
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
            let buttons = button_row![
                ("rename-cancel", "Cancel", Msg::RenameFormCancel),
                ("rename-confirm", "Rename", Msg::RenameFormSubmit),
            ];

            // Modal content
            let modal_content = Element::panel(
                Element::container(
                    col![
                        name_input => Length(3),
                        spacer!() => Length(1),
                        buttons => Length(3),
                    ]
                )
                .padding(2)
                .build()
            )
            .title("Rename Migration")
            .width(60)
            .height(13)
            .build();

            modal!(main_ui, modal_content)
        } else if state.show_create_modal {
            use crate::tui::element::Alignment;

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

                return modal!(main_ui, loading_content, Alignment::Center);
            }

            // Get filtered environment options
            let mut source_options = vec!["(Select source environment)".to_string()];
            source_options.extend(get_filtered_source_envs(&state.available_environments, &state.create_form.target_env));

            let mut target_options = vec!["(Select target environment)".to_string()];
            target_options.extend(get_filtered_target_envs(&state.available_environments, &state.create_form.source_env));

            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    "create-name-input",
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
                    "create-source-select",
                    source_options,
                    &mut state.create_form.source_select_state
                )
                .on_event(Msg::CreateFormSourceEvent)
                .build()
            )
            .title("Source Environment")
            .build();

            // Target environment select
            let target_select = Element::panel(
                Element::select(
                    "create-target-select",
                    target_options,
                    &mut state.create_form.target_select_state
                )
                .on_event(Msg::CreateFormTargetEvent)
                .build()
            )
            .title("Target Environment")
            .build();

            // Buttons
            let buttons = button_row![
                ("create-cancel", "Cancel", Msg::CreateFormCancel),
                ("create-confirm", "Confirm", Msg::CreateFormSubmit),
            ];

            // Modal content - use explicit sizing for proper display
            let modal_body = if state.create_form.validation_error.is_some() {
                col![
                    name_input => Length(3),
                    spacer!() => Length(1),
                    source_select => Length(10),
                    spacer!() => Length(1),
                    target_select => Length(10),
                    spacer!() => Length(1),
                    crate::error_display!(state.create_form.validation_error, theme) => Length(2),
                    buttons => Length(3),
                ]
            } else {
                col![
                    name_input => Length(3),
                    spacer!() => Length(1),
                    source_select => Length(10),
                    spacer!() => Length(1),
                    target_select => Length(10),
                    spacer!() => Length(1),
                    buttons => Length(3),
                ]
            };

            let modal_content = Element::panel(
                Element::container(modal_body)
                .padding(2)
                .build()
            )
            .title("Create New Migration")
            .width(80)
            .height(if state.create_form.validation_error.is_some() { 37 } else { 35 })
            .build();

            modal!(main_ui, modal_content)
        } else {
            main_ui
        }
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        let mut subs = vec![];

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
            show_create_modal: false,
            create_form: CreateMigrationForm::default(),
            available_environments: vec![],
            show_delete_confirm: false,
            delete_migration_name: None,
            show_rename_modal: false,
            rename_migration_name: None,
            rename_form: RenameMigrationForm::default(),
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
