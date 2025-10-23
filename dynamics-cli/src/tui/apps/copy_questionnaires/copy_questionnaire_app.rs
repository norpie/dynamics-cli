use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::Element,
    subscription::Subscription,
    renderer::LayeredView,
};
use crossterm::event::KeyCode;
use ratatui::{
    text::{Line, Span},
    style::Style,
    prelude::Stylize,
};

pub struct CopyQuestionnaireApp;

#[derive(Clone)]
pub struct State {
    questionnaire_id: String,
    questionnaire_name: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            questionnaire_name: String::new(),
        }
    }
}

#[derive(Clone)]
pub enum Msg {
    Back,
    StartCopy, // Placeholder for future functionality
}

pub struct CopyQuestionnaireParams {
    pub questionnaire_id: String,
    pub questionnaire_name: String,
}

impl Default for CopyQuestionnaireParams {
    fn default() -> Self {
        Self {
            questionnaire_id: String::new(),
            questionnaire_name: String::new(),
        }
    }
}

impl crate::tui::AppState for State {}

impl App for CopyQuestionnaireApp {
    type State = State;
    type Msg = Msg;
    type InitParams = CopyQuestionnaireParams;

    fn init(params: CopyQuestionnaireParams) -> (State, Command<Msg>) {
        let state = State {
            questionnaire_id: params.questionnaire_id,
            questionnaire_name: params.questionnaire_name,
        };

        (state, Command::None)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::Back => {
                Command::batch(vec![
                    Command::navigate_to(AppId::SelectQuestionnaire),
                    Command::quit_self(),
                ])
            }
            Msg::StartCopy => {
                // TODO: Implement actual copy functionality
                log::info!("Copy functionality not yet implemented for questionnaire: {}", state.questionnaire_id);
                Command::None
            }
        }
    }

    fn view(state: &mut Self::State) -> LayeredView<Self::Msg> {
        let theme = &crate::global_runtime_config().theme;

        // Placeholder view
        let content = Element::column(vec![
            Element::text(format!("Questionnaire: {}", state.questionnaire_name)),
            Element::text(format!("ID: {}", state.questionnaire_id)),
            Element::text(""),
            Element::styled_text(Line::from(vec![
                Span::styled(
                    "Copy functionality will be implemented in future iterations.",
                    Style::default().fg(theme.text_secondary).italic(),
                ),
            ])).build(),
            Element::text(""),
            Element::styled_text(Line::from(vec![
                Span::styled(
                    "Press ESC to go back",
                    Style::default().fg(theme.text_tertiary),
                ),
            ])).build(),
        ])
        .build();

        let panel = Element::panel(content)
            .title("Copy Questionnaire")
            .build();

        LayeredView::new(panel)
    }

    fn subscriptions(_state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Back to selection", Msg::Back),
            // TODO: Add keybinding for starting copy when implemented
            // Subscription::keyboard(KeyCode::Char('c'), "Start copy", Msg::StartCopy),
        ]
    }

    fn title() -> &'static str {
        "Copy Questionnaire"
    }

    fn status(state: &Self::State) -> Option<Line<'static>> {
        let theme = &crate::global_runtime_config().theme;
        Some(Line::from(vec![
            Span::styled(
                state.questionnaire_name.clone(),
                Style::default().fg(theme.text_primary),
            ),
        ]))
    }
}
