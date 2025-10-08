//! Operation Queue App

use crate::tui::{
    app::App,
    command::{AppId, Command},
    element::{Element, LayoutConstraint, FocusId, Alignment},
    subscription::Subscription,
    state::theme::Theme,
    renderer::LayeredView,
    widgets::{TreeState, TreeEvent, ScrollableState},
    ModalState,
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
    RequestClearQueue,
    ConfirmClearQueue,
    RequestDeleteSelected,
    ConfirmDeleteSelected,
    CancelModal,

    // Execution
    StartExecution(String),
    ExecutionCompleted(String, QueueResult),

    // Filters/Settings
    SetFilter(QueueFilter),
    SetSortMode(SortMode),
    SetMaxConcurrent(usize),

    // Details panel scrolling
    DetailsScroll(crossterm::event::KeyCode),
    DetailsSetDimensions(usize, usize, usize, usize),  // (viewport_height, content_height, viewport_width, content_width)

    // State loading and persistence
    StateLoaded(Result<(Vec<QueueItem>, crate::config::repository::queue::QueueSettings, Vec<QueueItem>), String>),
    PersistenceError(String),

    // Interruption warnings
    DismissInterruptionWarning,
    ClearInterruptionFlag(String),
    ClearInterruptionFlagSelected,

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

    // Modals
    clear_confirm_modal: ModalState<()>,
    delete_confirm_modal: ModalState<()>,
    interruption_warning_modal: ModalState<Vec<QueueItem>>,

    // Loading state
    is_loading: bool,
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
            clear_confirm_modal: ModalState::Closed,
            delete_confirm_modal: ModalState::Closed,
            interruption_warning_modal: ModalState::Closed,
            is_loading: true,
        }
    }
}

impl crate::tui::AppState for State {}

