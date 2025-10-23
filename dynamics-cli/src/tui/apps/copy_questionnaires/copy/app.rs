use super::models::*;
use super::data_loading::{load_full_snapshot, build_domain_model};
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
            questionnaire: Resource::Loading,
            tree_state: crate::tui::widgets::TreeState::with_selection(),
        };

        // Load complete questionnaire snapshot - single task that loads everything sequentially
        let questionnaire_id = params.questionnaire_id.clone();
        let cmd = Command::perform_parallel()
            .add_task(
                "Loading questionnaire structure",
                async move {
                    let snapshot = load_full_snapshot(&questionnaire_id).await?;
                    build_domain_model(snapshot)
                }
            )
            .with_title("Loading Questionnaire Data")
            .on_complete(AppId::CopyQuestionnaire)
            .build(|_task_idx, result| {
                let data = result.downcast::<Result<super::domain::Questionnaire, String>>().unwrap();
                Msg::QuestionnaireLoaded(*data)
            });

        (state, cmd)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::QuestionnaireLoaded(result) => {
                match result {
                    Ok(questionnaire) => {
                        log::info!("Successfully loaded questionnaire with {} total entities", questionnaire.total_entities());
                        state.questionnaire = Resource::Success(questionnaire);
                    }
                    Err(e) => {
                        log::error!("Failed to load questionnaire: {}", e);
                        state.questionnaire = Resource::Failure(e);
                    }
                }
                Command::None
            }
            Msg::TreeEvent(event) => {
                state.tree_state.handle_event(event);
                Command::None
            }
            Msg::TreeNodeClicked(node_id) => {
                // Handle tree node clicks (e.g., select node, expand/collapse)
                log::debug!("Tree node clicked: {}", node_id);
                // TODO: Add specific behavior when nodes are clicked
                Command::None
            }
            Msg::ViewportHeight(height) => {
                // Renderer provides viewport height for proper scrolling
                state.tree_state.set_viewport_height(height);
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
