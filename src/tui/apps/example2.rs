use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme};

pub struct Example2;

#[derive(Clone)]
pub enum Msg {
    RequestNavigate,
    ConfirmNavigate,
    CancelNavigate,
    ButtonHovered,
    ButtonUnhovered,
    StartLoading,
    LoadButtonHovered,
    LoadButtonUnhovered,
    StartFailingLoad,
    FailLoadButtonHovered,
    FailLoadButtonUnhovered,
    Task1Complete,
    Task2Failed,
    CancelLoading,
}

#[derive(Default)]
pub struct State {
    button_hovered: bool,
    show_confirm: bool,
    load_button_hovered: bool,
    fail_load_button_hovered: bool,
}

impl App for Example2 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::RequestNavigate => {
                state.show_confirm = true;
                // Clear hover state when modal opens
                state.button_hovered = false;
                Command::None
            }
            Msg::ConfirmNavigate => {
                state.show_confirm = false;
                Command::navigate_to(AppId::Example1)
            }
            Msg::CancelNavigate => {
                state.show_confirm = false;
                Command::None
            }
            Msg::ButtonHovered => {
                state.button_hovered = true;
                Command::None
            }
            Msg::ButtonUnhovered => {
                state.button_hovered = false;
                Command::None
            }
            Msg::LoadButtonHovered => {
                state.load_button_hovered = true;
                Command::None
            }
            Msg::LoadButtonUnhovered => {
                state.load_button_hovered = false;
                Command::None
            }
            Msg::StartLoading => {
                // Initialize loading screen with 3 tasks
                // LoadingScreen will spawn async work and handle progress itself
                let init_data = serde_json::json!({
                    "tasks": ["Fetching data", "Processing records", "Building cache"],
                    "target": "Example2",
                    "caller": "Example2",
                    "cancellable": true,
                });

                Command::Batch(vec![
                    Command::Publish {
                        topic: "loading:init".to_string(),
                        data: init_data,
                    },
                    Command::navigate_to(AppId::LoadingScreen),
                ])
            }
            Msg::StartFailingLoad => {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Initialize loading screen with 2 tasks
                // auto_complete: false means we control the task progress externally
                let init_data = serde_json::json!({
                    "tasks": ["Connecting to server", "Authenticating"],
                    "target": "ErrorScreen",
                    "caller": "Example2",
                    "cancellable": false,
                    "auto_complete": false,
                });

                let delay1 = rng.gen_range(1..=3);
                let delay2 = rng.gen_range(2..=4);

                Command::Batch(vec![
                    Command::Publish {
                        topic: "loading:init".to_string(),
                        data: init_data,
                    },
                    Command::navigate_to(AppId::LoadingScreen),
                    // Immediately mark tasks as InProgress
                    Command::Publish {
                        topic: "loading:progress".to_string(),
                        data: serde_json::json!({
                            "task": "Connecting to server",
                            "status": "InProgress",
                        }),
                    },
                    Command::Publish {
                        topic: "loading:progress".to_string(),
                        data: serde_json::json!({
                            "task": "Authenticating",
                            "status": "InProgress",
                        }),
                    },
                    // Task 1: Connecting to server
                    Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay1)).await;
                        },
                        |_| Msg::Task1Complete
                    ),
                    // Task 2: Authenticating (will fail)
                    Command::perform(
                        async move {
                            tokio::time::sleep(tokio::time::Duration::from_secs(delay2)).await;
                        },
                        |_| Msg::Task2Failed
                    ),
                ])
            }
            Msg::Task1Complete => {
                // Mark task 1 as completed
                Command::Publish {
                    topic: "loading:progress".to_string(),
                    data: serde_json::json!({
                        "task": "Connecting to server",
                        "status": "Completed",
                    }),
                }
            }
            Msg::Task2Failed => {
                // Mark task 2 as failed and initialize error screen
                Command::Batch(vec![
                    Command::Publish {
                        topic: "loading:progress".to_string(),
                        data: serde_json::json!({
                            "task": "Authenticating",
                            "status": "Failed",
                            "error": "Invalid credentials",
                        }),
                    },
                    Command::Publish {
                        topic: "error:init".to_string(),
                        data: serde_json::json!({
                            "message": "Failed to authenticate: Invalid credentials",
                            "target": "Example2",
                        }),
                    },
                ])
            }
            Msg::FailLoadButtonHovered => {
                state.fail_load_button_hovered = true;
                Command::None
            }
            Msg::FailLoadButtonUnhovered => {
                state.fail_load_button_hovered = false;
                Command::None
            }
            Msg::CancelLoading => {
                // Handle cancellation - just a no-op for now
                Command::None
            }
        }
    }

    fn view(state: &State, theme: &Theme) -> Element<Msg> {
        let button_style = if state.button_hovered {
            ratatui::style::Style::default().fg(theme.lavender)
        } else {
            ratatui::style::Style::default().fg(theme.text)
        };

        let load_button_style = if state.load_button_hovered {
            ratatui::style::Style::default().fg(theme.lavender)
        } else {
            ratatui::style::Style::default().fg(theme.text)
        };

        let fail_load_button_style = if state.fail_load_button_hovered {
            ratatui::style::Style::default().fg(theme.lavender)
        } else {
            ratatui::style::Style::default().fg(theme.text)
        };

        let main_ui = Element::column(vec![
            Element::text("Example 2 - Modal Confirmation Demo!"),
            Element::text(""),
            Element::button("[ Press 1 or click to go to Example 1 ]")
                .on_press(Msg::RequestNavigate)
                .on_hover(Msg::ButtonHovered)
                .on_hover_exit(Msg::ButtonUnhovered)
                .style(button_style)
                .build(),
            Element::text(""),
            Element::button("[ Press L to load data (cancellable) ]")
                .on_press(Msg::StartLoading)
                .on_hover(Msg::LoadButtonHovered)
                .on_hover_exit(Msg::LoadButtonUnhovered)
                .style(load_button_style)
                .build(),
            Element::text(""),
            Element::button("[ Press F to fail loading (uncancellable) ]")
                .on_press(Msg::StartFailingLoad)
                .on_hover(Msg::FailLoadButtonHovered)
                .on_hover_exit(Msg::FailLoadButtonUnhovered)
                .style(fail_load_button_style)
                .build(),
            Element::text(""),
            Element::text("Now with confirmation modal!"),
            Element::text("Stack/Layer system in action."),
            Element::text(""),
            Element::text("Try navigating - you'll see a modal popup."),
        ])
        .build();

        if state.show_confirm {
            Element::modal_confirm(
                main_ui,
                "Confirm Navigation",
                "Are you sure you want to navigate to Example 1?",
                Msg::ConfirmNavigate,
                Msg::CancelNavigate,
            )
        } else {
            main_ui
        }
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(
                KeyCode::Char('1'),
                "Navigate to Example 1 (with confirmation)",
                Msg::RequestNavigate,
            ),
            Subscription::keyboard(
                KeyCode::Char('l'),
                "Load data (cancellable)",
                Msg::StartLoading,
            ),
            Subscription::keyboard(
                KeyCode::Char('L'),
                "Load data (cancellable)",
                Msg::StartLoading,
            ),
            Subscription::keyboard(
                KeyCode::Char('f'),
                "Fail loading (uncancellable)",
                Msg::StartFailingLoad,
            ),
            Subscription::keyboard(
                KeyCode::Char('F'),
                "Fail loading (uncancellable)",
                Msg::StartFailingLoad,
            ),
            Subscription::subscribe("loading:cancel:Example2", |_| Some(Msg::CancelLoading)),
        ]
    }

    fn title() -> &'static str {
        "Example 2"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        if state.show_confirm {
            Some(Line::from(Span::styled("[Confirm]", Style::default().fg(theme.peach))))
        } else {
            None
        }
    }
}