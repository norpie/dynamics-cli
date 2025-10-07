//! Operation Queue App

use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint, FocusId},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
    widgets::{TreeState, TreeEvent, ScrollableState},
};
use crate::{col, row, use_constraints};
use crate::api::resilience::ResilienceConfig;
use ratatui::text::Line;
use std::collections::{HashSet, VecDeque};
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

    // Keyboard shortcuts for selected item
    IncreasePrioritySelected,
    DecreasePrioritySelected,
    TogglePauseSelected,
    DeleteSelected,
    RetrySelected,

    // Queue management
    AddItems(Vec<QueueItem>),
    ClearQueue,

    // Execution
    StartExecution(String),
    ExecutionCompleted(String, QueueResult),

    // Filters/Settings
    SetFilter(QueueFilter),
    SetSortMode(SortMode),
    SetMaxConcurrent(usize),

    // Details panel scrolling
    DetailsScroll(crossterm::event::KeyCode),
    DetailsSetDimensions(usize, usize),  // (viewport_height, content_height)

    // Navigation
    Back,
}

pub struct State {
    // Queue data
    queue_items: Vec<QueueItem>,
    tree_state: TreeState,

    // Execution state
    auto_play: bool,
    max_concurrent: usize,
    currently_running: HashSet<String>,

    // Performance tracking
    recent_completion_times: VecDeque<u64>, // Store last 10 completion times in ms

    // UI state
    filter: QueueFilter,
    sort_mode: SortMode,
    selected_item_id: Option<String>,
    details_scroll_state: ScrollableState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            queue_items: Vec::new(),
            tree_state: TreeState::with_selection(),
            auto_play: false,
            max_concurrent: 3,
            currently_running: HashSet::new(),
            recent_completion_times: VecDeque::with_capacity(10),
            filter: QueueFilter::All,
            sort_mode: SortMode::Priority,
            selected_item_id: None,
            details_scroll_state: ScrollableState::new(),
        }
    }
}

impl crate::tui::AppState for State {}

