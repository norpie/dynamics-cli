//! Operation Queue App

use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint, FocusId},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
    widgets::{TreeState, TreeEvent},
};
use crate::{col, row, use_constraints};
use crate::api::resilience::ResilienceConfig;
use ratatui::text::Line;
use std::collections::HashSet;
use super::models::{QueueItem, QueueFilter, SortMode, OperationStatus, QueueResult};
use super::tree_nodes::QueueTreeNode;

pub struct OperationQueueApp;

#[derive(Clone)]
pub enum Msg {
    // Tree interaction
    TreeEvent(TreeEvent),
    NodeSelected(String),
    ViewportHeight(usize),

    // Queue controls
    TogglePlay,
    StepOne,
    IncreasePriority(String),
    DecreasePriority(String),
    TogglePauseItem(String),
    DeleteItem(String),
    RetryItem(String),

    // Execution
    StartExecution(String),
    ExecutionCompleted(String, QueueResult),

    // Filters/Settings
    SetFilter(QueueFilter),
    SetSortMode(SortMode),
    SetMaxConcurrent(usize),

    // Navigation
    Back,
}

#[derive(Default)]
pub struct State {
    // Queue data
    queue_items: Vec<QueueItem>,
    tree_state: TreeState,

    // Execution state
    auto_play: bool,
    max_concurrent: usize,
    currently_running: HashSet<String>,

    // UI state
    filter: QueueFilter,
    sort_mode: SortMode,
}

impl crate::tui::AppState for State {}

impl App for OperationQueueApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        let state = State {
            queue_items: Vec::new(),
            tree_state: TreeState::with_selection(),
            auto_play: false,
            max_concurrent: 3,
            currently_running: HashSet::new(),
            filter: QueueFilter::All,
            sort_mode: SortMode::Priority,
        };

        (state, Command::set_focus(FocusId::new("queue-tree")))
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::TreeEvent(event) => {
                state.tree_state.handle_event(event);
                Command::None
            }

            Msg::NodeSelected(_id) => {
                // Could show details panel
                Command::None
            }

            Msg::ViewportHeight(height) => {
                state.tree_state.set_viewport_height(height);
                state.tree_state.update_scroll(height);
                Command::None
            }

            Msg::TogglePlay => {
                state.auto_play = !state.auto_play;
                if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                }
            }

            Msg::StepOne => {
                state.auto_play = false;
                execute_next_if_available(state)
            }

            Msg::IncreasePriority(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    if item.priority > 0 {
                        item.priority -= 1;
                    }
                }
                Command::None
            }

            Msg::DecreasePriority(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    if item.priority < 255 {
                        item.priority += 1;
                    }
                }
                Command::None
            }

            Msg::TogglePauseItem(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = match item.status {
                        OperationStatus::Pending => OperationStatus::Paused,
                        OperationStatus::Paused => OperationStatus::Pending,
                        _ => item.status.clone(),
                    };
                }
                Command::None
            }

            Msg::DeleteItem(id) => {
                state.queue_items.retain(|item| item.id != id);
                Command::None
            }

            Msg::RetryItem(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = OperationStatus::Pending;
                    item.result = None;
                }
                if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                }
            }

            Msg::StartExecution(id) => {
                // Mark as running
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = OperationStatus::Running;
                    state.currently_running.insert(id.clone());
                }

                // Get item for execution
                let item = state.queue_items.iter().find(|i| i.id == id).cloned();

                if let Some(item) = item {
                    Command::perform(
                        async move {
                            let start = std::time::Instant::now();

                            // Get client for this environment from global client manager
                            let client = match crate::client_manager().get_client(&item.metadata.environment_name).await {
                                Ok(client) => client,
                                Err(e) => {
                                    let duration_ms = start.elapsed().as_millis() as u64;
                                    return (item.id.clone(), QueueResult {
                                        success: false,
                                        operation_results: vec![],
                                        error: Some(format!("Failed to get client: {}", e)),
                                        duration_ms,
                                    });
                                }
                            };

                            let resilience = ResilienceConfig::default();
                            let result = item.operations.execute(&client, &resilience).await;
                            let duration_ms = start.elapsed().as_millis() as u64;

                            let queue_result = match result {
                                Ok(operation_results) => QueueResult {
                                    success: operation_results.iter().all(|r| r.success),
                                    operation_results,
                                    error: None,
                                    duration_ms,
                                },
                                Err(e) => QueueResult {
                                    success: false,
                                    operation_results: vec![],
                                    error: Some(e.to_string()),
                                    duration_ms,
                                },
                            };

                            (item.id.clone(), queue_result)
                        },
                        |(id, result)| Msg::ExecutionCompleted(id, result),
                    )
                } else {
                    Command::None
                }
            }

            Msg::ExecutionCompleted(id, result) => {
                state.currently_running.remove(&id);

                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = if result.success {
                        OperationStatus::Done
                    } else {
                        OperationStatus::Failed
                    };
                    item.result = Some(result.clone());

                    // Log completion
                    log::info!(
                        "Queue item {} completed: success={}, duration={}ms",
                        id,
                        result.success,
                        result.duration_ms
                    );
                }

                // Continue if auto-play
                if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                }
            }

            Msg::SetFilter(filter) => {
                state.filter = filter;
                Command::None
            }

            Msg::SetSortMode(sort_mode) => {
                state.sort_mode = sort_mode;
                Command::None
            }

            Msg::SetMaxConcurrent(max) => {
                state.max_concurrent = max;
                Command::None
            }

            Msg::Back => Command::navigate_to(AppId::AppLauncher),
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        use_constraints!();

        // Build tree nodes from filtered queue items
        let mut filtered_items: Vec<QueueItem> = state
            .queue_items
            .iter()
            .filter(|item| state.filter.matches(item))
            .cloned()
            .collect();

        // Sort items
        filtered_items.sort_by(|a, b| match state.sort_mode {
            SortMode::Priority => a.priority.cmp(&b.priority),
            SortMode::Status => {
                format!("{:?}", a.status).cmp(&format!("{:?}", b.status))
            }
            SortMode::Source => a.metadata.source.cmp(&b.metadata.source),
        });

        let tree_nodes: Vec<QueueTreeNode> = filtered_items
            .into_iter()
            .map(QueueTreeNode::Parent)
            .collect();

        // Controls row
        let play_button = if state.auto_play {
            Element::button("pause-btn", "⏸ Pause").on_press(Msg::TogglePlay)
        } else {
            Element::button("play-btn", "▶ Play").on_press(Msg::TogglePlay)
        }
        .build();

        let step_button = Element::button("step-btn", "→ Step")
            .on_press(Msg::StepOne)
            .build();

        let max_concurrent_text =
            Element::text(format!("Max Concurrent: {}", state.max_concurrent));

        let controls = row![
            play_button => Length(12),
            step_button => Length(10),
            max_concurrent_text => Length(20),
            Element::None => Fill(1),
        ];

        // Stats row
        let count_by_status = |status: OperationStatus| {
            state
                .queue_items
                .iter()
                .filter(|item| item.status == status)
                .count()
        };

        let stats = Element::text(format!(
            "Total: {}  Pending: {}  Running: {}  Paused: {}  Done: {}  Failed: {}",
            state.queue_items.len(),
            count_by_status(OperationStatus::Pending),
            state.currently_running.len(),
            count_by_status(OperationStatus::Paused),
            count_by_status(OperationStatus::Done),
            count_by_status(OperationStatus::Failed),
        ));

        // Filter row (simplified for now)
        let filter_text = Element::text(format!(
            "Filter: {}  Sort: {}",
            state.filter.label(),
            state.sort_mode.label()
        ));

        // Table tree
        let tree = Element::table_tree("queue-tree", &tree_nodes, &mut state.tree_state)
            .on_event(Msg::TreeEvent)
            .on_select(Msg::NodeSelected)
            .on_render(Msg::ViewportHeight)
            .build();

        let content = col![
            controls => Length(3),
            stats => Length(1),
            filter_text => Length(1),
            tree => Fill(1),
        ];

        LayeredView::new(Element::panel(content).title("Operation Queue").build())
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Operation Queue"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<Line<'static>> {
        None
    }
}

