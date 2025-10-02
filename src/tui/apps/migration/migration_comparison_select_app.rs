use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{ColumnBuilder, Element, FocusId, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    widgets::list::{ListItem, ListState},
    widgets::AutocompleteState,
};
use crate::config::repository::migrations::SavedComparison;
use crossterm::event::KeyCode;
use ratatui::{
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
};
use serde::{Deserialize, Serialize};

pub struct MigrationComparisonSelectApp;

#[derive(Clone, Default)]
pub struct CreateComparisonForm {
    name: String,
    name_input_state: crate::tui::widgets::TextInputState,
    source_entity: String,
    source_autocomplete_state: AutocompleteState,
    target_entity: String,
    target_autocomplete_state: AutocompleteState,
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
    source_entities: Vec<String>,
    target_entities: Vec<String>,
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

#[derive(Clone)]
pub enum Msg {
    ComparisonDataReceived(crate::tui::apps::migration::migration_environment_app::ComparisonData),
    ComparisonsLoaded(Result<Vec<SavedComparison>, String>),
    EntitiesLoaded(EntitiesLoadedData),
    ListNavigate(KeyCode),
    SelectComparison,
    CreateComparison,
    CreateFormNameChanged(KeyCode),
    CreateFormSourceInputChanged(KeyCode),
    CreateFormSourceNavigate(KeyCode),
    CreateFormSourceSelected(String),
    CreateFormTargetInputChanged(KeyCode),
    CreateFormTargetNavigate(KeyCode),
    CreateFormTargetSelected(String),
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

impl App for MigrationComparisonSelectApp {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        log::debug!("MigrationComparisonSelectApp::update() called with message");
        match msg {
            Msg::ComparisonDataReceived(data) => {
                log::info!("✓ ComparisonDataReceived message processed in update()");
                log::info!("  Migration: {} ({} -> {})", data.migration_name, data.source_env, data.target_env);
                log::info!("  Source entities: {}, Target entities: {}, Comparisons: {}",
                    data.source_entities.len(), data.target_entities.len(), data.comparisons.len());
                state.migration_name = Some(data.migration_name);
                state.source_env = Some(data.source_env);
                state.target_env = Some(data.target_env);
                state.comparisons = data.comparisons;
                state.source_entities = data.source_entities;
                state.target_entities = data.target_entities;
                state.list_state = ListState::new();
                if !state.comparisons.is_empty() {
                    state.list_state.select(Some(0));
                }
                log::info!("✓ State updated successfully");
                Command::None
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
            Msg::EntitiesLoaded(data) => {
                state.source_entities = data.source_entities;
                state.target_entities = data.target_entities;
                log::debug!("Loaded {} source entities and {} target entities",
                    state.source_entities.len(), state.target_entities.len());
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
            Msg::CreateFormSourceInputChanged(key) => {
                if let Some(new_value) = state.create_form.source_autocomplete_state.handle_input_key(
                    key,
                    &state.create_form.source_entity,
                    None,
                ) {
                    state.create_form.source_entity = new_value;
                    state.create_form.source_autocomplete_state.update_filtered_options(
                        &state.create_form.source_entity,
                        &state.source_entities,
                    );
                }
                Command::None
            }
            Msg::CreateFormSourceNavigate(key) => {
                match key {
                    KeyCode::Up => {
                        state.create_form.source_autocomplete_state.navigate_prev();
                    }
                    KeyCode::Down => {
                        state.create_form.source_autocomplete_state.navigate_next();
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = state.create_form.source_autocomplete_state.get_highlighted_option() {
                            state.create_form.source_entity = selected.clone();
                            state.create_form.source_autocomplete_state.close();
                            state.create_form.source_autocomplete_state.set_cursor_to_end(&state.create_form.source_entity);
                        }
                    }
                    KeyCode::Esc => {
                        state.create_form.source_autocomplete_state.close();
                    }
                    _ => {}
                }
                Command::None
            }
            Msg::CreateFormSourceSelected(entity) => {
                state.create_form.source_entity = entity.clone();
                state.create_form.source_autocomplete_state.close();
                state.create_form.source_autocomplete_state.set_cursor_to_end(&state.create_form.source_entity);
                Command::None
            }
            Msg::CreateFormTargetInputChanged(key) => {
                if let Some(new_value) = state.create_form.target_autocomplete_state.handle_input_key(
                    key,
                    &state.create_form.target_entity,
                    None,
                ) {
                    state.create_form.target_entity = new_value;
                    state.create_form.target_autocomplete_state.update_filtered_options(
                        &state.create_form.target_entity,
                        &state.target_entities,
                    );
                }
                Command::None
            }
            Msg::CreateFormTargetNavigate(key) => {
                match key {
                    KeyCode::Up => {
                        state.create_form.target_autocomplete_state.navigate_prev();
                    }
                    KeyCode::Down => {
                        state.create_form.target_autocomplete_state.navigate_next();
                    }
                    KeyCode::Enter => {
                        if let Some(selected) = state.create_form.target_autocomplete_state.get_highlighted_option() {
                            state.create_form.target_entity = selected.clone();
                            state.create_form.target_autocomplete_state.close();
                            state.create_form.target_autocomplete_state.set_cursor_to_end(&state.create_form.target_entity);
                        }
                    }
                    KeyCode::Esc => {
                        state.create_form.target_autocomplete_state.close();
                    }
                    _ => {}
                }
                Command::None
            }
            Msg::CreateFormTargetSelected(entity) => {
                state.create_form.target_entity = entity.clone();
                state.create_form.target_autocomplete_state.close();
                state.create_form.target_autocomplete_state.set_cursor_to_end(&state.create_form.target_entity);
                Command::None
            }
            Msg::CreateFormSubmit => {
                let name = state.create_form.name.trim().to_string();
                let source_entity = state.create_form.source_entity.trim().to_string();
                let target_entity = state.create_form.target_entity.trim().to_string();

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
                if !state.source_entities.contains(&source_entity) {
                    state.create_form.validation_error = Some(format!("Source entity '{}' not found", source_entity));
                    return Command::None;
                }

                if !state.target_entities.contains(&target_entity) {
                    state.create_form.validation_error = Some(format!("Target entity '{}' not found", target_entity));
                    return Command::None;
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
        use crate::tui::element::{Alignment, RowBuilder};

        log::trace!("MigrationComparisonSelectApp::view() - migration_name={:?}, comparisons={}",
            state.migration_name, state.comparisons.len());
        let list_content = if state.comparisons.is_empty() {
            Element::text("")
        } else {
            Element::list(
                FocusId::new("comparison-list"),
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
                FocusId::new("delete-cancel"),
                Msg::CancelDelete,
                FocusId::new("delete-confirm"),
                Msg::ConfirmDelete,
            )
        } else if state.show_rename_modal {
            use crate::tui::element::{RowBuilder};

            // Name input
            let name_input = Element::panel(
                Element::text_input(
                    FocusId::new("rename-name-input"),
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
            .title("Rename Comparison")
            .width(60)
            .height(13)
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
        } else if state.show_create_modal {
            // Name input (using TextInput directly without autocomplete for simple text)
            let name_input = Element::panel(
                Element::text_input(
                    FocusId::new("create-name-input"),
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
                FocusId::new("create-source-autocomplete"),
                state.source_entities.clone(),
                state.create_form.source_entity.clone(),
                &mut state.create_form.source_autocomplete_state,
            )
            .placeholder("Type source entity name...")
            .on_input(Msg::CreateFormSourceInputChanged)
            .on_select(Msg::CreateFormSourceSelected)
            .on_navigate(Msg::CreateFormSourceNavigate)
            .build();

            // Target entity label and autocomplete
            let target_label = Element::styled_text(Line::from(vec![
                Span::styled("Target Entity", Style::default().fg(theme.text)),
            ])).build();

            let target_autocomplete = Element::autocomplete(
                FocusId::new("create-target-autocomplete"),
                state.target_entities.clone(),
                state.create_form.target_entity.clone(),
                &mut state.create_form.target_autocomplete_state,
            )
            .placeholder("Type target entity name...")
            .on_input(Msg::CreateFormTargetInputChanged)
            .on_select(Msg::CreateFormTargetSelected)
            .on_navigate(Msg::CreateFormTargetNavigate)
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
                    Element::button(FocusId::new("create-confirm"), "Create")
                        .on_press(Msg::CreateFormSubmit)
                        .build(),
                    LayoutConstraint::Fill(1),
                )
                .spacing(0)
                .build();

            // Modal content
            let mut modal_builder = ColumnBuilder::new()
                .add(name_input, LayoutConstraint::Length(3))
                .add(Element::text(""), LayoutConstraint::Length(1))
                .add(source_label, LayoutConstraint::Length(1))
                .add(source_autocomplete, LayoutConstraint::Length(3))
                .add(Element::text(""), LayoutConstraint::Length(1))
                .add(target_label, LayoutConstraint::Length(1))
                .add(target_autocomplete, LayoutConstraint::Length(3))
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
            .title("Create New Comparison")
            .width(80)
            .height(if state.create_form.validation_error.is_some() { 25 } else { 23 })
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

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        let mut subs = vec![
            // Listen for comparison data from MigrationEnvironmentApp
            Subscription::subscribe("comparison_data", |data| {
                log::info!("✓ Subscription handler called for 'comparison_data' event");
                log::debug!("  Raw data: {:?}", data);
                match serde_json::from_value::<crate::tui::apps::migration::migration_environment_app::ComparisonData>(data.clone()) {
                    Ok(comparison_data) => {
                        log::info!("✓ Successfully deserialized comparison data");
                        Some(Msg::ComparisonDataReceived(comparison_data))
                    }
                    Err(e) => {
                        log::error!("✗ Failed to deserialize comparison data: {}", e);
                        log::error!("  Data was: {:?}", data);
                        None
                    }
                }
            }),
            // Listen for entities loaded events
            Subscription::subscribe("entities_loaded", |data| {
                serde_json::from_value::<EntitiesLoadedData>(data)
                    .ok()
                    .map(Msg::EntitiesLoaded)
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
            let source_count = state.source_entities.len();
            let target_count = state.target_entities.len();
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
