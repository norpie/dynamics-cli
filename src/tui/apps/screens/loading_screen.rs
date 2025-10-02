use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use serde_json::Value;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint};
use crate::tui::element::ColumnBuilder;

pub struct LoadingScreen;

#[derive(Clone)]
pub enum Msg {
    Initialize(Value),
    TaskProgress(Value),
    Tick,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Clone, Debug)]
pub struct LoadingTask {
    pub name: String,
    pub status: TaskStatus,
}

#[derive(Default)]
pub struct State {
    tasks: Vec<LoadingTask>,
    target_app: Option<AppId>,
    caller_app: Option<AppId>,
    cancellable: bool,
    spinner_state: usize,
    countdown_ticks: Option<usize>, // Number of ticks remaining before navigation (80ms per tick)
    initialized: bool, // Prevents stale events from affecting state before Initialize runs
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl App for LoadingScreen {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize(data) => {
                use rand::Rng;

                // IMPORTANT: Reset ALL state from previous runs
                // This prevents old countdown/state from interfering with new loads
                *state = State::default();

                // Parse initialization data
                let task_names: Vec<String> = if let Some(tasks_json) = data.get("tasks").and_then(|v| v.as_array()) {
                    tasks_json
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                } else {
                    Vec::new()
                };

                state.tasks = task_names
                    .iter()
                    .map(|name| LoadingTask {
                        name: name.clone(),
                        status: TaskStatus::Pending,
                    })
                    .collect();

                state.target_app = data
                    .get("target")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Example1" => Some(AppId::Example1),
                        "Example2" => Some(AppId::Example2),
                        "ErrorScreen" => Some(AppId::ErrorScreen),
                        "MigrationEnvironment" => Some(AppId::MigrationEnvironment),
                        "MigrationComparisonSelect" => Some(AppId::MigrationComparisonSelect),
                        _ => None,
                    });

