use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{ColumnBuilder, Element, FocusId, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    widgets::list::{ListItem, ListState},
    widgets::{AutocompleteField, AutocompleteEvent, TextInputField, TextInputEvent},
    renderer::LayeredView,
    Resource,
};
use dynamics_lib_macros::Validate;
use crate::config::repository::migrations::SavedComparison;
use crossterm::event::KeyCode;
use ratatui::{
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};
use crate::{col, row, spacer, button_row, use_constraints, error_display};

pub struct MigrationComparisonSelectApp;

#[derive(Clone, Default, Validate)]
pub struct CreateComparisonForm {
    #[validate(not_empty, message = "Comparison name is required")]
    name: TextInputField,

    #[validate(not_empty, message = "Source entity is required")]
    source_entity: AutocompleteField,

    #[validate(not_empty, message = "Target entity is required")]
    target_entity: AutocompleteField,

    validation_error: Option<String>,
}

#[derive(Clone, Default, Validate)]
pub struct RenameComparisonForm {
    #[validate(not_empty, message = "Comparison name is required")]
    new_name: TextInputField,
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
    ParallelDataLoaded(usize, Result<Vec<String>, String>),
    ComparisonsLoaded(Result<Vec<SavedComparison>, String>),
    ListNavigate(KeyCode),
    SelectComparison,
    CreateComparison,
    CreateFormNameEvent(TextInputEvent),
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
    RenameFormNameEvent(TextInputEvent),
    RenameFormSubmit,
    RenameFormCancel,
    ComparisonRenamed(Result<(), String>),
    Back,
}

impl ListItem for SavedComparison {
    type Msg = Msg;

