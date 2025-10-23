use super::models::*;
use super::view;
use crate::tui::{
    app::App,
    command::{AppId, Command},
    subscription::Subscription,
    renderer::LayeredView,
};
use crossterm::event::KeyCode;
use ratatui::text::Line;

pub struct PushQuestionnaireApp;

impl crate::tui::AppState for State {}

impl App for PushQuestionnaireApp {
    type State = State;
    type Msg = Msg;
    type InitParams = PushQuestionnaireParams;

    fn init(params: PushQuestionnaireParams) -> (State, Command<Msg>) {
        let state = State {
            questionnaire_id: params.questionnaire_id,
            copy_name: params.copy_name,
        };

        (state, Command::None)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::Back => {
                Command::batch(vec![
                    Command::navigate_to(AppId::CopyQuestionnaire),
                    Command::quit_self(),
                ])
            }
        }
    }

    fn view(state: &mut Self::State) -> LayeredView<Self::Msg> {
        view::render_view(state)
    }

    fn subscriptions(_state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Back", Msg::Back),
        ]
    }

    fn title() -> &'static str {
        "Push Questionnaire"
    }

    fn status(state: &Self::State) -> Option<Line<'static>> {
        view::render_status(state)
    }
}
