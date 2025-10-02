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
            Msg::ComparisonDataReceived(data) => {
                log::info!("Comparison data received: {} ({} -> {})", data.migration_name, data.source_env, data.target_env);
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
                log::debug!("Loaded {} comparisons, {} source entities, {} target entities",
                    state.comparisons.len(), state.source_entities.len(), state.target_entities.len());
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
            // Listen for comparison data from MigrationEnvironmentApp
            Subscription::subscribe("comparison_data", |data| {
                serde_json::from_value::<crate::tui::apps::migration::migration_environment_app::ComparisonData>(data)
                    .ok()
                    .map(Msg::ComparisonDataReceived)
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
