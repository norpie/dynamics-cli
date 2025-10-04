use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
};
use crossterm::event::KeyCode;
use ratatui::{
    prelude::Stylize,
    style::Style,
    text::{Line, Span},
};
use crate::{col, row, use_constraints};

pub struct EntityComparisonApp;

#[derive(Clone, Default)]
pub struct State {
    migration_name: String,
    source_env: String,
    target_env: String,
    source_entity: String,
    target_entity: String,
}

#[derive(Clone)]
pub enum Msg {
    Back,
}

pub struct EntityComparisonParams {
    pub migration_name: String,
    pub source_env: String,
    pub target_env: String,
    pub source_entity: String,
    pub target_entity: String,
}

impl Default for EntityComparisonParams {
    fn default() -> Self {
        Self {
            migration_name: String::new(),
            source_env: String::new(),
            target_env: String::new(),
            source_entity: String::new(),
            target_entity: String::new(),
        }
    }
}

impl crate::tui::AppState for State {}

impl App for EntityComparisonApp {
    type State = State;
    type Msg = Msg;
    type InitParams = EntityComparisonParams;

    fn init(params: EntityComparisonParams) -> (State, Command<Msg>) {
        let state = State {
            migration_name: params.migration_name,
            source_env: params.source_env,
            target_env: params.target_env,
            source_entity: params.source_entity,
            target_entity: params.target_entity,
        };

        (state, Command::None)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::Back => Command::navigate_to(AppId::MigrationComparisonSelect),
        }
    }

    fn view(state: &mut Self::State, theme: &Theme) -> LayeredView<Self::Msg> {
        use_constraints!();

        // Empty source panel
        let source_panel = Element::panel(
            Element::text("")
        )
        .title(format!("Source: {}", state.source_entity))
        .build();

        // Empty target panel
        let target_panel = Element::panel(
            Element::text("")
        )
        .title(format!("Target: {}", state.target_entity))
        .build();

        // Side-by-side layout
        let main_ui = row![
            source_panel => Fill(1),
            target_panel => Fill(1),
        ];

        LayeredView::new(main_ui)
    }

    fn subscriptions(_state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Back to comparison list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('b'), "Back to comparison list", Msg::Back),
            Subscription::keyboard(KeyCode::Char('B'), "Back to comparison list", Msg::Back),
        ]
    }

    fn title() -> &'static str {
        "Entity Comparison"
    }

    fn status(state: &Self::State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::styled(
                format!("Migration: {}", state.migration_name),
                Style::default().fg(theme.text),
            ),
            Span::styled(" | ", Style::default().fg(theme.overlay1)),
            Span::styled(
                format!("{} → {}", state.source_env, state.target_env),
                Style::default().fg(theme.subtext1),
            ),
            Span::styled(" | ", Style::default().fg(theme.overlay1)),
            Span::styled(
                format!("{} ↔ {}", state.source_entity, state.target_entity),
                Style::default().fg(theme.blue),
            ),
        ]))
    }
}
