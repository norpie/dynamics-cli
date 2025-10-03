use ratatui::Frame;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind, MouseButton};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use serde_json::Value;
use std::pin::Pin;
use std::future::Future;
use std::any::Any;

use crate::tui::{App, AppId, Command, Renderer, InteractionRegistry, Subscription, AppState};
use crate::tui::command::{ParallelConfig, DispatchTarget};
use crate::tui::renderer::{FocusRegistry, FocusableInfo, DropdownRegistry};
use crate::tui::element::FocusId;
use crate::tui::state::{RuntimeConfig, FocusMode};

/// Trait for runtime operations, allowing type-erased storage of different Runtime<A> types
pub trait AppRuntime {
    fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool>;
    fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool>;
    fn render_to_area(&mut self, frame: &mut Frame, area: ratatui::layout::Rect);
    fn get_title(&self) -> &'static str;
    fn get_status(&self) -> Option<ratatui::text::Line<'static>>;
    fn get_key_bindings(&self) -> Vec<(KeyCode, String)>;
    fn poll_timers(&mut self) -> Result<()>;
    fn poll_async(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>;
    fn take_navigation(&mut self) -> Option<AppId>;
    fn take_publishes(&mut self) -> Vec<(String, Value)>;
    fn handle_publish(&mut self, topic: &str, data: Value) -> Result<()>;
    fn focus_next(&mut self) -> Result<()>;
    fn focus_previous(&mut self) -> Result<()>;
}

/// Tracks the state of a parallel task execution
struct ParallelTaskCoordinator<Msg> {
    total_tasks: usize,
    results: Arc<Mutex<Vec<Option<Box<dyn Any + Send>>>>>,
    msg_mapper: Arc<dyn Fn(usize, Box<dyn Any + Send>) -> Msg + Send>,
    config: ParallelConfig,
}

/// The runtime manages app lifecycle, event routing, and command execution
pub struct Runtime<A: App> {
    /// Current app state
    state: A::State,

    /// Runtime configuration (theme, focus mode, etc.)
    config: RuntimeConfig,

    /// Interaction registry for mouse events
    registry: InteractionRegistry<A::Msg>,

    /// Focus registry for keyboard focus
    focus_registry: FocusRegistry<A::Msg>,

    /// Dropdown registry for select widget overlays
    dropdown_registry: DropdownRegistry<A::Msg>,

    /// Currently focused element ID
    focused_id: Option<FocusId>,

    /// Keyboard subscriptions
    key_subscriptions: HashMap<KeyCode, A::Msg>,

    /// Event bus for pub/sub
    event_bus: HashMap<String, Vec<Box<dyn Fn(Value) -> Option<A::Msg> + Send>>>,

    /// Timer subscriptions: (interval, last_tick, msg)
    timers: Vec<(Duration, Instant, A::Msg)>,

    /// Last hovered element position for tracking hover exits
    last_hover_pos: Option<(u16, u16)>,

    /// Pending navigation request
    navigation_target: Option<AppId>,

    /// Pending async commands
    pending_async: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = A::Msg> + Send>>>,

    /// Pending parallel coordination tasks (task_index, task_name)
    pending_parallel: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = (usize, String)> + Send>>>,

    /// Pending publish events to broadcast globally
    pending_publishes: Vec<(String, serde_json::Value)>,

    /// Active parallel task coordinator
    parallel_coordinator: Option<ParallelTaskCoordinator<A::Msg>>,
}

impl<A: App> Runtime<A> {
    pub fn new() -> Self {
        let (state, init_command) = A::init();
        let config = RuntimeConfig::default();

        let mut runtime = Self {
            state,
            config,
            registry: InteractionRegistry::new(),
            focus_registry: FocusRegistry::new(),
            dropdown_registry: DropdownRegistry::new(),
            focused_id: None,
            key_subscriptions: HashMap::new(),
            event_bus: HashMap::new(),
            timers: Vec::new(),
            last_hover_pos: None,
            navigation_target: None,
            pending_async: Vec::new(),
            pending_parallel: Vec::new(),
            pending_publishes: Vec::new(),
            parallel_coordinator: None,
        };

        // Initialize subscriptions
        runtime.update_subscriptions();

        // Execute init command
        runtime.execute_command(init_command).ok();

        runtime
    }

