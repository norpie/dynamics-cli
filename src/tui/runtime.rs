use ratatui::Frame;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind, MouseButton};
use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde_json::Value;
use std::pin::Pin;
use std::future::Future;

use crate::tui::{App, AppId, Command, Renderer, InteractionRegistry, Theme, ThemeVariant, Subscription};

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
}

/// The runtime manages app lifecycle, event routing, and command execution
pub struct Runtime<A: App> {
    /// Current app state
    state: A::State,

    /// Theme for rendering
    theme: Theme,

    /// Interaction registry for mouse events
    registry: InteractionRegistry<A::Msg>,

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

    /// Pending publish events to broadcast globally
    pending_publishes: Vec<(String, serde_json::Value)>,
}

impl<A: App> Runtime<A> {
    pub fn new() -> Self {
        let (state, init_command) = A::init();
        let theme = Theme::new(ThemeVariant::default());

        let mut runtime = Self {
            state,
            theme,
            registry: InteractionRegistry::new(),
            key_subscriptions: HashMap::new(),
            event_bus: HashMap::new(),
            timers: Vec::new(),
            last_hover_pos: None,
            navigation_target: None,
            pending_async: Vec::new(),
            pending_publishes: Vec::new(),
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
        A::status(&self.state, &self.theme)
    }

    /// Get a reference to the app's state
    pub fn get_state(&self) -> &A::State {
        &self.state
    }

    /// Set the interaction registry (for mouse events after rendering)
    pub fn set_registry(&mut self, registry: InteractionRegistry<A::Msg>) {
        self.registry = registry;
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

        // Check if we have a subscription for this key
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
                if let Some(msg) = self.registry.find_click(pos.0, pos.1) {
                    let command = A::update(&mut self.state, msg);
                    return self.execute_command(command);
                }
            }
            MouseEventKind::Moved => {
                // Handle hover exit if we moved to a different element
                if let Some(last_pos) = self.last_hover_pos {
                    if last_pos != pos {
                        if let Some(msg) = self.registry.find_hover_exit(last_pos.0, last_pos.1) {
                            let command = A::update(&mut self.state, msg);
                            self.execute_command(command)?;
                        }
                    }
                }

                // Handle hover enter
                if let Some(msg) = self.registry.find_hover(pos.0, pos.1) {
                    let command = A::update(&mut self.state, msg);
                    self.execute_command(command)?;
                }

                self.last_hover_pos = Some(pos);
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
        }
    }

    /// Render the current app
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.size();
        self.render_to_area(frame, area);
    }

    /// Render the app to a specific area
    pub fn render_to_area(&mut self, frame: &mut Frame, area: ratatui::layout::Rect) {
        // Clear registry for this frame
        self.registry.clear();

        // Get the view from the app
        let view = A::view(&self.state, &self.theme);

        // Render the view
        Renderer::render(frame, &self.theme, &mut self.registry, &view, area);
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
}