impl App for OperationQueueApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        let cmd = Command::perform(
            async move {
                let config = crate::global_config();

                // Load queue items
                let mut queue_items = config.list_queue_items().await
                    .map_err(|e| format!("Failed to load queue items: {}", e))?;

                // Load settings
                let settings = config.get_queue_settings().await
                    .map_err(|e| format!("Failed to load queue settings: {}", e))?;

                // Detect and handle interrupted items
                let mut interrupted_items = Vec::new();
                let now = chrono::Utc::now();

                log::info!("Checking for interrupted items. Total items loaded: {}", queue_items.len());

                for item in &mut queue_items {
                    log::debug!("Item {} has status: {:?}", item.id, item.status);
                    if item.status == OperationStatus::Running {
                        log::warn!("Found interrupted item: {} (was Running)", item.id);

                        // Mark as interrupted
                        item.status = OperationStatus::Pending;
                        item.was_interrupted = true;
                        item.interrupted_at = Some(now);
                        item.started_at = None;

                        interrupted_items.push(item.clone());

                        // Persist the changes
                        config.update_queue_item_status(&item.id, OperationStatus::Pending).await
                            .map_err(|e| format!("Failed to update status: {}", e))?;
                        config.mark_queue_item_interrupted(&item.id, now).await
                            .map_err(|e| format!("Failed to mark interrupted: {}", e))?;
                    }
                }

                log::info!("Found {} interrupted items", interrupted_items.len());

                Ok((queue_items, settings, interrupted_items))
            },
            Msg::StateLoaded
        );

        (State::default(), cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::StateLoaded(result) => {
                match result {
                    Ok((queue_items, settings, interrupted_items)) => {
                        state.queue_items = queue_items;
                        state.auto_play = false; // Always start paused on load
                        state.max_concurrent = settings.max_concurrent;
                        state.filter = settings.filter;
                        state.sort_mode = settings.sort_mode;
                        state.is_loading = false;

                        // Auto-select first item if queue is not empty
                        if !state.queue_items.is_empty() && state.selected_item_id.is_none() {
                            state.selected_item_id = state.queue_items.first().map(|item| item.id.clone());
                        }

                        // Show warning modal if there are interrupted items
                        if !interrupted_items.is_empty() {
                            state.interruption_warning_modal.open_with(interrupted_items);
                            Command::set_focus(FocusId::new("warning-close"))
                        } else {
                            Command::set_focus(FocusId::new("queue-tree"))
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to load queue state: {}", err);
                        state.is_loading = false;
                        Command::None
                    }
                }
            }

            Msg::PersistenceError(err) => {
                log::error!("Queue persistence error: {}", err);
                Command::None
            }

            Msg::DismissInterruptionWarning => {
                state.interruption_warning_modal.close();
                Command::set_focus(FocusId::new("queue-tree"))
            }

            Msg::ClearInterruptionFlag(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.was_interrupted = false;
                    item.interrupted_at = None;

                    // Persist to database
                    let item_id = id.clone();
                    return Command::perform(
                        async move {
                            let config = crate::global_config();
                            config.clear_queue_interruption_flag(&item_id).await
                                .map_err(|e| format!("Failed to clear interruption flag: {}", e))
                        },
                        |result| {
                            match result {
                                Err(err) => Msg::PersistenceError(err),
                                Ok(_) => Msg::PersistenceError("".to_string()),
                            }
                        }
                    );
                }
                Command::None
            }

            Msg::ClearInterruptionFlagSelected => {
                if let Some(id) = &state.selected_item_id {
                    return Self::update(state, Msg::ClearInterruptionFlag(id.clone()));
                }
                Command::None
            }

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

                let save_cmd = save_settings_command(state);
                let exec_cmd = if state.auto_play {
                    execute_next_if_available(state)
                } else {
                    Command::None
                };

                Command::Batch(vec![save_cmd, exec_cmd])
            }

            Msg::StepOne => {
                state.auto_play = false;
                execute_next_if_available(state)
            }

            Msg::IncreasePriority(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    if item.priority > 0 {
                        item.priority -= 1;
                        let new_priority = item.priority;
                        let item_id = id.clone();
                        return Command::perform(
                            async move {
                                crate::global_config().update_queue_item_priority(&item_id, new_priority).await
                                    .map_err(|e| format!("Failed to update priority: {}", e))
                            },
                            |result| {
                                if let Err(err) = result {
                                    Msg::PersistenceError(err)
                                } else {
                                    Msg::PersistenceError("".to_string())
                                }
                            }
                        );
                    }
                }
                Command::None
            }

            Msg::DecreasePriority(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    if item.priority < 255 {
                        item.priority += 1;
                        let new_priority = item.priority;
                        let item_id = id.clone();
                        return Command::perform(
                            async move {
                                crate::global_config().update_queue_item_priority(&item_id, new_priority).await
                                    .map_err(|e| format!("Failed to update priority: {}", e))
                            },
                            |result| {
                                if let Err(err) = result {
                                    Msg::PersistenceError(err)
                                } else {
                                    Msg::PersistenceError("".to_string())
                                }
                            }
                        );
                    }
                }
                Command::None
            }

            Msg::TogglePauseItem(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    let new_status = match item.status {
                        OperationStatus::Pending => OperationStatus::Paused,
                        OperationStatus::Paused => OperationStatus::Pending,
                        _ => item.status.clone(),
                    };
                    item.status = new_status.clone();

                    let item_id = id.clone();
                    return Command::perform(
                        async move {
                            crate::global_config().update_queue_item_status(&item_id, new_status).await
                                .map_err(|e| format!("Failed to update status: {}", e))
                        },
                        |result| {
                            if let Err(err) = result {
                                Msg::PersistenceError(err)
                            } else {
                                Msg::PersistenceError("".to_string())
                            }
                        }
                    );
                }
                Command::None
            }

            Msg::DeleteItem(id) => {
                state.queue_items.retain(|item| item.id != id);

                Command::perform(
                    async move {
                        crate::global_config().delete_queue_item(&id).await
                            .map_err(|e| format!("Failed to delete queue item: {}", e))
                    },
                    |result| {
                        if let Err(err) = result {
                            Msg::PersistenceError(err)
                        } else {
                            Msg::PersistenceError("".to_string())
                        }
                    }
                )
            }

            Msg::RetryItem(id) => {
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = OperationStatus::Pending;
                    item.result = None;
                    item.started_at = None;

                    let item_id = id.clone();
                    let persist_cmd = Command::perform(
                        async move {
                            crate::global_config().update_queue_item_status(&item_id, OperationStatus::Pending).await
                                .map_err(|e| format!("Failed to update status: {}", e))
                        },
                        |result| {
                            if let Err(err) = result {
                                Msg::PersistenceError(err)
                            } else {
                                Msg::PersistenceError("".to_string())
                            }
                        }
                    );

                    let exec_cmd = if state.auto_play {
                        execute_next_if_available(state)
                    } else {
                        Command::None
                    };

                    return Command::Batch(vec![persist_cmd, exec_cmd]);
                }
                Command::None
            }

            Msg::StartExecution(id) => {
                // Mark as running and set start time
                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    item.status = OperationStatus::Running;
                    item.started_at = Some(std::time::Instant::now());
                    state.currently_running.insert(id.clone());
                }

                // Persist Running status to database
                let item_id_for_persist = id.clone();
                let persist_cmd = Command::perform(
                    async move {
                        log::info!("Persisting Running status for item: {}", item_id_for_persist);
                        let result = crate::global_config().update_queue_item_status(&item_id_for_persist, OperationStatus::Running).await;
                        match &result {
                            Ok(_) => log::info!("Successfully persisted Running status for item: {}", item_id_for_persist),
                            Err(e) => log::error!("Failed to persist Running status: {}", e),
                        }
                        result.map_err(|e| format!("Failed to persist Running status: {}", e))
                    },
                    |result| {
                        if let Err(err) = result {
                            Msg::PersistenceError(err)
                        } else {
                            Msg::PersistenceError("".to_string())
                        }
                    }
                );

                // Get item for execution
                let item = state.queue_items.iter().find(|i| i.id == id).cloned();

                if let Some(item) = item {
                    let exec_cmd = Command::perform(
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
                    );

                    Command::Batch(vec![persist_cmd, exec_cmd])
                } else {
                    persist_cmd
                }
            }

            Msg::ExecutionCompleted(id, result) => {
                state.currently_running.remove(&id);

                let mut publish_cmd = Command::None;
                let mut persist_cmd = Command::None;

                if let Some(item) = state.queue_items.iter_mut().find(|i| i.id == id) {
                    let new_status = if result.success {
                        OperationStatus::Done
                    } else {
                        OperationStatus::Failed
                    };
                    item.status = new_status.clone();
                    item.result = Some(result.clone());

                    // Persist to database
                    let item_id = id.clone();
                    let result_clone = result.clone();
                    persist_cmd = Command::perform(
                        async move {
                            let config = crate::global_config();
                            config.update_queue_item_status(&item_id, new_status).await
                                .map_err(|e| format!("Failed to update status: {}", e))?;
                            config.update_queue_item_result(&item_id, &result_clone).await
                                .map_err(|e| format!("Failed to update result: {}", e))?;
                            Ok(())
                        },
                        |result| {
                            if let Err(err) = result {
                                Msg::PersistenceError(err)
                            } else {
                                Msg::PersistenceError("".to_string())
                            }
                        }
                    );

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

                Command::Batch(vec![publish_cmd, persist_cmd, next_cmd])
            }

            Msg::SetFilter(filter) => {
                state.filter = filter;
                save_settings_command(state)
            }

            Msg::SetSortMode(sort_mode) => {
                state.sort_mode = sort_mode;
                save_settings_command(state)
            }

            Msg::SetMaxConcurrent(max) => {
                state.max_concurrent = max;
                save_settings_command(state)
            }

            // Keyboard shortcuts operating on selected item
            Msg::IncreasePrioritySelected => {
                if let Some(id) = state.selected_item_id.clone() {
                    return Self::update(state, Msg::IncreasePriority(id));
                }
                Command::None
            }

            Msg::DecreasePrioritySelected => {
                if let Some(id) = state.selected_item_id.clone() {
                    return Self::update(state, Msg::DecreasePriority(id));
                }
                Command::None
            }

            Msg::TogglePauseSelected => {
                if let Some(id) = state.selected_item_id.clone() {
                    return Self::update(state, Msg::TogglePauseItem(id));
                }
                Command::None
            }

            Msg::RequestDeleteSelected => {
                // Only show modal if there's a selected item
                if state.selected_item_id.is_some() {
                    state.delete_confirm_modal.open_empty();
                    Command::set_focus(FocusId::new("confirmation-cancel"))
                } else {
                    Command::None
                }
            }

            Msg::ConfirmDeleteSelected => {
                state.delete_confirm_modal.close();
                if let Some(id) = &state.selected_item_id {
                    let item_id = id.clone();
                    state.queue_items.retain(|item| &item.id != id);
                    state.selected_item_id = None; // Clear selection after delete

                    return Command::perform(
                        async move {
                            crate::global_config().delete_queue_item(&item_id).await
                                .map_err(|e| format!("Failed to delete queue item: {}", e))
                        },
                        |result| {
                            if let Err(err) = result {
                                Msg::PersistenceError(err)
                            } else {
                                Msg::PersistenceError("".to_string())
                            }
                        }
                    );
                }
                Command::None
            }

            Msg::DeleteSelected => {
                // Deprecated - use RequestDeleteSelected instead
                Command::None
            }

            Msg::RetrySelected => {
                if let Some(id) = &state.selected_item_id {
                    if let Some(item) = state.queue_items.iter_mut().find(|i| &i.id == id) {
                        item.status = OperationStatus::Pending;
                        item.result = None;

                        let item_id = id.clone();
                        let persist_cmd = Command::perform(
                            async move {
                                crate::global_config().update_queue_item_status(&item_id, OperationStatus::Pending).await
                                    .map_err(|e| format!("Failed to update status: {}", e))
                            },
                            |result| {
                                if let Err(err) = result {
                                    Msg::PersistenceError(err)
                                } else {
                                    Msg::PersistenceError("".to_string())
                                }
                            }
                        );

                        let exec_cmd = if state.auto_play {
                            execute_next_if_available(state)
                        } else {
                            Command::None
                        };

                        return Command::Batch(vec![persist_cmd, exec_cmd]);
                    }
                }
                Command::None
            }

            Msg::AddItems(items) => {
                let was_empty = state.queue_items.is_empty();

                // Persist each item to database
                let items_to_save = items.clone();
                let persist_cmd = Command::perform(
                    async move {
                        let config = crate::global_config();
                        for item in &items_to_save {
                            if let Err(e) = config.save_queue_item(item).await {
                                return Err(format!("Failed to save queue item: {}", e));
                            }
                        }
                        Ok(())
                    },
                    |result| {
                        if let Err(err) = result {
                            Msg::PersistenceError(err)
                        } else {
                            Msg::PersistenceError("".to_string())
                        }
                    }
                );

                let mut items = items;
                state.queue_items.append(&mut items);

                // If queue was empty and we just added items, select the first one
                if was_empty && !state.queue_items.is_empty() && state.selected_item_id.is_none() {
                    state.selected_item_id = state.queue_items.first().map(|item| item.id.clone());
                }

                // If in play mode and we have capacity, start executing
                let exec_cmd = if state.auto_play && state.currently_running.len() < state.max_concurrent {
                    execute_next_if_available(state)
                } else {
                    Command::None
                };

                Command::Batch(vec![persist_cmd, exec_cmd])
            }

            Msg::RequestClearQueue => {
                state.clear_confirm_modal.open_empty();
                Command::set_focus(FocusId::new("confirmation-cancel"))
            }

            Msg::ConfirmClearQueue => {
                state.clear_confirm_modal.close();
                state.queue_items.clear();
                state.selected_item_id = None;

                Command::perform(
                    async move {
                        crate::global_config().clear_queue().await
                            .map_err(|e| format!("Failed to clear queue: {}", e))
                    },
                    |result| {
                        if let Err(err) = result {
                            Msg::PersistenceError(err)
                        } else {
                            Msg::PersistenceError("".to_string())
                        }
                    }
                )
            }

            Msg::CancelModal => {
                state.clear_confirm_modal.close();
                state.delete_confirm_modal.close();
                Command::None
            }

            Msg::DetailsScroll(key) => {
                // Dimensions are tracked from last on_render call
                let viewport_height = state.details_scroll_state.viewport_height().unwrap_or(20);
                let content_height = state.details_scroll_state.content_height().unwrap_or(20);
                state.details_scroll_state.handle_key(key, content_height, viewport_height);
                Command::None
            }

            Msg::DetailsSetDimensions(viewport_height, content_height, viewport_width, content_width) => {
                // Called every frame by renderer with actual dimensions
                state.details_scroll_state.set_viewport_height(viewport_height);
                state.details_scroll_state.update_scroll(viewport_height, content_height);
                state.details_scroll_state.set_viewport_width(viewport_width);
                state.details_scroll_state.update_horizontal_scroll(viewport_width, content_width);
                Command::None
            }

            Msg::Back => Command::navigate_to(AppId::AppLauncher),
        }
    }

    fn view(state: &mut State) -> LayeredView<Msg> {
        use_constraints!();
        let theme = &crate::global_runtime_config().theme;

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
            .on_press(Msg::RequestClearQueue)
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
        let details_panel = build_details_panel(state, &state.details_scroll_state);

        // Split into tree (left) and details (right) - 2/1 ratio
        let main_content = row![
            col![
                header => Length(3),
                tree => Fill(1),
            ] => Fill(2),
            details_panel => Fill(1),
        ];

        let mut view = LayeredView::new(Element::panel(main_content).build());

        // Add clear confirmation modal if open
        if state.clear_confirm_modal.is_open() {
            let modal = build_clear_confirm_modal();
            view = view.with_app_modal(modal, Alignment::Center);
        }

        // Add delete confirmation modal if open
        if state.delete_confirm_modal.is_open() {
            let modal = build_delete_confirm_modal();
            view = view.with_app_modal(modal, Alignment::Center);
        }

        // Add interruption warning modal if open
        if state.interruption_warning_modal.is_open() {
            let modal = build_interruption_warning_modal(state);
            view = view.with_app_modal(modal, Alignment::Center);
        }

        view
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        use crate::tui::{Subscription, KeyBinding};
        use crossterm::event::KeyCode;

        vec![
            // Keyboard shortcuts
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('P')), "Toggle play/pause (queue)", Msg::TogglePlay),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('p')), "Toggle pause (selected)", Msg::TogglePauseSelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('s')), "Step one operation", Msg::StepOne),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('C')), "Clear queue", Msg::RequestClearQueue),
            Subscription::keyboard(KeyBinding::new(KeyCode::Esc), "Back to launcher", Msg::Back),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('=')), "Increase priority (selected)", Msg::IncreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('+')), "Increase priority (selected)", Msg::IncreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('-')), "Decrease priority (selected)", Msg::DecreasePrioritySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('r')), "Retry (selected)", Msg::RetrySelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('d')), "Delete (selected)", Msg::RequestDeleteSelected),
            Subscription::keyboard(KeyBinding::new(KeyCode::Char('c')), "Clear interruption warning (selected)", Msg::ClearInterruptionFlagSelected),

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

    fn status(state: &State) -> Option<Line<'static>> {
        use ratatui::text::Span;
        let theme = &crate::global_runtime_config().theme;
        use ratatui::style::Style;

        let interrupted_count = state.queue_items.iter()
            .filter(|item| item.was_interrupted)
            .count();

        if interrupted_count > 0 {
            Some(Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(theme.accent_error)),
                Span::styled(
                    format!("{} interrupted operation(s) - verify before resuming", interrupted_count),
                    Style::default().fg(theme.accent_warning)
                ),
            ]))
        } else {
            None
        }
    }
}