    fn to_element(&self, is_selected: bool, _is_hovered: bool) -> Element<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;
        let (fg_color, bg_style) = if is_selected {
            (theme.accent_primary, Some(Style::default().bg(theme.bg_surface)))
        } else {
            (theme.text_primary, None)
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

impl State {
    fn open_delete_modal(&mut self, comparison_id: i64, comparison_name: String) {
        self.delete_comparison_id = Some(comparison_id);
        self.delete_comparison_name = Some(comparison_name);
        self.show_delete_confirm = true;
    }

    fn close_delete_modal(&mut self) {
        self.show_delete_confirm = false;
        self.delete_comparison_id = None;
        self.delete_comparison_name = None;
    }

    fn open_rename_modal(&mut self, comparison_id: i64, comparison_name: String) {
        self.rename_comparison_id = Some(comparison_id);
        self.rename_form.new_name.set_value(comparison_name);
        self.show_rename_modal = true;
    }

    fn close_rename_modal(&mut self) {
        self.show_rename_modal = false;
        self.rename_comparison_id = None;
        self.rename_form = RenameComparisonForm::default();
    }

    fn close_create_modal(&mut self) {
        self.show_create_modal = false;
        self.create_form.validation_error = None;
    }
}

pub struct MigrationSelectParams {
    pub migration_name: String,
    pub source_env: String,
    pub target_env: String,
}

impl Default for MigrationSelectParams {
    fn default() -> Self {
        Self {
            migration_name: String::new(),
            source_env: String::new(),
            target_env: String::new(),
        }
    }
}

impl App for MigrationComparisonSelectApp {
    type State = State;
    type Msg = Msg;
    type InitParams = MigrationSelectParams;

    fn init(params: MigrationSelectParams) -> (State, Command<Msg>) {
        let mut state = State::default();
        state.migration_name = Some(params.migration_name.clone());
        state.source_env = Some(params.source_env.clone());
        state.target_env = Some(params.target_env.clone());
        state.source_entities = crate::tui::Resource::Loading;
        state.target_entities = crate::tui::Resource::Loading;

        // Load entities in parallel with automatic LoadingScreen
        let cmd = Command::perform_parallel()
            .add_task(
                format!("Loading source entities ({})", params.source_env),
                {
                    let source_env = params.source_env.clone();
                    async move {
                        use crate::api::metadata::parse_entity_list;
                        let config = crate::global_config();
                        let manager = crate::client_manager();

                        match config.get_entity_cache(&source_env, 24).await {
                            Ok(Some(cached)) => Ok::<Vec<String>, String>(cached),
                            _ => {
                                let client = manager.get_client(&source_env).await.map_err(|e| e.to_string())?;
                                let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;
                                let _ = config.set_entity_cache(&source_env, entities.clone()).await;
                                Ok(entities)
                            }
                        }
                    }
                }
            )
            .add_task(
                format!("Loading target entities ({})", params.target_env),
                {
                    let target_env = params.target_env.clone();
                    async move {
                        use crate::api::metadata::parse_entity_list;
                        let config = crate::global_config();
                        let manager = crate::client_manager();

                        match config.get_entity_cache(&target_env, 24).await {
                            Ok(Some(cached)) => Ok::<Vec<String>, String>(cached),
                            _ => {
                                let client = manager.get_client(&target_env).await.map_err(|e| e.to_string())?;
                                let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;
                                let _ = config.set_entity_cache(&target_env, entities.clone()).await;
                                Ok(entities)
                            }
                        }
                    }
                }
            )
            .with_title("Loading Migration Data")
            .on_complete(AppId::MigrationComparisonSelect)
            .build(|task_idx, result| {
                let data = result.downcast::<Result<Vec<String>, String>>().unwrap();
                Msg::ParallelDataLoaded(task_idx, *data)
            });

        (state, cmd)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        log::debug!("MigrationComparisonSelectApp::update() called with message");
        match msg {
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
                            let config = crate::global_config();
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
                            // Focus the list after loading comparisons
                            return Command::set_focus(FocusId::new("comparison-list"));
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
                log::info!("SelectComparison triggered - list size: {}, selected: {:?}",
                    state.comparisons.len(), state.list_state.selected());
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(comparison) = state.comparisons.get(selected_idx) {
                        log::info!("Opening comparison: {} -> {}",
                            comparison.source_entity, comparison.target_entity);
                        let params = super::entity_comparison::EntityComparisonParams {
                            migration_name: state.migration_name.clone().unwrap_or_default(),
                            source_env: state.source_env.clone().unwrap_or_default(),
                            target_env: state.target_env.clone().unwrap_or_default(),
                            source_entity: comparison.source_entity.clone(),
                            target_entity: comparison.target_entity.clone(),
                        };
                        return Command::start_app(AppId::EntityComparison, params);
                    } else {
                        log::warn!("Selected index {} out of bounds", selected_idx);
                    }
                } else {
                    log::warn!("No comparison selected");
                }
                Command::None
            }
            Msg::CreateComparison => {
                state.show_create_modal = true;
                state.create_form = CreateComparisonForm::default();
                Command::set_focus(FocusId::new("create-name-input"))
            }
            Msg::CreateFormNameEvent(event) => {
                state.create_form.name.handle_event(event, Some(50));
                Command::None
            }
            Msg::CreateFormSourceEvent(event) => {
                let options = state.source_entities.as_ref().ok().cloned().unwrap_or_default();
                state.create_form.source_entity.handle_event::<Msg>(event, &options);
                Command::None
            }
            Msg::CreateFormTargetEvent(event) => {
                let options = state.target_entities.as_ref().ok().cloned().unwrap_or_default();
                state.create_form.target_entity.handle_event::<Msg>(event, &options);
                Command::None
            }
            Msg::CreateFormSubmit => {
                // Validate using generated macro method
                match state.create_form.validate() {
                    Ok(_) => {
                        let name = state.create_form.name.value().trim().to_string();
                        let source_entity = state.create_form.source_entity.value().trim().to_string();
                        let target_entity = state.create_form.target_entity.value().trim().to_string();

                        // Additional validation: check entities exist in lists
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
                                let config = crate::global_config();
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
                    Err(validation_error) => {
                        state.create_form.validation_error = Some(validation_error);
                        Command::None
                    }
                }
            }
            Msg::CreateFormCancel => {
                state.close_create_modal();
                Command::None
            }
            Msg::ComparisonCreated(result) => {
                match result {
                    Ok(id) => {
                        log::info!("Created comparison with ID: {}", id);
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        reload_comparisons(migration_name)
                    }
                    Err(e) => {
                        log::error!("Failed to create comparison: {}", e);
                        Command::None
                    }
                }
            }
            Msg::RequestDelete => {
                if let Some(selected_idx) = state.list_state.selected() {
                    if let Some(comparison) = state.comparisons.get(selected_idx) {
                        state.open_delete_modal(comparison.id, comparison.name.clone());
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
                            let config = crate::global_config();
                            config.delete_comparison(id).await.map_err(|e| e.to_string())
                        },
                        Msg::ComparisonDeleted
                    )
                } else {
                    Command::None
                }
            }
            Msg::CancelDelete => {
                state.close_delete_modal();
                Command::None
            }
            Msg::ComparisonDeleted(result) => {
                match result {
                    Ok(_) => {
                        state.close_delete_modal();
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        reload_comparisons(migration_name)
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
                        state.open_rename_modal(comparison.id, comparison.name.clone());
                    }
                }
                Command::None
            }
            Msg::RenameFormNameEvent(event) => {
                state.rename_form.new_name.handle_event(event, Some(50));
                Command::None
            }
            Msg::RenameFormSubmit => {
                let id = state.rename_comparison_id;
                let new_name = state.rename_form.new_name.value().trim().to_string();

                if new_name.is_empty() || id.is_none() {
                    return Command::None;
                }

                state.show_rename_modal = false;
                let id = id.unwrap();

                Command::perform(
                    async move {
                        let config = crate::global_config();
                        config.rename_comparison(id, &new_name).await
                            .map_err(|e| e.to_string())
                    },
                    Msg::ComparisonRenamed
                )
            }
            Msg::RenameFormCancel => {
                state.close_rename_modal();
                Command::None
            }
            Msg::ComparisonRenamed(result) => {
                match result {
                    Ok(_) => {
                        state.close_rename_modal();
                        let migration_name = state.migration_name.clone().unwrap_or_default();
                        reload_comparisons(migration_name)
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

    fn view(state: &mut Self::State) -> LayeredView<Self::Msg> {
        use_constraints!();
        let theme = &crate::global_runtime_config().theme;

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
            .on_select(|_| Msg::SelectComparison)
            .on_navigate(Msg::ListNavigate)
            .on_activate(|_| Msg::SelectComparison)
            .build()
        };

        let main_ui = Element::panel(list_content)
            .title("Comparisons")
            .build();

        if state.show_delete_confirm {
            // Render delete confirmation modal
            let comparison_name = state.delete_comparison_name.as_deref().unwrap_or("Unknown");

            // Delete confirmation buttons
            let cancel_button = Element::button("delete-cancel", "Cancel".to_string())
                .on_press(Msg::CancelDelete)
                .build();

            let confirm_button = Element::button("delete-confirm", "Delete".to_string())
                .on_press(Msg::ConfirmDelete)
                .style(Style::default().fg(theme.accent_error))
                .build();

            let buttons = Element::row(vec![cancel_button, confirm_button])
                .spacing(2)
                .build();

            // Modal content
            let modal_content = Element::panel(
                Element::container(
                    col![
                        Element::styled_text(Line::from(vec![
                            Span::styled("Delete Comparison", Style::default().fg(theme.accent_tertiary).bold())
                        ])).build() => Length(1),
                        spacer!() => Length(1),
                        Element::text(format!("Delete comparison '{}'?\n\nThis action cannot be undone.", comparison_name)) => Length(3),
                        spacer!() => Length(1),
                        buttons => Length(3),
                    ]
                )
                .padding(2)
                .build()
            )
            .width(60)
            .height(13)
            .build();

            LayeredView::new(main_ui).with_app_modal(modal_content, crate::tui::Alignment::Center)
        } else if state.show_rename_modal {
            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    "rename-name-input",
                    state.rename_form.new_name.value(),
                    &state.rename_form.new_name.state
                )
                .placeholder("Comparison name")
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
            .title("Rename Comparison")
            .width(60)
            .height(13)
            .build();

            LayeredView::new(main_ui).with_app_modal(modal_content, crate::tui::Alignment::Center)
        } else if state.show_create_modal {
            // Name input (using TextInput directly without autocomplete for simple text)
            let name_input = Element::panel(
                Element::text_input(
                    "create-name-input",
                    state.create_form.name.value(),
                    &state.create_form.name.state,
                )
                .placeholder("Comparison name")
                .on_event(Msg::CreateFormNameEvent)
                .build()
            )
            .title("Name")
            .build();

            // Source entity autocomplete with panel
            let source_autocomplete = Element::panel(
                Element::autocomplete(
                    "create-source-autocomplete",
                    state.source_entities.as_ref().ok().cloned().unwrap_or_default(),
                    state.create_form.source_entity.value().to_string(),
                    &mut state.create_form.source_entity.state,
                )
                .placeholder("Type source entity name...")
                .on_event(Msg::CreateFormSourceEvent)
                .build()
            )
            .title("Source Entity")
            .build();

            // Target entity autocomplete with panel
            let target_autocomplete = Element::panel(
                Element::autocomplete(
                    "create-target-autocomplete",
                    state.target_entities.as_ref().ok().cloned().unwrap_or_default(),
                    state.create_form.target_entity.value().to_string(),
                    &mut state.create_form.target_entity.state,
                )
                .placeholder("Type target entity name...")
                .on_event(Msg::CreateFormTargetEvent)
                .build()
            )
            .title("Target Entity")
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
                    source_autocomplete => Length(3),
                    spacer!() => Length(1),
                    target_autocomplete => Length(3),
                    spacer!() => Length(1),
                    error_display!(state.create_form.validation_error, theme) => Length(2),
                    buttons => Length(3),
                ]
            } else {
                col![
                    name_input => Length(3),
                    spacer!() => Length(1),
                    source_autocomplete => Length(3),
                    spacer!() => Length(1),
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
            .height(if state.create_form.validation_error.is_some() { 23 } else { 21 })
            .build();

            LayeredView::new(main_ui).with_app_modal(modal_content, crate::tui::Alignment::Center)
        } else {
            LayeredView::new(main_ui)
        }
    }

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        let mut subs = vec![];

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
        } else if state.show_create_modal {
            subs.push(Subscription::keyboard(KeyCode::Esc, "Close modal", Msg::CreateFormCancel));
        } else if state.show_delete_confirm {
            subs.push(Subscription::keyboard(KeyCode::Esc, "Cancel delete", Msg::CancelDelete));
        } else if state.show_rename_modal {
            subs.push(Subscription::keyboard(KeyCode::Esc, "Close modal", Msg::RenameFormCancel));
        }

        subs
    }

    fn title() -> &'static str {
        "Migration Comparison Select"
    }

