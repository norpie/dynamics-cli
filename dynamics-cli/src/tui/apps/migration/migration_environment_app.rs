use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId, Resource};
use crate::tui::widgets::list::{ListItem, ListState};
use crate::tui::widgets::{TextInputField, SelectField, TextInputEvent, SelectEvent};
use crate::config::SavedMigration;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use crate::{col, row, spacer, button_row, modal, use_constraints};
use dynamics_lib_macros::Validate;

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

#[derive(Clone, Default, Validate)]
pub struct RenameMigrationForm {
    #[validate(not_empty, message = "Migration name is required")]
    new_name: TextInputField,
}

#[derive(Clone, Default, Validate)]
pub struct CreateMigrationForm {
    #[validate(not_empty, message = "Migration name is required")]
    name: TextInputField,

    #[validate(required, message = "Source environment is required")]
    source: SelectField,

    #[validate(required, custom = "validate_target_different", message = "Target must differ from source")]
    target: SelectField,

    validation_error: Option<String>,
}

impl CreateMigrationForm {
    fn validate_target_different(&self) -> Result<(), String> {
        if self.source.value() == self.target.value() {
            Err("Source and target environments must be different".to_string())
        } else {
            Ok(())
        }
    }
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
    CreateFormNameEvent(TextInputEvent),
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
    RenameFormNameEvent(TextInputEvent),
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
            Msg::CreateFormNameEvent(event) => {
                state.create_form.name.handle_event(event, Some(50));
                Command::None
            }
            Msg::CreateFormSourceEvent(event) => {
                let filtered_envs = get_filtered_source_envs(&state.available_environments, state.create_form.target.value());
                state.create_form.source.handle_event::<Msg>(event, &filtered_envs);
                Command::None
            }
            Msg::CreateFormTargetEvent(event) => {
                let filtered_envs = get_filtered_target_envs(&state.available_environments, state.create_form.source.value());
                state.create_form.target.handle_event::<Msg>(event, &filtered_envs);
                Command::None
            }
            Msg::CreateFormSubmit => {
                // Validate using generated macro method
                match state.create_form.validate() {
                    Ok(_) => {
                        let name = state.create_form.name.value().trim().to_string();
                        let source = state.create_form.source.value().unwrap().to_string();
                        let target = state.create_form.target.value().unwrap().to_string();

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
                    Err(validation_error) => {
                        state.create_form.validation_error = Some(validation_error);
                        Command::None
                    }
                }
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
                        state.rename_form.new_name.set_value(migration.name.clone());  // Pre-fill with current name
                        state.show_rename_modal = true;
                    }
                }
                Command::None
            }
            Msg::RenameFormNameEvent(event) => {
                state.rename_form.new_name.handle_event(event, Some(50));
                Command::None
            }
            Msg::RenameFormSubmit => {
                let old_name = state.rename_migration_name.clone();
                let new_name = state.rename_form.new_name.value().trim().to_string();

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
                    state.rename_form.new_name.value(),
                    &state.rename_form.new_name.state
                )
                .placeholder("Migration name")
                .on_event(Msg::RenameFormNameEvent)
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
            let source_options = get_filtered_source_envs(&state.available_environments, state.create_form.target.value());
            let target_options = get_filtered_target_envs(&state.available_environments, state.create_form.source.value());

            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    "create-name-input",
                    state.create_form.name.value(),
                    &state.create_form.name.state
                )
                .placeholder("Migration name")
                .on_event(Msg::CreateFormNameEvent)
                .build()
            )
            .title("Name")
            .build();

            // Source environment select
            let source_select = Element::panel(
                Element::select(
                    "create-source-select",
                    source_options,
                    &mut state.create_form.source.state
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
                    &mut state.create_form.target.state
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

fn get_filtered_source_envs(all_envs: &[String], exclude: Option<&str>) -> Vec<String> {
    all_envs.iter()
        .filter(|e| {
            if let Some(target) = exclude {
                e.as_str() != target
            } else {
                true
            }
        })
        .cloned()
        .collect()
}

fn get_filtered_target_envs(all_envs: &[String], exclude: Option<&str>) -> Vec<String> {
    all_envs.iter()
        .filter(|e| {
            if let Some(source) = exclude {
                e.as_str() != source
            } else {
                true
            }
        })
        .cloned()
        .collect()
}