    /// Take the pending navigation target (if any)
    pub fn take_navigation(&mut self) -> Option<AppId> {
        self.navigation_target.take()
    }

    /// Take pending publish events
    pub fn take_publishes(&mut self) -> Vec<(String, serde_json::Value)> {
        std::mem::take(&mut self.pending_publishes)
    }

    /// Get keyboard bindings for help menu
    pub fn get_key_bindings(&self) -> Vec<(KeyCode, String)> {
        use crate::tui::Subscription;

        A::subscriptions(&self.state)
            .into_iter()
            .filter_map(|sub| match sub {
                Subscription::Keyboard { key, description, .. } => Some((key, description)),
                _ => None,
            })
            .collect()
    }

    /// Get the app's title (static string)
    pub fn get_title(&self) -> &'static str {
        A::title()
    }

    /// Get the app's status (optional, dynamic)
    pub fn get_status(&self) -> Option<ratatui::text::Line<'static>> {
        A::status(&self.state, &self.config.theme)
    }

    /// Get a reference to the app's state
    pub fn get_state(&self) -> &A::State {
        &self.state
    }

    /// Get the currently focused element ID
    pub fn get_focused_id(&self) -> Option<&FocusId> {
        self.focused_id.as_ref()
    }

    /// Set the interaction registry (for mouse events after rendering)
    pub fn set_registry(&mut self, registry: InteractionRegistry<A::Msg>) {
        self.registry = registry;
    }

    /// Focus the next element (Tab)
    pub fn focus_next(&mut self) -> Result<()> {
        let focusable_ids = self.focus_registry.focusable_ids_in_active_layer();

        if focusable_ids.is_empty() {
            return Ok(());
        }

        let next_id = if let Some(current_id) = &self.focused_id {
            if let Some(pos) = focusable_ids.iter().position(|id| id == current_id) {
                focusable_ids[(pos + 1) % focusable_ids.len()].clone()
            } else {
                focusable_ids[0].clone()
            }
        } else {
            focusable_ids[0].clone()
        };

        let cmd = Command::set_focus(next_id);
        self.execute_command(cmd)?;
        Ok(())
    }

    /// Focus the previous element (Shift-Tab)
    pub fn focus_previous(&mut self) -> Result<()> {
        let focusable_ids = self.focus_registry.focusable_ids_in_active_layer();

        if focusable_ids.is_empty() {
            return Ok(());
        }

        let prev_id = if let Some(current_id) = &self.focused_id {
            if let Some(pos) = focusable_ids.iter().position(|id| id == current_id) {
                let prev_pos = if pos == 0 {
                    focusable_ids.len() - 1
                } else {
                    pos - 1
                };
                focusable_ids[prev_pos].clone()
            } else {
                focusable_ids[0].clone()
            }
        } else {
            focusable_ids[focusable_ids.len() - 1].clone()
        };

        let cmd = Command::set_focus(prev_id);
        self.execute_command(cmd)?;
        Ok(())
    }

    /// Poll timer subscriptions and fire those that are ready
    pub fn poll_timers(&mut self) -> Result<()> {
        let now = Instant::now();
        let mut messages = Vec::new();

        // Check which timers need to fire
        for (interval, last_tick, msg) in &mut self.timers {
            if now.duration_since(*last_tick) >= *interval {
                messages.push(msg.clone());
                *last_tick = now;
            }
        }

        // Execute messages
        for msg in messages {
            let command = A::update(&mut self.state, msg);
            self.execute_command(command)?;
        }

        Ok(())
    }

    /// Poll pending async commands and process completed ones
    pub async fn poll_async(&mut self) -> Result<()> {
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        // Create a dummy waker
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);

        // Poll regular async commands
        let mut completed = Vec::new();
        for (i, future) in self.pending_async.iter_mut().enumerate() {
            if let Poll::Ready(msg) = future.as_mut().poll(&mut cx) {
                completed.push((i, msg));
            }
        }

        // Remove completed futures (in reverse order to maintain indices)
        completed.sort_by(|a, b| b.0.cmp(&a.0));
        for (i, msg) in completed {
            self.pending_async.remove(i);
            let command = A::update(&mut self.state, msg);
            self.execute_command(command)?;
        }

        // Poll parallel coordination tasks
        let mut parallel_completed = Vec::new();
        for (i, future) in self.pending_parallel.iter_mut().enumerate() {
            if let Poll::Ready((task_idx, task_name)) = future.as_mut().poll(&mut cx) {
                parallel_completed.push((i, task_idx, task_name));
            }
        }

        // Process completed parallel tasks
        parallel_completed.sort_by(|a, b| b.0.cmp(&a.0));
        for (i, task_idx, task_name) in parallel_completed {
            self.pending_parallel.remove(i);

            // Publish task completion to LoadingScreen
            self.pending_publishes.push((
                "loading:progress".to_string(),
                serde_json::json!({
                    "task": task_name,
                    "status": "Completed",
                }),
            ));

            // Check if all tasks are complete
            if let Some(coordinator) = &self.parallel_coordinator {
                let results = coordinator.results.lock().unwrap();
                let all_complete = results.iter().all(|r| r.is_some());

                if all_complete {
                    // Get all results and apply them via msg_mapper
                    let total = coordinator.total_tasks;
                    let mapper = coordinator.msg_mapper.clone();
                    let config = coordinator.config.clone();

                    // Clone results out of the lock
                    let results_vec: Vec<_> = results.iter()
                        .enumerate()
                        .filter_map(|(idx, r)| {
                            if let Some(result) = r {
                                // We need to clone the Box<dyn Any + Send>
                                // This is tricky - we can't clone Any, so we pass ownership
                                // Actually, we can't take ownership here either
                                // Let's restructure to take results outside the lock
                                Some(idx)
                            } else {
                                None
                            }
                        })
                        .collect();

                    drop(results); // Release lock

                    // Take ownership of coordinator to get results
                    if let Some(coordinator) = self.parallel_coordinator.take() {
                        let mut results = coordinator.results.lock().unwrap();

                        // Apply each result via msg_mapper
                        for idx in 0..total {
                            if let Some(result) = results[idx].take() {
                                let msg = (coordinator.msg_mapper)(idx, result);
                                let command = A::update(&mut self.state, msg);
                                self.execute_command(command)?;
                            }
                        }

                        // Navigate to target if specified
                        if let Some(target) = config.on_complete {
                            self.navigation_target = Some(target);
                        }

                        // Clear all pending parallel futures to prevent re-polling
                        self.pending_parallel.clear();
                    }

                    break; // Exit loop since we took the coordinator
                }
            }
        }

        Ok(())
    }

    /// Update subscriptions based on current state
    fn update_subscriptions(&mut self) {
        self.key_subscriptions.clear();
        self.event_bus.clear();
        self.timers.clear();

        let subscriptions = A::subscriptions(&self.state);
        for sub in subscriptions {
            match sub {
                Subscription::Keyboard { key, msg, description: _ } => {
                    // description is used for help menus, not for runtime lookup
                    self.key_subscriptions.insert(key, msg);
                }
                Subscription::Subscribe { topic, handler } => {
                    self.event_bus
                        .entry(topic)
                        .or_insert_with(Vec::new)
                        .push(handler);
                }
                Subscription::Timer { interval, msg } => {
                    self.timers.push((interval, Instant::now(), msg));
                }
            }
        }
    }

    /// Handle a keyboard event
    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        if key_event.kind != KeyEventKind::Press {
            return Ok(true);
        }

        // Special handling for Escape: blur focused element first
        if key_event.code == KeyCode::Esc {
            if let Some(focused_id) = self.focused_id.take() {
                // Send blur message to focused element
                if let Some(focusable) = self.focus_registry.find_in_active_layer(&focused_id) {
                    if let Some(on_blur) = focusable.on_blur.clone() {
                        let command = A::update(&mut self.state, on_blur);
                        self.execute_command(command)?;
                    }
                }
                // Focus cleared, Escape consumed
                return Ok(true);
            }
            // Nothing focused, fall through to app subscriptions
        }

        // If there's a focused element, try routing the key to it first
        if let Some(focused_id) = &self.focused_id {
            if let Some(focusable) = self.focus_registry.find_in_active_layer(focused_id) {
                match (focusable.on_key)(key_event.code) {
                    DispatchTarget::WidgetEvent(boxed_event) => {
                        // Try widget auto-dispatch
                        if self.state.dispatch_widget_event(focused_id, boxed_event.as_ref()) {
                            return Ok(true);  // Handled!
                        }
                        // Not handled - widget event was not auto-dispatched
                        // This means the widget has no #[widget] attribute or doesn't match
                        // Just ignore the event (no-op)
                        return Ok(true);
                    }
                    DispatchTarget::AppMsg(msg) => {
                        // Direct to update()
                        let command = A::update(&mut self.state, msg);
                        return self.execute_command(command);
                    }
                }
            }
        }

        // No focused element handled it, check global subscriptions
        if let Some(msg) = self.key_subscriptions.get(&key_event.code).cloned() {
            let command = A::update(&mut self.state, msg);
            return self.execute_command(command);
        }

        Ok(true)
    }

    /// Handle a mouse event
    pub fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        let pos = (mouse_event.column, mouse_event.row);

        match mouse_event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // STEP 1: Handle focus change
                if let Some(clicked_id) = self.focus_registry.find_at_position(pos.0, pos.1) {
                    // Clicked on focusable element - focus it if not already focused
                    if self.focused_id.as_ref() != Some(&clicked_id) {
                        let cmd = Command::set_focus(clicked_id);
                        self.execute_command(cmd)?;
                    }
                } else {
                    // Clicked on non-focusable area - clear focus
                    if self.focused_id.is_some() {
                        let cmd = Command::clear_focus();
                        self.execute_command(cmd)?;
                    }
                }

                // STEP 2: Handle click action
                if let Some(msg) = self.registry.find_click(pos.0, pos.1) {
                    let command = A::update(&mut self.state, msg);
                    return self.execute_command(command);
                }
            }
            MouseEventKind::Moved => {
                // Handle focus-on-hover based on config
                match self.config.focus_mode {
                    FocusMode::Click => {
                        // Do nothing - focus only changes on click
                    }
                    FocusMode::Hover => {
                        // Always focus on hover
                        if let Some(hovered_id) = self.focus_registry.find_at_position(pos.0, pos.1) {
                            if self.focused_id.as_ref() != Some(&hovered_id) {
                                let cmd = Command::set_focus(hovered_id);
                                self.execute_command(cmd)?;
                            }
                        }
                    }
                    FocusMode::HoverWhenUnfocused => {
                        // Only focus on hover if nothing is currently focused
                        if self.focused_id.is_none() {
                            if let Some(hovered_id) = self.focus_registry.find_at_position(pos.0, pos.1) {
                                let cmd = Command::set_focus(hovered_id);
                                self.execute_command(cmd)?;
                            }
                        }
                    }
                }

                self.last_hover_pos = Some(pos);
            }
            MouseEventKind::ScrollUp => {
                // Scroll up - send as Up arrow key to focused element
                if let Some(focused_id) = &self.focused_id {
                    if let Some(focusable) = self.focus_registry.find_in_active_layer(focused_id) {
                        // Check if scroll happened over the focused element
                        if pos.0 >= focusable.rect.x
                            && pos.0 < focusable.rect.x + focusable.rect.width
                            && pos.1 >= focusable.rect.y
                            && pos.1 < focusable.rect.y + focusable.rect.height
                        {
                            match (focusable.on_key)(KeyCode::Up) {
                                DispatchTarget::WidgetEvent(boxed_event) => {
                                    if self.state.dispatch_widget_event(focused_id, boxed_event.as_ref()) {
                                        return Ok(true);
                                    }
                                    return Ok(true);
                                }
                                DispatchTarget::AppMsg(msg) => {
                                    let command = A::update(&mut self.state, msg);
                                    return self.execute_command(command);
                                }
                            }
                        }
                    }
                }
            }
            MouseEventKind::ScrollDown => {
                // Scroll down - send as Down arrow key to focused element
                if let Some(focused_id) = &self.focused_id {
                    if let Some(focusable) = self.focus_registry.find_in_active_layer(focused_id) {
                        // Check if scroll happened over the focused element
                        if pos.0 >= focusable.rect.x
                            && pos.0 < focusable.rect.x + focusable.rect.width
                            && pos.1 >= focusable.rect.y
                            && pos.1 < focusable.rect.y + focusable.rect.height
                        {
                            match (focusable.on_key)(KeyCode::Down) {
                                DispatchTarget::WidgetEvent(boxed_event) => {
                                    if self.state.dispatch_widget_event(focused_id, boxed_event.as_ref()) {
                                        return Ok(true);
                                    }
                                    return Ok(true);
                                }
                                DispatchTarget::AppMsg(msg) => {
                                    let command = A::update(&mut self.state, msg);
                                    return self.execute_command(command);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(true)
    }

    /// Handle a published event (from global or local event bus)
    pub fn handle_publish(&mut self, topic: &str, data: serde_json::Value) -> Result<()> {
        // Collect messages first to avoid borrow checker issues
        let messages: Vec<A::Msg> = if let Some(handlers) = self.event_bus.get(topic) {
            handlers
                .iter()
                .filter_map(|handler| handler(data.clone()))
                .collect()
        } else {
            Vec::new()
        };

        // Now execute the messages
        for msg in messages {
            let command = A::update(&mut self.state, msg);
            self.execute_command(command)?;
        }
        Ok(())
    }

    /// Execute a command
    fn execute_command(&mut self, command: Command<A::Msg>) -> Result<bool> {
        match command {
            Command::None => Ok(true),

            Command::Batch(commands) => {
                for cmd in commands {
                    if !self.execute_command(cmd)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }

            Command::Quit => Ok(false),

            Command::NavigateTo(app_id) => {
                // Store navigation target to be picked up by multi-runtime
                self.navigation_target = Some(app_id);
                Ok(true)
            }

            Command::Publish { topic, data } => {
                // Handle locally first
                self.handle_publish(&topic, data.clone())?;
                // Store for global broadcasting by MultiAppRuntime
                self.pending_publishes.push((topic, data));
                Ok(true)
            }

            Command::Perform(future) => {
                // Add to pending async commands
                self.pending_async.push(future);
                Ok(true)
            }

            Command::PerformParallel { tasks, config, msg_mapper } => {
                // Navigate to LoadingScreen immediately
                let task_names: Vec<String> = tasks.iter().map(|t| t.description.clone()).collect();
                let total_tasks = tasks.len();

                let loading_data = serde_json::json!({
                    "tasks": task_names,
                    "title": config.title.clone().unwrap_or_else(|| "Loading".to_string()),
                    "target": format!("{:?}", config.on_complete.unwrap_or(AppId::AppLauncher)),
                    "auto_complete": false, // We control completion manually
                });

                // Navigate to loading screen
                self.navigation_target = Some(AppId::LoadingScreen);
                self.pending_publishes.push(("loading:init".to_string(), loading_data));

                // Initialize shared state for task results
                let results = Arc::new(Mutex::new((0..total_tasks).map(|_| None).collect::<Vec<_>>()));
                let msg_mapper = Arc::new(msg_mapper);

                // Set up coordinator
                self.parallel_coordinator = Some(ParallelTaskCoordinator {
                    total_tasks,
                    results: results.clone(),
                    msg_mapper: msg_mapper.clone(),
                    config: config.clone(),
                });

                // Spawn all tasks as separate futures
                for (idx, task) in tasks.into_iter().enumerate() {
                    let results_ref = results.clone();
                    let task_name = task.description.clone();
                    let future = task.future;

                    // Immediately publish InProgress status
                    self.pending_publishes.push((
                        "loading:progress".to_string(),
                        serde_json::json!({
                            "task": task_name,
                            "status": "InProgress",
                        }),
                    ));

                    // Create wrapper future that stores result and signals completion
                    let wrapper_future = Box::pin(async move {
                        // Execute the actual task
                        let result = future.await;

                        // Store in shared results
                        {
                            let mut lock = results_ref.lock().unwrap();
                            lock[idx] = Some(result);
                        }

                        // Return task index and name for coordination
                        (idx, task_name)
                    });

                    self.pending_parallel.push(wrapper_future);
                }

                Ok(true)
            }

            Command::SetFocus(id) => {
                // Send blur to currently focused element (if any)
                if let Some(old_id) = self.focused_id.take() {
                    if let Some(focusable) = self.focus_registry.find_in_active_layer(&old_id) {
                        if let Some(on_blur) = focusable.on_blur.clone() {
                            let cmd = A::update(&mut self.state, on_blur);
                            self.execute_command(cmd)?;
                        }
                    }
                }

                // Set new focus
                self.focused_id = Some(id.clone());

                // Send focus message to new element
                if let Some(focusable) = self.focus_registry.find_in_active_layer(&id) {
                    if let Some(on_focus) = focusable.on_focus.clone() {
                        let cmd = A::update(&mut self.state, on_focus);
                        self.execute_command(cmd)?;
                    }
                }

                Ok(true)
            }

            Command::ClearFocus => {
                if let Some(old_id) = self.focused_id.take() {
                    if let Some(focusable) = self.focus_registry.find_in_active_layer(&old_id) {
                        if let Some(on_blur) = focusable.on_blur.clone() {
                            let cmd = A::update(&mut self.state, on_blur);
                            self.execute_command(cmd)?;
                        }
                    }
                }
                Ok(true)
            }
        }
    }

    /// Render the current app
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.size();
        self.render_to_area(frame, area);
    }

    /// Render the app to a specific area
    pub fn render_to_area(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Clear registries for this frame
        self.registry.clear();
        self.focus_registry.clear();
        self.dropdown_registry.clear();

        // Set current hover position for auto-hover tracking
        self.registry.set_hover_pos(self.last_hover_pos);

        // Get the layered view from the app
        let layered_view = A::view(&mut self.state, &self.config.theme);

        // Render using the new layered API
        Renderer::render_layers(
            frame,
            &self.config.theme,
            &mut self.registry,
            &mut self.focus_registry,
            self.focused_id.as_ref(),
            &layered_view,
            area,
            None, // No global UI area in single-app runtime
        );

        // Check if focused element still exists in the tree
        if let Some(focused_id) = &self.focused_id {
            if !self.focus_registry.contains(focused_id) {
                // Element removed while focused, clear focus
                self.focused_id = None;
            }
        }
    }
}

/// Blanket implementation of AppRuntime for Runtime<A>
/// This allows different Runtime<App> types to be stored in a type-erased collection
impl<A: App + 'static> AppRuntime for Runtime<A>
where
    A::State: 'static,
    A::Msg: 'static,
{
    fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        Runtime::handle_key(self, key_event)
    }

    fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        Runtime::handle_mouse(self, mouse_event)
    }

    fn render_to_area(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        Runtime::render_to_area(self, frame, area)
    }

    fn get_title(&self) -> &'static str {
        Runtime::get_title(self)
    }

    fn get_status(&self) -> Option<ratatui::text::Line<'static>> {
        Runtime::get_status(self)
    }

    fn get_key_bindings(&self) -> Vec<(KeyCode, String)> {
        Runtime::get_key_bindings(self)
    }

    fn poll_timers(&mut self) -> Result<()> {
        Runtime::poll_timers(self)
    }

    fn poll_async(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + '_>> {
        Box::pin(Runtime::poll_async(self))
    }

    fn take_navigation(&mut self) -> Option<AppId> {
        Runtime::take_navigation(self)
    }

    fn take_publishes(&mut self) -> Vec<(String, Value)> {
        Runtime::take_publishes(self)
    }

    fn handle_publish(&mut self, topic: &str, data: Value) -> Result<()> {
        Runtime::handle_publish(self, topic, data)
    }

    fn focus_next(&mut self) -> Result<()> {
        Runtime::focus_next(self)
    }

    fn focus_previous(&mut self) -> Result<()> {
        Runtime::focus_previous(self)
    }
}