    fn status(state: &Self::State) -> Option<Line<'static>> {
        log::trace!("MigrationComparisonSelectApp::status() - migration_name={:?}", state.migration_name);
        let theme = &crate::global_runtime_config().theme;
        if let Some(ref migration_name) = state.migration_name {
            let source = state.source_env.as_deref().unwrap_or("?");
            let target = state.target_env.as_deref().unwrap_or("?");

            let source_count_str = match &state.source_entities {
                Resource::Loading => "...".to_string(),
                Resource::Success(v) => v.len().to_string(),
                Resource::Failure(_) => "ERR".to_string(),
                Resource::NotAsked => "0".to_string(),
            };

            let target_count_str = match &state.target_entities {
                Resource::Loading => "...".to_string(),
                Resource::Success(v) => v.len().to_string(),
                Resource::Failure(_) => "ERR".to_string(),
                Resource::NotAsked => "0".to_string(),
            };

            Some(Line::from(vec![
                Span::styled(migration_name.clone(), Style::default().fg(theme.text_primary)),
                Span::styled(
                    format!(" ({} → {})", source, target),
                    Style::default().fg(theme.text_secondary),
                ),
                Span::styled(
                    format!(" ({}:{})", source_count_str, target_count_str),
                    Style::default().fg(theme.border_primary),
                ),
            ]))
        } else {
            Some(Line::from(vec![
                Span::styled("Loading migration data...", Style::default().fg(theme.text_secondary))
            ]))
        }
    }
}

// Helper functions

fn reload_comparisons(migration_name: String) -> Command<Msg> {
    Command::perform(
        async move {
            let config = crate::global_config();
            config.get_comparisons(&migration_name).await.map_err(|e| e.to_string())
        },
        Msg::ComparisonsLoaded
    )
}
