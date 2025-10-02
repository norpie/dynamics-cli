use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{ColumnBuilder, Element, FocusId, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    widgets::list::{ListItem, ListState},
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
pub struct State {
    migration_name: Option<String>,
    source_env: Option<String>,
    target_env: Option<String>,
    comparisons: Vec<SavedComparison>,
    list_state: ListState,
    source_entities: Vec<String>,
    target_entities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationSelectedData {
    pub name: String,
    pub source_env: String,
    pub target_env: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntitiesLoadedData {
    pub source_entities: Vec<String>,
    pub target_entities: Vec<String>,
}

#[derive(Clone)]
pub enum Msg {
    MigrationSelected(MigrationSelectedData),
    StartLoading(MigrationSelectedData),
    DataLoaded(Result<(Vec<SavedComparison>, Vec<String>, Vec<String>), String>),
    ComparisonsLoaded(Result<Vec<SavedComparison>, String>),
    EntitiesLoaded(EntitiesLoadedData),
    ListNavigate(KeyCode),
    SelectComparison,
    CreateComparison,
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
        match msg {
            Msg::MigrationSelected(data) => {
                log::info!("Migration selected: {} ({} -> {})", data.name, data.source_env, data.target_env);
                state.migration_name = Some(data.name.clone());
                state.source_env = Some(data.source_env.clone());
                state.target_env = Some(data.target_env.clone());

                // Navigate to loading screen, then trigger loading via event
                let loading_init = serde_json::json!({
                    "tasks": ["Loading entity metadata and comparisons"],
                    "target": "MigrationComparisonSelect",
                    "caller": "MigrationComparisonSelect",
                    "cancellable": false,
                });

                Command::batch(vec![
                    Command::publish("loading:init", loading_init),
                    Command::navigate_to(AppId::LoadingScreen),
                    // Publish event to trigger loading AFTER navigation completes
                    Command::publish("comparison:start_loading", serde_json::to_value(&data).unwrap()),
                ])
            }
            Msg::StartLoading(data) => {
                log::info!("Starting entity and comparison loading for migration: {}", data.name);
                // Now that we're on the loading screen, start the async work
                Command::perform(
                    async move {
                        use crate::api::metadata::parse_entity_list;

                            let config = crate::config();
                            let manager = crate::client_manager();

                            // Load source entities
                            let source_entities = match config.get_entity_cache(&data.source_env, 24).await {
                                Ok(Some(cached)) => {
                                    log::debug!("Using cached entities for source: {}", data.source_env);
                                    cached
                                }
                                _ => {
                                    log::debug!("Fetching fresh metadata for source: {}", data.source_env);
                                    let client = manager.get_client(&data.source_env).await.map_err(|e| e.to_string())?;
                                    let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                    let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;
                                    let _ = config.set_entity_cache(&data.source_env, entities.clone()).await;
                                    entities
                                }
                            };

                            // Load target entities
                            let target_entities = match config.get_entity_cache(&data.target_env, 24).await {
                                Ok(Some(cached)) => {
                                    log::debug!("Using cached entities for target: {}", data.target_env);
                                    cached
                                }
                                _ => {
                                    log::debug!("Fetching fresh metadata for target: {}", data.target_env);
                                    let client = manager.get_client(&data.target_env).await.map_err(|e| e.to_string())?;
                                    let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                                    let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;
                                    let _ = config.set_entity_cache(&data.target_env, entities.clone()).await;
                                    entities
                                }
                            };

                            // Load comparisons
                            log::info!("Loading comparisons for migration: {}", data.name);
                            let comparisons = config.get_comparisons(&data.name).await.map_err(|e| e.to_string())?;

                            log::info!("Successfully loaded all data: {} comparisons, {} source entities, {} target entities",
                                comparisons.len(), source_entities.len(), target_entities.len());

                            Ok::<_, String>((comparisons, source_entities, target_entities))
                        },
                        |result| {
                            Msg::DataLoaded(result)
                        },
                    )
            }
            Msg::DataLoaded(result) => {
                match result {
                    Ok((comparisons, source_entities, target_entities)) => {
                        state.comparisons = comparisons;
                        state.source_entities = source_entities;
                        state.target_entities = target_entities;
                        state.list_state = ListState::new();
                        if !state.comparisons.is_empty() {
                            state.list_state.select(Some(0));
                        }
                        log::debug!("Loaded {} comparisons, {} source entities, {} target entities",
                            state.comparisons.len(), state.source_entities.len(), state.target_entities.len());
                        Command::None
                    }
                    Err(e) => {
                        log::error!("Failed to load data: {}", e);
                        Command::batch(vec![
                            Command::publish("error:init", serde_json::json!({
                                "message": format!("Failed to load migration data: {}", e),
                                "target": "MigrationEnvironment",
                            })),
                            Command::navigate_to(AppId::ErrorScreen),
                        ])
                    }
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
                // TODO: Navigate to create comparison modal
                log::info!("Create comparison");
                Command::None
            }
            Msg::Back => Command::navigate_to(AppId::MigrationEnvironment),
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> Element<Self::Msg> {
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

        Element::panel(list_content)
            .title("Comparisons")
            .build()
    }

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        let mut subs = vec![
            // Listen for migration selection events
            Subscription::subscribe("migration_selected", |data| {
                serde_json::from_value::<MigrationSelectedData>(data)
                    .ok()
                    .map(Msg::MigrationSelected)
            }),
            // Listen for start loading event (fired after navigation to loading screen)
            Subscription::subscribe("comparison:start_loading", |data| {
                serde_json::from_value::<MigrationSelectedData>(data)
                    .ok()
                    .map(Msg::StartLoading)
            }),
            // Listen for entities loaded events
            Subscription::subscribe("entities_loaded", |data| {
                serde_json::from_value::<EntitiesLoadedData>(data)
                    .ok()
                    .map(Msg::EntitiesLoaded)
            }),
            // Back navigation
            Subscription::keyboard(KeyCode::Esc, "Back to migration list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('b'), "Back to migration list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('B'), "Back to migration list", Msg::Back),
        ];

        if !state.comparisons.is_empty() {
            subs.push(Subscription::keyboard(
                KeyCode::Enter,
                "Select comparison",
                Msg::SelectComparison,
            ));
        }

        subs.push(Subscription::keyboard(
            KeyCode::Char('c'),
            "Create comparison",
            Msg::CreateComparison,
        ));
        subs.push(Subscription::keyboard(
            KeyCode::Char('C'),
            "Create comparison",
            Msg::CreateComparison,
        ));

        subs
    }

    fn title() -> &'static str {
        "Migration Comparison Select"
    }

    fn status(state: &Self::State, theme: &Theme) -> Option<Line<'static>> {
        if let Some(ref migration_name) = state.migration_name {
            let source = state.source_env.as_deref().unwrap_or("?");
            let target = state.target_env.as_deref().unwrap_or("?");
            Some(Line::from(vec![
                Span::styled(migration_name.clone(), Style::default().fg(theme.text)),
                Span::styled(
                    format!(" ({} → {})", source, target),
                    Style::default().fg(theme.subtext1),
                ),
            ]))
        } else {
            Some(Line::from(vec![
                Span::styled("No migration selected", Style::default().fg(theme.subtext1))
            ]))
        }
    }
}
