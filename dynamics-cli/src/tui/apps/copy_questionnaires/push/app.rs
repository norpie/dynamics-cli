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
            copy_code: params.copy_code,
            questionnaire: params.questionnaire,
            push_state: PushState::Confirming,
        };

        (state, Command::None)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::StartCopy => {
                // TODO: Implement actual copy logic
                // For now, simulate immediate progress
                log::info!("Starting copy operation (stub)");
                state.push_state = PushState::Copying(CopyProgress::new(&state.questionnaire));
                Command::None
            }

            Msg::CopyProgressUpdate(progress) => {
                // Update progress from async task
                state.push_state = PushState::Copying(progress);
                Command::None
            }

            Msg::CopySuccess(result) => {
                // Copy completed successfully
                state.push_state = PushState::Success(result);
                Command::None
            }

            Msg::CopyFailed(error) => {
                // Copy failed
                state.push_state = PushState::Failed(error);
                Command::None
            }

            Msg::Cancel | Msg::Back => {
                Command::batch(vec![
                    Command::navigate_to(AppId::CopyQuestionnaire),
                    Command::quit_self(),
                ])
            }

            Msg::Done => {
                // Success - go back to questionnaire list
                Command::batch(vec![
                    Command::navigate_to(AppId::SelectQuestionnaire),
                    Command::quit_self(),
                ])
            }

            Msg::Retry => {
                // Reset to confirmation screen
                state.push_state = PushState::Confirming;
                Command::None
            }

            Msg::ViewCopy => {
                // TODO: Navigate to view the newly created questionnaire
                log::info!("View copy (not implemented)");
                Command::None
            }

            Msg::CopyAnother => {
                // Go back to select questionnaire screen
                Command::batch(vec![
                    Command::navigate_to(AppId::SelectQuestionnaire),
                    Command::quit_self(),
                ])
            }

            Msg::ViewLogs => {
                // TODO: Show detailed error logs
                log::info!("View logs (not implemented)");
                Command::None
            }
        }
    }

    fn view(state: &mut Self::State) -> LayeredView<Self::Msg> {
        view::render_view(state)
    }

    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>> {
        match &state.push_state {
            PushState::Confirming => {
                vec![
                    Subscription::keyboard(KeyCode::Enter, "Start Copy", Msg::StartCopy),
                    Subscription::keyboard(KeyCode::Esc, "Cancel", Msg::Cancel),
                ]
            }
            PushState::Copying(_) => {
                // No user input during copy
                vec![]
            }
            PushState::Success(_) => {
                vec![
                    Subscription::keyboard(KeyCode::Enter, "Done", Msg::Done),
                    Subscription::keyboard(KeyCode::Char('c'), "Copy Another", Msg::CopyAnother),
                    Subscription::keyboard(KeyCode::Char('v'), "View Copy", Msg::ViewCopy),
                ]
            }
            PushState::Failed(_) => {
                vec![
                    Subscription::keyboard(KeyCode::Char('r'), "Retry", Msg::Retry),
                    Subscription::keyboard(KeyCode::Char('l'), "View Logs", Msg::ViewLogs),
                    Subscription::keyboard(KeyCode::Esc, "Cancel", Msg::Cancel),
                ]
            }
        }
    }

    fn title() -> &'static str {
        "Push Questionnaire"
    }

    fn status(state: &Self::State) -> Option<Line<'static>> {
        view::render_status(state)
    }
}