impl App for OperationQueueApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        (State::default(), Command::set_focus(FocusId::new("queue-tree")))
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::TreeEvent(event) => {
                let old_selected = state.selected_item_id.clone();
                state.tree_state.handle_event(event);
                // Update selected item when navigating (not just on Enter)
                let new_selected = state.tree_state.selected().map(|s| s.to_string());

                // Reset scroll state when selection changes
                if old_selected != new_selected {
                    state.details_scroll_state = ScrollableState::new();
                }

                state.selected_item_id = new_selected;
                Command::None
            }

            Msg::NodeSelected(id) => {
                // Reset scroll state when selecting a new item
                if state.selected_item_id.as_ref() != Some(&id) {
                    state.details_scroll_state = ScrollableState::new();
                }
                state.selected_item_id = Some(id);
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
                    item.started_at = None;
                }
                if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                }
            }

            Msg::StartExecution(id) => {
                // Mark as running and set start time
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = OperationStatus::Running;
                    item.started_at = Some(std::time::Instant::now());
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

                let mut publish_cmd = Command::None;

                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = if result.success {
                        OperationStatus::Done
                    } else {
                        OperationStatus::Failed
                    };
                    item.result = Some(result.clone());

                    // Track completion time for successful operations
                    if result.success {
                        state.recent_completion_times.push_back(result.duration_ms);
                        // Keep only last 10 completion times
                        if state.recent_completion_times.len() > 10 {
                            state.recent_completion_times.pop_front();
                        }
                    }

                    // Log completion with error details
                    if result.success {
                        log::info!(
                            "✓ Queue item {} completed successfully: duration={}ms",
                            id,
                            result.duration_ms
                        );
                    } else {
                        log::error!(
                            "✗ Queue item {} FAILED: duration={}ms",
                            id,
                            result.duration_ms
                        );

                        // Log top-level error if present
                        if let Some(ref error) = result.error {
                            log::error!("  Error: {}", error);
                        }

                        // Log individual operation failures
                        for (idx, op_result) in result.operation_results.iter().enumerate() {
                            if !op_result.success {
                                log::error!(
                                    "  Operation {} failed: {} on entity '{}' (status: {})",
                                    idx + 1,
                                    op_result.operation.operation_type(),
                                    op_result.operation.entity(),
                                    op_result.status_code.map(|s| s.to_string()).unwrap_or_else(|| "unknown".to_string())
                                );
                                if let Some(ref err) = op_result.error {
                                    log::error!("    Details: {}", err);
                                }
                            }
                        }
                    }

                    // Publish completion event for subscribers
                    let completion_data = serde_json::json!({
                        "id": id,
                        "result": result,
                        "metadata": item.metadata,
                    });
                    publish_cmd = Command::Publish {
                        topic: "queue:item_completed".to_string(),
                        data: completion_data,
                    };
                }

                // Continue if auto-play
                let next_cmd = if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                };

                Command::Batch(vec![publish_cmd, next_cmd])
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

            // Keyboard shortcuts operating on selected item
            Msg::IncreasePrioritySelected => {
                if let Some(id) = &state.selected_item_id {
                    if let Some(item) = state.queue_items.iter_mut().find(|i| &i.id == id) {
                        if item.priority > 0 {
                            item.priority -= 1;
                        }
                    }
                }
                Command::None
            }

            Msg::DecreasePrioritySelected => {
                if let Some(id) = &state.selected_item_id {
                    if let Some(item) = state.queue_items.iter_mut().find(|i| &i.id == id) {
                        if item.priority < 255 {
                            item.priority += 1;
                        }
                    }
                }
                Command::None
            }

            Msg::TogglePauseSelected => {
                if let Some(id) = &state.selected_item_id {
                    if let Some(item) = state.queue_items.iter_mut().find(|i| &i.id == id) {
                        item.status = match item.status {
                            OperationStatus::Pending => OperationStatus::Paused,
                            OperationStatus::Paused => OperationStatus::Pending,
                            _ => item.status.clone(),
                        };
                    }
                }
                Command::None
            }

            Msg::DeleteSelected => {
                if let Some(id) = &state.selected_item_id {
                    state.queue_items.retain(|item| &item.id != id);
                    state.selected_item_id = None; // Clear selection after delete
                }
                Command::None
            }

            Msg::RetrySelected => {
                if let Some(id) = &state.selected_item_id {
                    if let Some(item) = state.queue_items.iter_mut().find(|i| &i.id == id) {
                        item.status = OperationStatus::Pending;
                        item.result = None;
                    }
                    if state.auto_play {
                        return execute_next_if_available(state);
                    }
                }
                Command::None
            }

            Msg::AddItems(mut items) => {
                state.queue_items.append(&mut items);

                // If in play mode and we have capacity, start executing
                if state.auto_play && state.currently_running.len() < state.max_concurrent {
                    execute_next_if_available(state)
                } else {
                    Command::None
                }
            }

            Msg::ClearQueue => {
                state.queue_items.clear();
                state.selected_item_id = None;
                Command::None
            }

            Msg::DetailsScroll(key) => {
                // Dimensions are tracked from last on_render call
                let viewport_height = state.details_scroll_state.viewport_height().unwrap_or(20);
                let content_height = state.details_scroll_state.content_height().unwrap_or(20);
                state.details_scroll_state.handle_key(key, content_height, viewport_height);
                Command::None
            }

            Msg::DetailsSetDimensions(viewport_height, content_height) => {
                // Called every frame by renderer with actual dimensions
                state.details_scroll_state.set_viewport_height(viewport_height);
                state.details_scroll_state.update_scroll(viewport_height, content_height);
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

        // Controls and stats row
        let play_button = if state.auto_play {
            Element::button("pause-btn", "[P] Pause").on_press(Msg::TogglePlay)
        } else {
            Element::button("play-btn", "[P] Play").on_press(Msg::TogglePlay)
        }
        .build();

        let step_button = Element::button("step-btn", "[s] Step")
            .on_press(Msg::StepOne)
            .build();

        let clear_button = Element::button("clear-btn", "[C] Clear")
            .on_press(Msg::ClearQueue)
            .build();

        let count_by_status = |status: OperationStatus| {
            state
                .queue_items
                .iter()
                .filter(|item| item.status == status)
                .count()
        };

        let stats_text = format!(
            "Total: {}  Pending: {}  Running: {}  Done: {}  Failed: {}",
            state.queue_items.len(),
            count_by_status(OperationStatus::Pending),
            state.currently_running.len(),
            count_by_status(OperationStatus::Done),
            count_by_status(OperationStatus::Failed),
        );

        // Time estimates
        let est_3 = estimate_remaining_time(state, 3).unwrap_or_else(|| "-".to_string());
        let est_5 = estimate_remaining_time(state, 5).unwrap_or_else(|| "-".to_string());
        let est_10 = estimate_remaining_time(state, 10).unwrap_or_else(|| "-".to_string());

        let estimates_text = format!(
            "⏱ Est. remaining (last 3/5/10): {} / {} / {}",
            est_3, est_5, est_10
        );

        let buttons = row![
            play_button => Length(14),
            Element::None => Length(1),
            step_button => Length(11),
            Element::None => Length(1),
            clear_button => Length(11),
        ];

        let stats_and_estimates = col![
            Element::text(stats_text) => Length(1),
            Element::None => Length(1),
            Element::text(estimates_text) => Length(1),
        ];

        let header = row![
            buttons => Length(38),
            Element::None => Length(2),
            stats_and_estimates => Fill(1),
        ];

        // Table tree
        let tree_widget = Element::table_tree("queue-tree", &tree_nodes, &mut state.tree_state)
            .on_event(Msg::TreeEvent)
            .on_select(Msg::NodeSelected)
            .on_render(Msg::ViewportHeight)
            .build();

        let tree = Element::panel(tree_widget)
            .title("Queue")
            .build();

        // Build details panel for selected item
        let details_panel = build_details_panel(state, theme, &state.details_scroll_state);

        // Split into tree (left) and details (right) - 2/1 ratio
        let main_content = row![
            col![
                header => Length(3),
                tree => Fill(1),
            ] => Fill(2),
            details_panel => Fill(1),
        ];

        LayeredView::new(Element::panel(main_content).build())
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        use crate::tui::{Subscription, KeyBinding};
        use crossterm::event::KeyCode;

        vec![
            // Keyboard shortcuts
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('P')), "Toggle play/pause (queue)", Msg::TogglePlay),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('p')), "Toggle pause (selected)", Msg::TogglePauseSelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('s')), "Step one operation", Msg::StepOne),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('C')), "Clear queue", Msg::ClearQueue),
            Subscription::keyboard(KeyBinding::new(KeyCode::Esc), "Back to launcher", Msg::Back),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('=')), "Increase priority (selected)", Msg::IncreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('+')), "Increase priority (selected)", Msg::IncreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('-')), "Decrease priority (selected)", Msg::DecreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('r')), "Retry (selected)", Msg::RetrySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('d')), "Delete (selected)", Msg::DeleteSelected),

            // Event subscriptions
            Subscription::subscribe("queue:add_items", |value| {
                // Deserialize Vec<QueueItem> from JSON
                serde_json::from_value::<Vec<QueueItem>>(value)
                    .ok()
                    .map(Msg::AddItems)
            }),
        ]
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
        // Mark as running immediately and set start time
        if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
            item.status = OperationStatus::Running;
            item.started_at = Some(std::time::Instant::now());
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

