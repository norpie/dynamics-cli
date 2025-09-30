use ratatui::Frame;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind, MouseButton};
use anyhow::Result;
use std::collections::HashMap;
use serde_json::Value;

use crate::tui::{App, AppId, Command, Renderer, InteractionRegistry, Theme, ThemeVariant, Subscription};

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

    /// Last hovered element position for tracking hover exits
    last_hover_pos: Option<(u16, u16)>,

    /// Pending navigation request
    navigation_target: Option<AppId>,

    /// Pending async commands
    pending_async: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = A::Msg> + Send>>>,
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
            last_hover_pos: None,
            navigation_target: None,
            pending_async: Vec::new(),
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

    /// Check if there are any pending async commands
    pub fn has_pending_async(&self) -> bool {
        !self.pending_async.is_empty()
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

        let subscriptions = A::subscriptions(&self.state);
        for sub in subscriptions {
            match sub {
                Subscription::Keyboard { key, msg } => {
                    self.key_subscriptions.insert(key, msg);
                }
                Subscription::Subscribe { topic, handler } => {
                    self.event_bus
                        .entry(topic)
                        .or_insert_with(Vec::new)
                        .push(handler);
                }
                Subscription::Timer { .. } => {
                    // TODO: Implement timer subscriptions
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
                // Publish to event bus
                // Collect messages first to avoid borrow checker issues
                let messages: Vec<A::Msg> = if let Some(handlers) = self.event_bus.get(&topic) {
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
        // Clear registry for this frame
        self.registry.clear();

        // Get the view from the app
        let view = A::view(&self.state, &self.theme);

        // Get the frame size
        let area = frame.size();

        // Render the view
        Renderer::render(frame, &self.theme, &mut self.registry, &view, area);

        // Update subscriptions after rendering (in case state affects subscriptions)
        self.update_subscriptions();
    }
}