/// Helper function to execute the next available operation
fn execute_next_if_available(state: &mut State) -> Command<Msg> {
    // Check if we can run more
    if state.currently_running.len() >= state.max_concurrent {
        return Command::None;
    }

    // Find next pending (not paused) item by priority
    let next = state
        .queue_items
        .iter()
        .filter(|item| item.status == OperationStatus::Pending)
        .min_by_key(|item| item.priority)
        .map(|item| item.id.clone());

    if let Some(id) = next {
        // Mark as running immediately
        if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
            item.status = OperationStatus::Running;
            state.currently_running.insert(id.clone());
        }

        // Get item for execution
        let item = state.queue_items.iter().find(|i| i.id == id).cloned();

        if let Some(item) = item {
            Command::perform(
                async move {
                    let start = std::time::Instant::now();

                    // Get client for this environment from global client manager
                    let client = match crate::client_manager().get_client(&item.metadata.environment_name).await {
                        Ok(client) => client,
                        Err(e) => {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            return (item.id.clone(), QueueResult {
                                success: false,
                                operation_results: vec![],
                                error: Some(format!("Failed to get client: {}", e)),
                                duration_ms,
                            });
                        }
                    };

                    let resilience = ResilienceConfig::default();
                    let result = item.operations.execute(&client, &resilience).await;
                    let duration_ms = start.elapsed().as_millis() as u64;

                    let queue_result = match result {
                        Ok(operation_results) => QueueResult {
                            success: operation_results.iter().all(|r| r.success),
                            operation_results,
                            error: None,
                            duration_ms,
                        },
                        Err(e) => QueueResult {
                            success: false,
                            operation_results: vec![],
                            error: Some(e.to_string()),
                            duration_ms,
                        },
                    };

                    (item.id.clone(), queue_result)
                },
                |(id, result)| Msg::ExecutionCompleted(id, result),
            )
        } else {
            Command::None
        }
    } else {
        Command::None
    }
}