/// Build the details panel for the selected queue item
/// Calculate average completion time from last N successful operations
fn calculate_avg_time(recent_times: &VecDeque<u64>, n: usize) -> Option<u64> {
    if recent_times.is_empty() {
        return None;
    }

    let count = recent_times.len().min(n);
    let sum: u64 = recent_times.iter().rev().take(count).sum();
    Some(sum / count as u64)
}

/// Estimate time remaining for pending operations
fn estimate_remaining_time(state: &State, n: usize) -> Option<String> {
    let avg_time = calculate_avg_time(&state.recent_completion_times, n)?;
    let pending_count = state.queue_items.iter()
        .filter(|item| item.status == OperationStatus::Pending)
        .count();

    if pending_count == 0 {
        return None;
    }

    // Account for concurrent execution
    let concurrent = state.max_concurrent.max(1);
    let estimated_ms = (avg_time * pending_count as u64) / concurrent as u64;

    Some(format_duration_estimate(estimated_ms))
}

/// Format duration estimate in a readable way
fn format_duration_estimate(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.0}s", ms as f64 / 1000.0)
    } else if ms < 3_600_000 {
        let minutes = ms / 60_000;
        let seconds = (ms % 60_000) / 1000;
        if seconds > 0 {
            format!("{}m{}s", minutes, seconds)
        } else {
            format!("{}m", minutes)
        }
    } else {
        let hours = ms / 3_600_000;
        let minutes = (ms % 3_600_000) / 60_000;
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    }
}