/// Helper function to save queue settings to database
/// Note: auto_play is NOT persisted (always starts paused)
fn save_settings_command(state: &State) -> Command<Msg> {
    let settings = crate::config::repository::queue::QueueSettings {
        auto_play: false, // Never persist auto_play - always start paused
        max_concurrent: state.max_concurrent,
        filter: state.filter,
        sort_mode: state.sort_mode,
    };

    Command::perform(
        async move {
            crate::global_config().save_queue_settings(&settings).await
                .map_err(|e| format!("Failed to save queue settings: {}", e))
        },
        |result| {
            if let Err(err) = result {
                Msg::PersistenceError(err)
            } else {
                Msg::PersistenceError("".to_string())
            }
        }
    )
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

        // Persist Running status to database
        let item_id_for_persist = id.clone();
        let persist_cmd = Command::perform(
            async move {
                log::info!("Persisting Running status for item: {}", item_id_for_persist);
                let result = crate::global_config().update_queue_item_status(&item_id_for_persist, OperationStatus::Running).await;
                match &result {
                    Ok(_) => log::info!("Successfully persisted Running status for item: {}", item_id_for_persist),
                    Err(e) => log::error!("Failed to persist Running status: {}", e),
                }
                result.map_err(|e| format!("Failed to persist Running status: {}", e))
            },
            |result| {
                if let Err(err) = result {
                    Msg::PersistenceError(err)
                } else {
                    Msg::PersistenceError("".to_string())
                }
            }
        );

        // Get item for execution
        let item = state.queue_items.iter().find(|i| i.id == id).cloned();

        if let Some(item) = item {
            let exec_cmd = Command::perform(
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
            );

            Command::Batch(vec![persist_cmd, exec_cmd])
        } else {
            persist_cmd
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

fn build_details_panel(state: &State, scroll_state: &ScrollableState) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use ratatui::style::Style;
    use ratatui::text::{Line as RataLine, Span};
    use ratatui::prelude::Stylize;

    // Check if selected ID is a child node (format: "parent_id_index")
    let (selected_item, child_index) = if let Some(selected_id) = &state.selected_item_id {
        // Try to parse as child ID
        if let Some(last_underscore_pos) = selected_id.rfind('_') {
            let potential_parent_id = &selected_id[..last_underscore_pos];
            let potential_index = &selected_id[last_underscore_pos + 1..];

            // Check if the part after underscore is a number and parent exists
            if let Ok(index) = potential_index.parse::<usize>() {
                if let Some(item) = state.queue_items.iter().find(|item| item.id == potential_parent_id) {
                    (Some(item.clone()), Some(index))
                } else {
                    // Not a valid child, try as parent
                    (state.queue_items.iter().find(|item| &item.id == selected_id).cloned(), None)
                }
            } else {
                // Not a number, must be a parent ID
                (state.queue_items.iter().find(|item| &item.id == selected_id).cloned(), None)
            }
        } else {
            // No underscore, must be a parent ID
            (state.queue_items.iter().find(|item| &item.id == selected_id).cloned(), None)
        }
    } else {
        (None, None)
    };

    let content = if let Some(item) = selected_item {
        // If viewing a child node, show details about that specific operation
        if let Some(child_idx) = child_index {
            // Get the specific operation (child_idx is 1-based from tree_nodes.rs, but we skip(1) in the tree)
            // So child_idx=1 means index 1 in the operations array (second operation)
            let operations = item.operations.operations();
            if child_idx < operations.len() {
                let operation = &operations[child_idx];

                let mut lines = vec![
                    // Header
                    Element::styled_text(RataLine::from(vec![
                        Span::styled(
                            format!("Operation {} of {}", child_idx + 1, operations.len()),
                            Style::default().fg(theme.text_primary).bold()
                        ),
                    ])).build(),
                    Element::text(""),

                    // Parent batch info
                    Element::styled_text(RataLine::from(vec![
                        Span::styled("Batch: ", Style::default().fg(theme.border_primary)),
                        Span::styled(
                            item.metadata.description.clone(),
                            Style::default().fg(theme.text_primary)
                        ),
                    ])).build(),
                    Element::text(""),

                    // Operation type
                    Element::styled_text(RataLine::from(vec![
                        Span::styled("Type: ", Style::default().fg(theme.border_primary)),
                        Span::styled(
                            operation.operation_type().to_string(),
                            Style::default().fg(theme.accent_secondary)
                        ),
                    ])).build(),

                    // Entity
                    Element::styled_text(RataLine::from(vec![
                        Span::styled("Entity: ", Style::default().fg(theme.border_primary)),
                        Span::styled(
                            operation.entity().to_string(),
                            Style::default().fg(theme.text_primary)
                        ),
                    ])).build(),
                ];

                // Construct endpoint
                use crate::api::operations::Operation;
                let endpoint = match operation {
                    Operation::Create { entity, .. } | Operation::CreateWithRefs { entity, .. } => {
                        format!("POST /{}", entity)
                    }
                    Operation::Update { entity, id, .. } => {
                        format!("PATCH /{}({})", entity, id)
                    }
                    Operation::Delete { entity, id, .. } => {
                        format!("DELETE /{}({})", entity, id)
                    }
                    Operation::Upsert { entity, key_field, key_value, .. } => {
                        format!("PATCH /{}({}='{}')", entity, key_field, key_value)
                    }
                    Operation::AssociateRef { entity, entity_ref, navigation_property, .. } => {
                        format!("POST /{}({})/{}/$ref", entity, entity_ref, navigation_property)
                    }
                };

                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("Endpoint: ", Style::default().fg(theme.border_primary)),
                    Span::styled(
                        endpoint,
                        Style::default().fg(theme.accent_secondary)
                    ),
                ])).build());

                // Show data based on operation type
                match operation {
                    Operation::Create { data, .. } | Operation::CreateWithRefs { data, .. }
                    | Operation::Update { data, .. } | Operation::Upsert { data, .. } => {
                        lines.push(Element::text(""));
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Data:", Style::default().fg(theme.accent_muted).bold()),
                        ])).build());

                        // Pretty print JSON data (limit to reasonable size)
                        if let Ok(json_str) = serde_json::to_string_pretty(data) {
                            for line in json_str.lines().take(20) {
                                lines.push(Element::styled_text(RataLine::from(vec![
                                    Span::styled(format!("  {}", line), Style::default().fg(theme.text_primary)),
                                ])).build());
                            }
                            if json_str.lines().count() > 20 {
                                lines.push(Element::styled_text(RataLine::from(vec![
                                    Span::styled("  ... (truncated)", Style::default().fg(theme.border_primary).italic()),
                                ])).build());
                            }
                        }
                    }
                    Operation::Delete { id, .. } => {
                        lines.push(Element::text(""));
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Record ID: ", Style::default().fg(theme.border_primary)),
                            Span::styled(id.clone(), Style::default().fg(theme.text_primary)),
                        ])).build());
                    }
                    Operation::AssociateRef { entity_ref, navigation_property, target_ref, .. } => {
                        lines.push(Element::text(""));
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Entity Ref: ", Style::default().fg(theme.border_primary)),
                            Span::styled(entity_ref.clone(), Style::default().fg(theme.text_primary)),
                        ])).build());
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Navigation: ", Style::default().fg(theme.border_primary)),
                            Span::styled(navigation_property.clone(), Style::default().fg(theme.text_primary)),
                        ])).build());
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Target: ", Style::default().fg(theme.border_primary)),
                            Span::styled(target_ref.clone(), Style::default().fg(theme.text_primary)),
                        ])).build());
                    }
                }

                // Show result if operation completed
                if let Some(result) = &item.result {
                    if child_idx < result.operation_results.len() {
                        let op_result = &result.operation_results[child_idx];

                        lines.push(Element::text(""));
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("Result:", Style::default().fg(theme.accent_muted).bold()),
                        ])).build());

                        let status_color = if op_result.success { theme.accent_success } else { theme.accent_error };
                        lines.push(Element::styled_text(RataLine::from(vec![
                            Span::styled("  Status: ", Style::default().fg(theme.border_primary)),
                            Span::styled(
                                if op_result.success { "Success" } else { "Failed" },
                                Style::default().fg(status_color)
                            ),
                        ])).build());

                        if let Some(status_code) = op_result.status_code {
                            lines.push(Element::styled_text(RataLine::from(vec![
                                Span::styled("  Status Code: ", Style::default().fg(theme.border_primary)),
                                Span::styled(
                                    status_code.to_string(),
                                    Style::default().fg(theme.text_primary)
                                ),
                            ])).build());
                        }

                        if let Some(error) = &op_result.error {
                            lines.push(Element::text(""));
                            lines.push(Element::styled_text(RataLine::from(vec![
                                Span::styled("  Error:", Style::default().fg(theme.accent_error).bold()),
                            ])).build());

                            for error_line in error.lines() {
                                lines.push(Element::styled_text(RataLine::from(vec![
                                    Span::styled(format!("    {}", error_line), Style::default().fg(theme.accent_error)),
                                ])).build());
                            }
                        }
                    }
                }

                Element::column(lines).spacing(0).build()
            } else {
                // Invalid child index
                Element::column(vec![
                    Element::styled_text(RataLine::from(vec![
                        Span::styled("Invalid operation index", Style::default().fg(theme.accent_error)),
                    ])).build(),
                ]).spacing(0).build()
            }
        } else {
            // Parent node - show batch overview
            let mut lines = vec![
            // Header with status
            Element::styled_text(RataLine::from(vec![
                Span::styled(
                    format!("{} ", item.status.symbol()),
                    Style::default().fg(match item.status {
                        OperationStatus::Pending => theme.accent_warning,
                        OperationStatus::Running => theme.accent_secondary,
                        OperationStatus::Paused => theme.border_primary,
                        OperationStatus::Done => theme.accent_success,
                        OperationStatus::Failed => theme.accent_error,
                    })
                ),
                Span::styled(
                    item.metadata.description.clone(),
                    Style::default().fg(theme.text_primary).bold()
                ),
            ])).build(),
            Element::text(""),

            // Priority
            Element::styled_text(RataLine::from(vec![
                Span::styled("Priority: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    item.priority.to_string(),
                    Style::default().fg(theme.accent_tertiary)
                ),
            ])).build(),

            // Source
            Element::styled_text(RataLine::from(vec![
                Span::styled("Source: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    item.metadata.source.clone(),
                    Style::default().fg(theme.text_primary)
                ),
            ])).build(),

            // Entity type
            Element::styled_text(RataLine::from(vec![
                Span::styled("Entity: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    item.metadata.entity_type.clone(),
                    Style::default().fg(theme.text_primary)
                ),
            ])).build(),

            // Environment
            Element::styled_text(RataLine::from(vec![
                Span::styled("Environment: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    item.metadata.environment_name.clone(),
                    Style::default().fg(theme.text_primary)
                ),
            ])).build(),
        ];

        // Row number if applicable
        if let Some(row) = item.metadata.row_number {
            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("Row: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    row.to_string(),
                    Style::default().fg(theme.text_primary)
                ),
            ])).build());
        }

        // Warning section if interrupted
        if item.was_interrupted {
            lines.push(Element::text(""));
            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("⚠ WARNING: ", Style::default().fg(theme.accent_error).bold()),
                Span::styled(
                    "Operation was interrupted and may have partially executed.",
                    Style::default().fg(theme.accent_warning)
                ),
            ])).build());

            if let Some(interrupted_at) = item.interrupted_at {
                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("  Interrupted at: ", Style::default().fg(theme.border_primary)),
                    Span::styled(
                        interrupted_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                        Style::default().fg(theme.text_primary)
                    ),
                ])).build());
            }

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled(
                    "  → Verify completion in Dynamics before retrying or deleting",
                    Style::default().fg(theme.accent_warning).italic()
                ),
            ])).build());

            // Add clear warning button
            let clear_warning_btn = Element::button(
                FocusId::new("clear-warning"),
                "[c] Mark as Verified".to_string()
            )
            .on_press(Msg::ClearInterruptionFlag(item.id.clone()))
            .build();

            lines.push(Element::text(""));
            lines.push(clear_warning_btn);
        }

        lines.push(Element::text(""));

        // Operations list
        lines.push(Element::styled_text(RataLine::from(vec![
            Span::styled(
                format!("Operations ({}):", item.operations.len()),
                Style::default().fg(theme.accent_muted).bold()
            ),
        ])).build());

        for (idx, op) in item.operations.operations().iter().enumerate() {
            let op_type = op.operation_type().to_string();
            let entity = op.entity().to_string();

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled(format!("  {}. ", idx + 1), Style::default().fg(theme.border_primary)),
                Span::styled(op_type, Style::default().fg(theme.accent_secondary)),
                Span::raw(" "),
                Span::styled(entity, Style::default().fg(theme.text_primary)),
            ])).build());
        }

        // Show results if completed or failed
        if let Some(result) = &item.result {
            lines.push(Element::text(""));
            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("Result:", Style::default().fg(theme.accent_muted).bold()),
            ])).build());

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("  Status: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    if result.success { "Success" } else { "Failed" },
                    Style::default().fg(if result.success { theme.accent_success } else { theme.accent_error })
                ),
            ])).build());

            lines.push(Element::styled_text(RataLine::from(vec![
                Span::styled("  Duration: ", Style::default().fg(theme.border_primary)),
                Span::styled(
                    format!("{}ms", result.duration_ms),
                    Style::default().fg(theme.text_primary)
                ),
            ])).build());

            // Show error if any
            if let Some(error) = &result.error {
                lines.push(Element::text(""));
                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("Error:", Style::default().fg(theme.accent_error).bold()),
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
                        Span::styled(format!("  {}", error_line), Style::default().fg(theme.accent_error)),
                    ])).build());
                }
            }

            // Show individual operation results
            if !result.operation_results.is_empty() {
                lines.push(Element::text(""));
                lines.push(Element::styled_text(RataLine::from(vec![
                    Span::styled("Operation Results:", Style::default().fg(theme.accent_muted).bold()),
                ])).build());

                for (idx, op_result) in result.operation_results.iter().enumerate() {
                    let status_symbol = if op_result.success { "✓" } else { "✗" };
                    let status_color = if op_result.success { theme.accent_success } else { theme.accent_error };

                    let msg = if let Some(err) = &op_result.error {
                        err.clone()
                    } else {
                        "OK".to_string()
                    };

                    lines.push(Element::styled_text(RataLine::from(vec![
                        Span::styled(format!("  {}. ", idx + 1), Style::default().fg(theme.border_primary)),
                        Span::styled(status_symbol, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::styled(msg, Style::default().fg(theme.text_primary)),
                    ])).build());
                }
            }
        }

            Element::column(lines).spacing(0).build()
        }
    } else {
        // No selection
        Element::column(vec![
            Element::styled_text(RataLine::from(vec![
                Span::styled("No item selected", Style::default().fg(theme.border_primary).italic()),
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

fn build_clear_confirm_modal() -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::modals::ConfirmationModal;

    ConfirmationModal::new("Clear Queue")
        .message("Are you sure you want to clear all queue items?\nThis action cannot be undone.")
        .confirm_text("Yes")
        .cancel_text("No")
        .on_confirm(Msg::ConfirmClearQueue)
        .on_cancel(Msg::CancelModal)
        .width(60)
        .build()
}

fn build_delete_confirm_modal() -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::modals::ConfirmationModal;

    ConfirmationModal::new("Delete Item")
        .message("Are you sure you want to delete this queue item?\nThis action cannot be undone.")
        .confirm_text("Yes")
        .cancel_text("No")
        .on_confirm(Msg::ConfirmDeleteSelected)
        .on_cancel(Msg::CancelModal)
        .width(60)
        .build()
}

fn build_interruption_warning_modal(state: &State) -> Element<Msg> {
    let theme = &crate::global_runtime_config().theme;
    use crate::tui::modals::WarningModal;

    let interrupted_items = if let Some(items) = state.interruption_warning_modal.data() {
        items
    } else {
        return Element::None; // Should not happen
    };

    let count = interrupted_items.len();
    let message = format!(
        "The application was closed while {} operation(s) were executing.\n\
        These may have partially completed in Dynamics 365.\n\
        \n\
        Before resuming:\n\
        • Verify in Dynamics whether operations succeeded\n\
        • Delete items that already completed (press 'd')\n\
        • Keep items that need retry\n\
        \n\
        Items are marked with ⚠ in the queue.\n\
        Press 'c' on an item to clear its warning.",
        count
    );

    let mut modal = WarningModal::new("Interrupted Operations Detected")
        .message(message)
        .on_close(Msg::DismissInterruptionWarning)
        .width(80);

    // Add first few items as examples (limit to 5)
    for item in interrupted_items.iter().take(5) {
        let item_desc = format!("{} ({})", item.metadata.description, item.metadata.environment_name);
        modal = modal.add_item(item_desc);
    }

    if interrupted_items.len() > 5 {
        modal = modal.add_item(format!("... and {} more", interrupted_items.len() - 5));
    }

    modal.build()
}
