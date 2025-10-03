use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{ColumnBuilder, Element, FocusId, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    widgets::list::{ListItem, ListState},
    widgets::{AutocompleteField, AutocompleteEvent},
    Resource,
};
use crate::config::repository::migrations::SavedComparison;
use crossterm::event::KeyCode;
use ratatui::{
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};
use crate::{col, row, spacer, button_row, modal, use_constraints, error_display};

pub struct MigrationComparisonSelectApp;

#[derive(Clone, Default)]
pub struct CreateComparisonForm {
    name: String,
    name_input_state: crate::tui::widgets::TextInputState,
    source_entity_field: AutocompleteField,
    target_entity_field: AutocompleteField,
    validation_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct RenameComparisonForm {
    new_name: String,
    name_input_state: crate::tui::widgets::TextInputState,
}

#[derive(Clone, Default)]
pub struct State {
    migration_name: Option<String>,
    source_env: Option<String>,
    target_env: Option<String>,
    comparisons: Vec<SavedComparison>,
    list_state: ListState,
    source_entities: Resource<Vec<String>>,
    target_entities: Resource<Vec<String>>,
    show_create_modal: bool,
    create_form: CreateComparisonForm,
    show_delete_confirm: bool,
    delete_comparison_id: Option<i64>,
    delete_comparison_name: Option<String>,
    show_rename_modal: bool,
    rename_comparison_id: Option<i64>,
    rename_form: RenameComparisonForm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitiesLoadedData {
    pub source_entities: Vec<String>,
    pub target_entities: Vec<String>,
}

#[derive(Clone, serde::Deserialize)]
pub struct MigrationMetadata {
    pub migration_name: String,
    pub source_env: String,
    pub target_env: String,
}

#[derive(Clone)]
pub enum Msg {
    Initialize(MigrationMetadata),
    ParallelDataLoaded(usize, Result<Vec<String>, String>),
    ComparisonsLoaded(Result<Vec<SavedComparison>, String>),
    ListNavigate(KeyCode),
    SelectComparison,
    CreateComparison,
    CreateFormNameChanged(KeyCode),
    CreateFormSourceEvent(AutocompleteEvent),
    CreateFormTargetEvent(AutocompleteEvent),
    CreateFormSubmit,
    CreateFormCancel,
    ComparisonCreated(Result<i64, String>),
    RequestDelete,
    ConfirmDelete,
    CancelDelete,
    ComparisonDeleted(Result<(), String>),
    RequestRename,
    RenameFormNameChanged(KeyCode),
    RenameFormSubmit,
    RenameFormCancel,
    ComparisonRenamed(Result<(), String>),
    Back,
}

impl ListItem for SavedComparison {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Self::Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(
                format!("  {} ({} -> {})", self.name, self.source_entity, self.target_entity),
                Style::default().fg(fg_color),
            ),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

impl crate::tui::AppState for State {}

impl App for MigrationComparisonSelectApp {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        log::debug!("MigrationComparisonSelectApp::update() called with message");
        match msg {
            Msg::Initialize(metadata) => {
                log::info!("✓ Initialize with migration: {} ({} -> {})",
                    metadata.migration_name, metadata.source_env, metadata.target_env);

                state.migration_name = Some(metadata.migration_name.clone());
                state.source_env = Some(metadata.source_env.clone());
                state.target_env = Some(metadata.target_env.clone());
                state.source_entities = Resource::Loading;
                state.target_entities = Resource::Loading;

                // Load entities in parallel with automatic LoadingScreen
                Command::perform_parallel()
                    .add_task(
                        format!("Loading source entities ({})", metadata.source_env),
                        async move {
                            use crate::api::metadata::parse_entity_list;
                            let config = crate::config();
                            let manager = crate::client_manager();

                            match config.get_entity_cache(&metadata.source_env, 24).await {
                                Ok(Some(cached)) => {
                                    log::debug!("Using cached entities for source: {}", metadata.source_env);
                                    Ok::<Vec<String>, String>(cached)
                                }
                                _ => {
                                    log::debug!("Fetching fresh metadata for source: {}", metadata.source_env);
                                    let client = manager.get_client(&metadata.source_env).await.map_err(|e| e.to_string())?;
                                    let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                    let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;

                                    match config.set_entity_cache(&metadata.source_env, entities.clone()).await {
                                        Ok(_) => log::info!("Successfully cached {} entities for {}", entities.len(), metadata.source_env),
                                        Err(e) => log::error!("Failed to cache entities for {}: {}", metadata.source_env, e),
                                    }

                                    Ok(entities)
                                }
                            }
                        }
                    )
                    .add_task(
                        format!("Loading target entities ({})", metadata.target_env),
                        async move {
                            use crate::api::metadata::parse_entity_list;
                            let config = crate::config();
                            let manager = crate::client_manager();

                            match config.get_entity_cache(&metadata.target_env, 24).await {
                                Ok(Some(cached)) => {
                                    log::debug!("Using cached entities for target: {}", metadata.target_env);
                                    Ok::<Vec<String>, String>(cached)
                                }
                                _ => {
                                    log::debug!("Fetching fresh metadata for target: {}", metadata.target_env);
                                    let client = manager.get_client(&metadata.target_env).await.map_err(|e| e.to_string())?;
                                    let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                    let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;

                                    match config.set_entity_cache(&metadata.target_env, entities.clone()).await {
                                        Ok(_) => log::info!("Successfully cached {} entities for {}", entities.len(), metadata.target_env),
                                        Err(e) => log::error!("Failed to cache entities for {}: {}", metadata.target_env, e),
                                    }

                                    Ok(entities)
                                }
                            }
                        }
                    )
                    .with_title("Loading Migration Data")
                    .on_complete(AppId::MigrationComparisonSelect)
                    .build(|task_idx, result| {
                        let data = result.downcast::<Result<Vec<String>, String>>().unwrap();
                        Msg::ParallelDataLoaded(task_idx, *data)
                    })
            }
            Msg::ParallelDataLoaded(task_idx, result) => {
                // Store result in appropriate Resource
                match task_idx {
                    0 => state.source_entities = Resource::from_result(result),
                    1 => state.target_entities = Resource::from_result(result),
                    _ => {}
                }

                // Load comparisons when both entities are loaded
                if let (Resource::Success(_), Resource::Success(_)) =
                    (&state.source_entities, &state.target_entities)
                {
                    let migration_name = state.migration_name.clone().unwrap();
                    Command::perform(
                        async move {
                            let config = crate::config();
                            config.get_comparisons(&migration_name).await.map_err(|e| e.to_string())
                        },
                        Msg::ComparisonsLoaded,
                    )
                } else {
                    Command::None
                }
            }
            Msg::ComparisonsLoaded(result) => {
                match result {
                    Ok(comparisons) => {
                        state.comparisons = comparisons;
                        state.list_state = ListState::new();
                        if !state.comparisons.is_empty() {
                            state.list_state.select(Some(0));
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to load comparisons: {}", e);
                    }
                }
                Command::None
            }
            Msg::ListNavigate(key) => {
                let visible_height = 20;
                state.list_state.handle_key(key, state.comparisons.len(), visible_height);
                Command::None
            }
            Msg::SelectComparison => {
                if let Some(_selected_idx) = state.list_state.selected() {
                    // TODO: Navigate to comparison detail app
                    log::info!("Selected comparison");
                }
                Command::None
            }
            Msg::CreateComparison => {
                state.show_create_modal = true;
                state.create_form = CreateComparisonForm::default();
                Command::set_focus(FocusId::new("create-name-input"))
            }
            Msg::CreateFormNameChanged(key) => {
                if let Some(new_value) = state.create_form.name_input_state.handle_key(
                    key,
                    &state.create_form.name,
                    Some(50), // Max 50 characters
                ) {
                    state.create_form.name = new_value;
                }
                Command::None
            }
            Msg::CreateFormSourceEvent(event) => {
                let options = state.source_entities.as_ref().ok().cloned().unwrap_or_default();
                state.create_form.source_entity_field.handle_event::<Msg>(event, &options);
                Command::None
            }
            Msg::CreateFormTargetEvent(event) => {
                let options = state.target_entities.as_ref().ok().cloned().unwrap_or_default();
                state.create_form.target_entity_field.handle_event::<Msg>(event, &options);
                Command::None
            }
            Msg::CreateFormSubmit => {
                let name = state.create_form.name.trim().to_string();
                let source_entity = state.create_form.source_entity_field.value().trim().to_string();
                let target_entity = state.create_form.target_entity_field.value().trim().to_string();

                if name.is_empty() {
                    state.create_form.validation_error = Some("Comparison name is required".to_string());
                    return Command::None;
                }

                if source_entity.is_empty() {
                    state.create_form.validation_error = Some("Source entity is required".to_string());
                    return Command::None;
                }

                if target_entity.is_empty() {
                    state.create_form.validation_error = Some("Target entity is required".to_string());
                    return Command::None;
                }

                // Validate that entities exist in their respective lists
                if let Resource::Success(source_list) = &state.source_entities {
                    if !source_list.contains(&source_entity) {
                        state.create_form.validation_error = Some(format!("Source entity '{}' not found", source_entity));
                        return Command::None;
                    }
                }

                if let Resource::Success(target_list) = &state.target_entities {
                    if !target_list.contains(&target_entity) {
                        state.create_form.validation_error = Some(format!("Target entity '{}' not found", target_entity));
                        return Command::None;
                    }
                }

                let migration_name = state.migration_name.clone().unwrap_or_default();
                state.show_create_modal = false;
                state.create_form.validation_error = None;

                Command::perform(
                    async move {
                        let config = crate::config();
                        let comparison = SavedComparison {
                            id: 0, // Will be assigned by database
                            name,
                            migration_name,
                            source_entity,
                            target_entity,
                            entity_comparison: None,
                            created_at: chrono::Utc::now(),
                            last_used: chrono::Utc::now(),
                        };
                        config.add_comparison(comparison).await
                            .map_err(|e| e.to_string())
                    },
                    Msg::ComparisonCreated
                )
            }
            Msg::CreateFormCancel => {
                state.show_create_modal = false;
                state.create_form.validation_error = None;
                Command::None
            }
            Msg::ComparisonCreated(result) => {
                match result {
                    Ok(id) => {
                        log::info!("Created comparison with ID: {}", id);
                        // Reload comparisons list
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        Command::perform(
                            async move {
                                let config = crate::config();
                                config.get_comparisons(&migration_name).await
                                    .map_err(|e| e.to_string())
                            },
                            Msg::ComparisonsLoaded
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to create comparison: {}", e);
                        Command::None
                    }
                }
            }
            Msg::RequestDelete => {
                // Get selected comparison
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(comparison) = state.comparisons.get(selected_idx) {
                        state.delete_comparison_id = Some(comparison.id);
                        state.delete_comparison_name = Some(comparison.name.clone());
                        state.show_delete_confirm = true;
                    }
                }
                Command::None
            }
            Msg::ConfirmDelete => {
                if let Some(id) = state.delete_comparison_id {
                    state.show_delete_confirm = false;
                    // Async delete from database
                    Command::perform(
                        async move {
                            let config = crate::config();
                            config.delete_comparison(id).await.map_err(|e| e.to_string())
                        },
                        Msg::ComparisonDeleted
                    )
                } else {
                    Command::None
                }
            }
            Msg::CancelDelete => {
                state.show_delete_confirm = false;
                state.delete_comparison_id = None;
                state.delete_comparison_name = None;
                Command::None
            }
            Msg::ComparisonDeleted(result) => {
                match result {
                    Ok(_) => {
                        state.delete_comparison_id = None;
                        state.delete_comparison_name = None;
                        // Reload comparisons list
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        Command::perform(
                            async move {
                                let config = crate::config();
                                config.get_comparisons(&migration_name).await
                                    .map_err(|e| e.to_string())
                            },
                            Msg::ComparisonsLoaded
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to delete comparison: {}", e);
                        Command::None
                    }
                }
            }
            Msg::RequestRename => {
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(comparison) = state.comparisons.get(selected_idx) {
                        state.rename_comparison_id = Some(comparison.id);
                        state.rename_form.new_name = comparison.name.clone();
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
                let id = state.rename_comparison_id;
                let new_name = state.rename_form.new_name.trim().to_string();

                if new_name.is_empty() || id.is_none() {
                    return Command::None;
                }

                state.show_rename_modal = false;
                let id = id.unwrap();

                Command::perform(
                    async move {
                        let config = crate::config();
                        config.rename_comparison(id, &new_name).await
                            .map_err(|e| e.to_string())
                    },
                    Msg::ComparisonRenamed
                )
            }
            Msg::RenameFormCancel => {
                state.show_rename_modal = false;
                state.rename_comparison_id = None;
                state.rename_form = RenameComparisonForm::default();
                Command::None
            }
            Msg::ComparisonRenamed(result) => {
                match result {
                    Ok(_) => {
                        state.rename_comparison_id = None;
                        state.rename_form = RenameComparisonForm::default();
                        // Reload list
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        Command::perform(
                            async move {
                                let config = crate::config();
                                config.get_comparisons(&migration_name).await
                                    .map_err(|e| e.to_string())
                            },
                            Msg::ComparisonsLoaded
                        )
                    }
                    Err(e) => {
                        log::error!("Failed to rename comparison: {}", e);
                        Command::None
                    }
                }
            }
            Msg::Back => Command::navigate_to(AppId::MigrationEnvironment),
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> Element<Self::Msg> {
        use crate::tui::element::Alignment;
        use_constraints!();

        log::trace!("MigrationComparisonSelectApp::view() - migration_name={:?}, comparisons={}",
            state.migration_name, state.comparisons.len());
        let list_content = if state.comparisons.is_empty() {
            Element::text("")
        } else {
            Element::list(
                "comparison-list",
                &state.comparisons,
                &state.list_state,
                theme,
            )
            .on_navigate(Msg::ListNavigate)
            .build()
        };

        let main_ui = Element::panel(list_content)
            .title("Comparisons")
            .build();

        if state.show_delete_confirm {
            // Render delete confirmation modal
            let comparison_name = state.delete_comparison_name.as_deref().unwrap_or("Unknown");

            Element::modal_confirm(
                main_ui,
                "Delete Comparison",
                format!("Delete comparison '{}'?", comparison_name),
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
                    &mut state.rename_form.name_input_state
                )
                .placeholder("Comparison name")
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
            .title("Rename Comparison")
            .width(60)
            .height(13)
            .build();

            modal!(main_ui, modal_content)
        } else if state.show_create_modal {
            // Name input (using TextInput directly without autocomplete for simple text)
            let name_input = Element::panel(
                Element::text_input(
                    "create-name-input",
                    &state.create_form.name,
                    &mut state.create_form.name_input_state,
                )
                .placeholder("Comparison name")
                .on_change(Msg::CreateFormNameChanged)
                .build()
            )
            .title("Name")
            .build();

            // Source entity label and autocomplete
            let source_label = Element::styled_text(Line::from(vec![
                Span::styled("Source Entity", Style::default().fg(theme.text)),
            ])).build();

            let source_autocomplete = Element::autocomplete(
                "create-source-autocomplete",
                state.source_entities.as_ref().ok().cloned().unwrap_or_default(),
                state.create_form.source_entity_field.value().to_string(),
                &mut state.create_form.source_entity_field.state,
            )
            .placeholder("Type source entity name...")
            .on_event(Msg::CreateFormSourceEvent)
            .build();

            // Target entity label and autocomplete
            let target_label = Element::styled_text(Line::from(vec![
                Span::styled("Target Entity", Style::default().fg(theme.text)),
            ])).build();

            let target_autocomplete = Element::autocomplete(
                "create-target-autocomplete",
                state.target_entities.as_ref().ok().cloned().unwrap_or_default(),
                state.create_form.target_entity_field.value().to_string(),
                &mut state.create_form.target_entity_field.state,
            )
            .placeholder("Type target entity name...")
            .on_event(Msg::CreateFormTargetEvent)
            .build();

            // Buttons
            let buttons = button_row![
                ("create-cancel", "Cancel", Msg::CreateFormCancel),
                ("create-confirm", "Create", Msg::CreateFormSubmit),
            ];

            // Modal content
            let modal_body = if state.create_form.validation_error.is_some() {
                col![
                    name_input => Length(3),
                    spacer!() => Length(1),
                    source_label => Length(1),
                    source_autocomplete => Length(3),
                    spacer!() => Length(1),
                    target_label => Length(1),
                    target_autocomplete => Length(3),
                    spacer!() => Length(1),
                    error_display!(state.create_form.validation_error, theme) => Length(2),
                    buttons => Length(3),
                ]
            } else {
                col![
                    name_input => Length(3),
                    spacer!() => Length(1),
                    source_label => Length(1),
                    source_autocomplete => Length(3),
                    spacer!() => Length(1),
                    target_label => Length(1),
                    target_autocomplete => Length(3),
                    spacer!() => Length(1),
                    buttons => Length(3),
                ]
            };

            let modal_content = Element::panel(
                Element::container(modal_body)
                .padding(2)
                .build()
            )
            .title("Create New Comparison")
            .width(80)
            .height(if state.create_form.validation_error.is_some() { 25 } else { 23 })
            .build();

            modal!(main_ui, modal_content)
        } else {
            main_ui
        }
    }

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        let mut subs = vec![
            // Listen for migration selection from MigrationEnvironmentApp
            Subscription::subscribe("migration:selected", |data| {
                log::info!("✓ Subscription handler called for 'migration:selected' event");
                log::debug!("  Raw data: {:?}", data);
                match serde_json::from_value::<MigrationMetadata>(data.clone()) {
                    Ok(metadata) => {
                        log::info!("✓ Successfully deserialized migration metadata");
                        Some(Msg::Initialize(metadata))
                    }
                    Err(e) => {
                        log::error!("✗ Failed to deserialize migration metadata: {}", e);
                        log::error!("  Data was: {:?}", data);
                        None
                    }
                }
            }),
        ];

        if !state.show_create_modal && !state.show_delete_confirm && !state.show_rename_modal {
            subs.push(Subscription::keyboard(KeyCode::Esc, "Back to migration list", Msg::Back));
            subs.push(Subscription::keyboard(KeyCode::Char('b'), "Back to migration list", Msg::Back));
            subs.push(Subscription::keyboard(KeyCode::Char('B'), "Back to migration list", Msg::Back));

            if !state.comparisons.is_empty() {
                subs.push(Subscription::keyboard(
                    KeyCode::Enter,
                    "Select comparison",
                    Msg::SelectComparison,
                ));
            }

            subs.push(Subscription::keyboard(
                KeyCode::Char('n'),
                "Create comparison",
                Msg::CreateComparison,
            ));
            subs.push(Subscription::keyboard(
                KeyCode::Char('N'),
                "Create comparison",
                Msg::CreateComparison,
            ));
            subs.push(Subscription::keyboard(
                KeyCode::Char('d'),
                "Delete comparison",
                Msg::RequestDelete,
            ));
            subs.push(Subscription::keyboard(
                KeyCode::Char('D'),
                "Delete comparison",
                Msg::RequestDelete,
            ));
            subs.push(Subscription::keyboard(
                KeyCode::Char('r'),
                "Rename comparison",
                Msg::RequestRename,
            ));
            subs.push(Subscription::keyboard(
                KeyCode::Char('R'),
                "Rename comparison",
                Msg::RequestRename,
            ));
        }

        subs
    }

    fn title() -> &'static str {
        "Migration Comparison Select"
    }

    fn status(state: &Self::State, theme: &Theme) -> Option<Line<'static>> {
        log::trace!("MigrationComparisonSelectApp::status() - migration_name={:?}", state.migration_name);
        if let Some(ref migration_name) = state.migration_name {
            let source = state.source_env.as_deref().unwrap_or("?");
            let target = state.target_env.as_deref().unwrap_or("?");
            let source_count = state.source_entities.as_ref().ok().map(|v| v.len()).unwrap_or(0);
            let target_count = state.target_entities.as_ref().ok().map(|v| v.len()).unwrap_or(0);
            Some(Line::from(vec![
                Span::styled(migration_name.clone(), Style::default().fg(theme.text)),
                Span::styled(
                    format!(" ({} → {})", source, target),
                    Style::default().fg(theme.subtext1),
                ),
                Span::styled(
                    format!(" ({}:{})", source_count, target_count),
                    Style::default().fg(theme.overlay1),
                ),
            ]))
        } else {
            Some(Line::from(vec![
                Span::styled("Loading migration data...", Style::default().fg(theme.subtext1))
            ]))
        }
    }
}