                state.caller_app = data
                    .get("caller")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "Example1" => Some(AppId::Example1),
                        "Example2" => Some(AppId::Example2),
                        "ErrorScreen" => Some(AppId::ErrorScreen),
                        "MigrationEnvironment" => Some(AppId::MigrationEnvironment),
                        "MigrationComparisonSelect" => Some(AppId::MigrationComparisonSelect),
                        _ => None,
                    });

                state.cancellable = data.get("cancellable").and_then(|v| v.as_bool()).unwrap_or(false);

                // Check if tasks should auto-complete (default: true)
                // Set to false if you want to control task progress externally
                let auto_complete = data.get("auto_complete")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let mut commands = Vec::new();

                if auto_complete {
                    // Spawn async work for each task with random delays
                    let mut rng = rand::thread_rng();

                    for task_name in task_names {
                        let delay_secs = rng.gen_range(1..=5);
                        let task_name_clone = task_name.clone();

                        commands.push(Command::perform(
                            async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;

                                // Mark task as completed
                                serde_json::json!({
                                    "task": task_name_clone,
                                    "status": "Completed",
                                })
                            },
                            |data| Msg::TaskProgress(data),
                        ));

                        // Also immediately send InProgress status
                        commands.push(Command::Publish {
                            topic: "loading:progress".to_string(),
                            data: serde_json::json!({
                                "task": task_name,
                                "status": "InProgress",
                            }),
                        });
                    }
                }

                // Mark as initialized AFTER all setup is complete
                state.initialized = true;

                Command::Batch(commands)
            }

            Msg::TaskProgress(data) => {
                // Ignore stale events from previous loads
                if !state.initialized {
                    return Command::None;
                }
                let task_name = data.get("task").and_then(|v| v.as_str()).unwrap_or("");
                let status_str = data.get("status").and_then(|v| v.as_str()).unwrap_or("");

                if let Some(task) = state.tasks.iter_mut().find(|t| t.name == task_name) {
                    task.status = match status_str {
                        "InProgress" => TaskStatus::InProgress,
                        "Completed" => TaskStatus::Completed,
                        "Failed" => TaskStatus::Failed(
                            data.get("error")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Unknown error")
                                .to_string()
                        ),
                        _ => TaskStatus::Pending,
                    };
                }

                // Check if all tasks are complete
                // IMPORTANT: Only start countdown if we have tasks AND they're all done
                // This prevents edge cases where empty task list triggers immediate countdown
                let all_done = !state.tasks.is_empty() && state.tasks.iter().all(|t| {
                    matches!(t.status, TaskStatus::Completed | TaskStatus::Failed(_))
                });

                if all_done && state.countdown_ticks.is_none() {
                    // Start countdown: 1000ms / 80ms per tick = 12.5 ticks, round to 13
                    state.countdown_ticks = Some(13);
                }

                Command::None
            }

            Msg::Tick => {
                state.spinner_state = (state.spinner_state + 1) % SPINNER_FRAMES.len();

                // Handle countdown only if initialized
                if state.initialized {
                    if let Some(remaining) = state.countdown_ticks {
                        if remaining <= 1 {
                            // Only navigate if we have tasks (prevents stale countdown from navigating)
                            if !state.tasks.is_empty() {
                                if let Some(target) = state.target_app {
                                    return Command::navigate_to(target);
                                }
                            }
                        } else {
                            state.countdown_ticks = Some(remaining - 1);
                        }
                    }
                }

                Command::None
            }

            Msg::Cancel => {
                if let Some(caller) = state.caller_app {
                    // Notify caller to cancel work
                    let topic = format!("loading:cancel:{:?}", caller);
                    Command::Batch(vec![
                        Command::Publish {
                            topic,
                            data: serde_json::json!({}),
                        },
                        Command::navigate_to(caller),
                    ])
                } else {
                    Command::None
                }
            }

        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let mut content = vec![];

        // Header
        let all_done = state.tasks.iter().all(|t| {
            matches!(t.status, TaskStatus::Completed | TaskStatus::Failed(_))
        });

        let header_text = if state.countdown_ticks.is_some() {
            "All tasks completed! Returning in 1 second...".to_string()
        } else if all_done {
            "All tasks completed!".to_string()
        } else {
            "Loading...".to_string()
        };

        content.push(Element::styled_text(Line::from(vec![
            Span::styled(SPINNER_FRAMES[state.spinner_state], Style::default().fg(theme.sky).bold()),
            Span::raw(" "),
            Span::styled(header_text.clone(), Style::default().fg(theme.sky)),
        ])).build());
        content.push(Element::text(""));

        // Tasks
        for task in &state.tasks {
            let (symbol, color) = match &task.status {
                TaskStatus::Pending => ("◯", theme.overlay1),
                TaskStatus::InProgress => (SPINNER_FRAMES[state.spinner_state], theme.sky),
                TaskStatus::Completed => ("✓", theme.green),
                TaskStatus::Failed(_) => ("❌", theme.red),
            };

            content.push(Element::styled_text(Line::from(vec![
                Span::styled(format!(" {} ", symbol), Style::default().fg(color)),
                Span::styled(task.name.clone(), Style::default().fg(color)),
            ])).build());
        }

        content.push(Element::text(""));

        // Footer
        let footer_text = if all_done {
            "Press any key to continue..."
        } else if state.cancellable {
            "Press ESC to cancel..."
        } else {
            "Please wait..."
        };

        content.push(Element::styled_text(Line::from(
            Span::styled(footer_text, Style::default().fg(theme.overlay1))
        )).build());

        // Wrap in panel
        Element::panel(
            Element::container(
                ColumnBuilder::new()
                    .add(Element::column(content).build(), LayoutConstraint::Fill(1))
                    .build()
            )
            .padding(2)
            .build()
        )
        .title("Loading Tasks")
        .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::subscribe("loading:init", |data| Some(Msg::Initialize(data))),
            Subscription::subscribe("loading:progress", |data| Some(Msg::TaskProgress(data))),
            Subscription::timer(std::time::Duration::from_millis(80), Msg::Tick),
            Subscription::keyboard(KeyCode::Esc, "Cancel loading", Msg::Cancel),
        ]
    }

    fn title() -> &'static str {
        "Loading"
    }
}