fn build_details_panel(state: &State, theme: &Theme, scroll_state: &ScrollableState) -> Element<Msg> {
    use ratatui::style::Style;
    use ratatui::text::{Line as RataLine, Span};
    use ratatui::prelude::Stylize;

    let selected_item = state.selected_item_id.as_ref()
        .and_then(|id| state.queue_items.iter().find(|item| &item.id == id))
        .cloned();

    let content = if let Some(item) = selected_item {
        // Build detailed information about the selected item
        let mut lines = vec![
            // Header with status
            Element::styled_text(RataLine::from(vec![
                Span::styled(
                    format!("{} ", item.status.symbol()),
                    Style::default().fg(match item.status {
                        OperationStatus::Pending => theme.yellow,
                        OperationStatus::Running => theme.blue,
                        OperationStatus::Paused => theme.overlay1,
                        OperationStatus::Done => theme.green,
                        OperationStatus::Failed => theme.red,
                    })
                ),
                Span::styled(
                    item.metadata.description.clone(),
                    Style::default().fg(theme.text).bold()
                ),
            ])).build(),
            Element::text(""),

            // Priority
            Element::styled_text(RataLine::from(vec![
                Span::styled("Priority: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    item.priority.to_string(),
                    Style::default().fg(theme.mauve)
                ),
            ])).build(),

            // Source
            Element::styled_text(RataLine::from(vec![
                Span::styled("Source: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    item.metadata.source.clone(),
                    Style::default().fg(theme.text)
                ),
            ])).build(),

            // Entity type
            Element::styled_text(RataLine::from(vec![
                Span::styled("Entity: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    item.metadata.entity_type.clone(),
                    Style::default().fg(theme.text)
                ),
            ])).build(),

            // Environment
            Element::styled_text(RataLine::from(vec![
                Span::styled("Environment: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    item.metadata.environment_name.clone(),
                    Style::default().fg(theme.text)
                ),
            ])).build(),
        ];

        // Row number if applicable
        if let Some(row) = item.metadata.row_number {
            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("Row: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    row.to_string(),
                    Style::default().fg(theme.text)
                ),
            ])).build());
        }

        lines.push(Element::text(""));

        // Operations list
        lines.push(Element::styled_text(RataLine::from(vec![
            Span::styled(
                format!("Operations ({}):", item.operations.len()),
                Style::default().fg(theme.peach).bold()
            ),
        ])).build());

        for (idx, op) in item.operations.operations().iter().enumerate() {
            let op_type = op.operation_type().to_string();
            let entity = op.entity().to_string();

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled(format!("  {}. ", idx + 1), Style::default().fg(theme.overlay1)),
                Span::styled(op_type, Style::default().fg(theme.blue)),
                Span::raw(" "),
                Span::styled(entity, Style::default().fg(theme.text)),
            ])).build());
        }

        // Show results if completed or failed
        if let Some(result) = &item.result {
            lines.push(Element::text(""));
            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("Result:", Style::default().fg(theme.peach).bold()),
            ])).build());

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("  Status: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    if result.success { "Success" } else { "Failed" },
                    Style::default().fg(if result.success { theme.green } else { theme.red })
                ),
            ])).build());

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("  Duration: ", Style::default().fg(theme.overlay1)),
                Span::styled(
                    format!("{}ms", result.duration_ms),
                    Style::default().fg(theme.text)
                ),
            ])).build());

            // Show error if any
            if let Some(error) = &result.error {
                lines.push(Element::text(""));
                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("Error:", Style::default().fg(theme.red).bold()),
                ])).build());

                // Split error message into lines if too long
                let max_width = 40;
                let error_lines: Vec<&str> = error.as_str()
                    .split('\n')
                    .flat_map(|line| {
                        if line.len() <= max_width {
                            vec![line]
                        } else {
                            line.as_bytes()
                                .chunks(max_width)
                                .map(|chunk| std::str::from_utf8(chunk).unwrap_or(""))
                                .collect()
                        }
                    })
                    .collect();

                for error_line in error_lines {
                    lines.push(Element::styled_text(RataLine::from(vec![
                        Span::styled(format!("  {}", error_line), Style::default().fg(theme.red)),
                    ])).build());
                }
            }

            // Show individual operation results
            if !result.operation_results.is_empty() {
                lines.push(Element::text(""));
                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("Operation Results:", Style::default().fg(theme.peach).bold()),
                ])).build());

                for (idx, op_result) in result.operation_results.iter().enumerate() {
                    let status_symbol = if op_result.success { "✓" } else { "✗" };
                    let status_color = if op_result.success { theme.green } else { theme.red };

                    let msg = if let Some(err) = &op_result.error {
                        err.clone()
                    } else {
                        "OK".to_string()
                    };

                    lines.push(Element::styled_text(RataLine::from(vec![
                        Span::styled(format!("  {}. ", idx + 1), Style::default().fg(theme.overlay1)),
                        Span::styled(status_symbol, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::styled(msg, Style::default().fg(theme.text)),
                    ])).build());
                }
            }
        }

        Element::column(lines).spacing(0).build()
    } else {
        // No selection
        Element::column(vec![
            Element::styled_text(RataLine::from(vec![
                Span::styled("No item selected", Style::default().fg(theme.overlay1).italic()),
            ])).build(),
        ]).spacing(0).build()
    };

    // Wrap content in scrollable
    let scrollable_content = Element::scrollable(
        FocusId::new("details-scroll"),
        content,
        scroll_state
    )
    .on_navigate(Msg::DetailsScroll)
    .on_render(Msg::DetailsSetDimensions)
    .build();

    Element::panel(scrollable_content)
        .title("Details")
        .build()
}
