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
use std::collections::HashMap;

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
            id_map: HashMap::new(),
            created_ids: Vec::new(),
            start_time: None,
        };

        (state, Command::None)
    }

    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg> {
        match msg {
            Msg::StartCopy => {
                log::info!("Starting copy operation");

                // Initialize state
                state.start_time = Some(std::time::Instant::now());
                state.id_map.clear();
                state.created_ids.clear();

                // Transition to copying state
                state.push_state = PushState::Copying(CopyProgress::new(&state.questionnaire));

                // Start Step 1
                let questionnaire = state.questionnaire.clone();
                let copy_name = state.copy_name.clone();
                let copy_code = state.copy_code.clone();

                Command::perform(
                    super::step_commands::step1_create_questionnaire(questionnaire, copy_name, copy_code),
                    |result| Msg::Step1Complete(result.map(|(id, _)| id))
                )
            }

            Msg::Step1Complete(result) => {
                match result {
                    Ok(new_q_id) => {
                        // Update id map and created_ids
                        state.id_map.insert(state.questionnaire_id.clone(), new_q_id.clone());
                        state.created_ids.push(("nrq_questionnaires".to_string(), new_q_id));

                        // Update progress
                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingPages;
                            progress.step = 2;
                            progress.questionnaire = (1, 1);
                            progress.total_created = 1;
                        }

                        // Start Step 2
                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step2_create_pages(questionnaire, id_map, created_ids),
                            Msg::Step2Complete  // Keep full tuple
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step2Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        // Update state with new mappings
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        // Update progress
                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingPageLines;
                            progress.step = 3;
                            progress.pages = (progress.pages.1, progress.pages.1);
                            progress.total_created += progress.pages.1;
                        }

                        // Start Step 3
                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step3_create_page_lines(questionnaire, id_map, created_ids),
                            Msg::Step3Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step3Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingGroups;
                            progress.step = 4;
                            progress.page_lines = (progress.page_lines.1, progress.page_lines.1);
                            progress.total_created += progress.page_lines.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step4_create_groups(questionnaire, id_map, created_ids),
                            Msg::Step4Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step4Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingGroupLines;
                            progress.step = 5;
                            progress.groups = (progress.groups.1, progress.groups.1);
                            progress.total_created += progress.groups.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step5_create_group_lines(questionnaire, id_map, created_ids),
                            Msg::Step5Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step6Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingTemplateLines;
                            progress.step = 7;
                            progress.questions = (progress.questions.1, progress.questions.1);
                            progress.total_created += progress.questions.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step7_create_template_lines(questionnaire, id_map, created_ids),
                            Msg::Step7Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step7Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingConditions;
                            progress.step = 8;
                            progress.template_lines = (progress.template_lines.1, progress.template_lines.1);
                            progress.total_created += progress.template_lines.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step8_create_conditions(questionnaire, id_map, created_ids),
                            Msg::Step8Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step8Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingConditionActions;
                            progress.step = 9;
                            progress.conditions = (progress.conditions.1, progress.conditions.1);
                            progress.total_created += progress.conditions.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step9_create_condition_actions(questionnaire, id_map, created_ids),
                            Msg::Step9Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step9Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingClassifications;
                            progress.step = 10;
                            progress.condition_actions = (progress.condition_actions.1, progress.condition_actions.1);
                            progress.total_created += progress.condition_actions.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step10_create_classifications(questionnaire, id_map, created_ids),
                            Msg::Step10Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step10Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        // Calculate final statistics
                        let new_questionnaire_id = state.id_map.get(&state.questionnaire_id)
                            .cloned()
                            .unwrap_or_else(|| "unknown".to_string());

                        let mut entities_created = HashMap::new();
                        for (entity_set, _) in &state.created_ids {
                            *entities_created.entry(entity_set.clone()).or_insert(0) += 1;
                        }

                        let total_entities = state.created_ids.len();
                        let duration = state.start_time
                            .map(|t| t.elapsed())
                            .unwrap_or_default();

                        // Transition to success state
                        state.push_state = PushState::Success(CopyResult {
                            new_questionnaire_id,
                            new_questionnaire_name: state.copy_name.clone(),
                            entities_created,
                            total_entities,
                            duration,
                        });

                        Command::None
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
            }

            Msg::Step5Complete(result) => {
                match result {
                    Ok((new_id_map, new_created_ids)) => {
                        state.id_map = new_id_map;
                        state.created_ids = new_created_ids;

                        if let PushState::Copying(ref mut progress) = state.push_state {
                            progress.phase = CopyPhase::CreatingQuestions;
                            progress.step = 6;
                            progress.group_lines = (progress.group_lines.1, progress.group_lines.1);
                            progress.total_created += progress.group_lines.1;
                        }

                        let questionnaire = state.questionnaire.clone();
                        let id_map = state.id_map.clone();
                        let created_ids = state.created_ids.clone();

                        Command::perform(
                            super::step_commands::step6_create_questions(questionnaire, id_map, created_ids),
                            Msg::Step6Complete
                        )
                    }
                    Err(error) => {
                        state.push_state = PushState::Failed(error);
                        Command::None
                    }
                }
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
