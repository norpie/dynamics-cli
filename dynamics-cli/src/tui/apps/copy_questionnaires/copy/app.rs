use super::models::*;
use super::data_loading::load_full_snapshot;
use super::view;
use crate::tui::{
    app::App,
    command::{AppId, Command},
    subscription::Subscription,
    renderer::LayeredView,
    Resource,
};
use crossterm::event::KeyCode;
use ratatui::text::Line;

pub struct CopyQuestionnaireApp;

impl crate::tui::AppState for State {}

impl App for CopyQuestionnaireApp {
    type State = State;
    type Msg = Msg;
    type InitParams = CopyQuestionnaireParams;

    fn init(params: CopyQuestionnaireParams) -> (State, Command<Msg>) {
        let mut state = State {
            questionnaire_id: params.questionnaire_id.clone(),
            questionnaire_name: params.questionnaire_name,
            snapshot: Resource::Loading,
        };

        // Load complete questionnaire snapshot - single task that loads everything sequentially
        let questionnaire_id = params.questionnaire_id.clone();
        let cmd = Command::perform_parallel()
            .add_task(
                "Loading questionnaire snapshot",
                async move {
                    load_full_snapshot(&questionnaire_id).await
                }
            )
            .with_title("Loading Questionnaire Data")
            .on_complete(AppId::CopyQuestionnaire)
            .build(|_task_idx, result| {
                let data = result.downcast::<Result<QuestionnaireSnapshot, String>>().unwrap();
                Msg::SnapshotLoaded(*data)
            });

        (state, cmd)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::SnapshotLoaded(result) => {
                match result {
                    Ok(snapshot) => {
                        log::info!("Successfully loaded questionnaire snapshot with {} total entities", snapshot.total_entities());
                        state.snapshot = Resource::Success(snapshot);
                    }
                    Err(e) => {
                        log::error!("Failed to load questionnaire snapshot: {}", e);
                        state.snapshot = Resource::Failure(e);
                    }
                }
                Command::None
            }
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
        view::render_view(state)
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
        view::render_status(state)
    }
}
