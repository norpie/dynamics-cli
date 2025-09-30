use std::collections::{HashMap, VecDeque};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    style::Style,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind, MouseButton};
use anyhow::Result;

use crate::tui::{
    theme::{Theme, ThemeVariant},
    app::{App, AppId, TuiMessage, AppMessage, HeaderContent, InteractionRegistry, Interaction, ScrollDirection, StartupContext},
};

pub struct TuiOrchestrator {
    theme: Theme,
    focus_stack: Vec<AppId>,
    apps: HashMap<AppId, Box<dyn App>>,
    message_queues: HashMap<AppId, VecDeque<AppMessage>>,
    interaction_registry: InteractionRegistry,
    last_hovered_element: Option<String>,
}

impl TuiOrchestrator {
    pub async fn new() -> Result<Self> {
        let theme = Theme::new(ThemeVariant::default());

        let mut orchestrator = Self {
            theme,
            focus_stack: vec![AppId::Example1],
            apps: HashMap::new(),
            message_queues: HashMap::new(),
            interaction_registry: InteractionRegistry::new(),
            last_hovered_element: None,
        };

        // Start the initial app
        orchestrator.start_app(AppId::Example1).await?;

        Ok(orchestrator)
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<bool> {
        match event {
            Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) => {
                match code {
                    KeyCode::F(1) => {
                        // TODO: Show help modal
                        return Ok(true);
                    }
                    other_key => {
                        // Route to focused app
                        if let Some(focused_app_id) = self.current_focus() {
                            if let Some(app) = self.apps.get_mut(&focused_app_id) {
                                if let Some(message) = app.handle_key(other_key).await {
                                    if !self.handle_tui_message(message).await? {
                                        return Ok(false);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Event::Mouse(mouse_event) => {
                return self.handle_mouse_event(mouse_event).await;
            }
            _ => {}
        }

        // Process any pending messages
        let responses = self.process_pending_messages().await?;
        for response in responses {
            if !self.handle_tui_message(response).await? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn handle_mouse_event(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        let mouse_pos = (mouse_event.column, mouse_event.row);
        
        // Find which element the mouse is over
        let current_element = self.interaction_registry.find_element(mouse_pos).map(|s| s.to_string());
        
        // Handle hover exit events
        if let Some(last_element) = &self.last_hovered_element {
            if current_element.as_ref() != Some(last_element) {
                // Mouse left the previously hovered element
                if let Some(focused_app_id) = self.current_focus() {
                    if let Some(app) = self.apps.get_mut(&focused_app_id) {
                        if let Some(message) = app.handle_interaction(last_element, Interaction::HoverExit).await {
                            self.handle_tui_message(message).await?;
                        }
                    }
                }
            }
        }

        // Handle current mouse event
        if let Some(element_id) = &current_element {
            let interaction = match mouse_event.kind {
                MouseEventKind::Down(MouseButton::Left) => Interaction::Click,
                MouseEventKind::Down(MouseButton::Right) => Interaction::RightClick,
                MouseEventKind::ScrollUp => Interaction::Scroll(ScrollDirection::Up),
                MouseEventKind::ScrollDown => Interaction::Scroll(ScrollDirection::Down),
                MouseEventKind::Moved => Interaction::Hover,
                _ => {
                    self.last_hovered_element = current_element;
                    return Ok(true);
                }
            };

            if let Some(focused_app_id) = self.current_focus() {
                if let Some(app) = self.apps.get_mut(&focused_app_id) {
                    if let Some(message) = app.handle_interaction(element_id, interaction).await {
                        let result = self.handle_tui_message(message).await?;
                        self.last_hovered_element = current_element;
                        return Ok(result);
                    }
                }
            }
        }

        self.last_hovered_element = current_element;
        Ok(true)
    }

    async fn handle_tui_message(&mut self, message: TuiMessage) -> Result<bool> {
        match message {
            TuiMessage::SwitchFocus(app_id) => {
                if !self.apps.contains_key(&app_id) {
                    self.start_app(app_id).await?;
                }
                self.focus_stack.push(app_id);
                Ok(true)
            }
            TuiMessage::YieldFocus => {
                if self.focus_stack.len() > 1 {
                    self.focus_stack.pop();
                }
                Ok(true)
            }
            TuiMessage::SendMessage { target, message } => {
                self.send_message_to_app(target, message).await?;
                Ok(true)
            }
            TuiMessage::KillApp(app_id) => {
                self.shutdown_app(app_id).await?;
                Ok(true)
            }
            TuiMessage::Quit => {
                Ok(false)
            }
        }
    }

    async fn start_app(&mut self, app_id: AppId) -> Result<()> {
        let mut app: Box<dyn App> = match app_id {
            AppId::Example1 => Box::new(crate::tui::apps::example1::Example1App::new()),
            AppId::Example2 => Box::new(crate::tui::apps::example2::Example2App::new()),
            _ => return Err(anyhow::anyhow!("App {:?} not implemented yet", app_id)),
        };

        let context = StartupContext {};
        app.startup(context).await?;
        self.apps.insert(app_id, app);

        Ok(())
    }

    async fn send_message_to_app(&mut self, target: AppId, message: AppMessage) -> Result<()> {
        if !self.apps.contains_key(&target) {
            self.start_app(target).await?;
        }
        self.message_queues.entry(target).or_default().push_back(message);
        Ok(())
    }

    async fn process_pending_messages(&mut self) -> Result<Vec<TuiMessage>> {
        let mut messages_to_process = Vec::new();
        let mut responses = Vec::new();

        for (app_id, queue) in &mut self.message_queues {
            while let Some(message) = queue.pop_front() {
                messages_to_process.push((*app_id, message));
            }
        }

        for (app_id, message) in messages_to_process {
            if let Some(app) = self.apps.get_mut(&app_id) {
                if let Some(response) = app.handle_message(message).await {
                    responses.push(response);
                }
            }
        }

        Ok(responses)
    }

    async fn shutdown_app(&mut self, app_id: AppId) -> Result<()> {
        if let Some(mut app) = self.apps.remove(&app_id) {
            if app.can_exit() {
                app.shutdown().await;
            } else {
                self.apps.insert(app_id, app);
                return Err(anyhow::anyhow!("App {:?} cannot exit yet", app_id));
            }
        }

        self.message_queues.remove(&app_id);
        self.focus_stack.retain(|id| *id != app_id);

        if self.focus_stack.is_empty() {
            return Err(anyhow::anyhow!("No apps remaining"));
        }

        Ok(())
    }

    fn current_focus(&self) -> Option<AppId> {
        self.focus_stack.last().copied()
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),      // Header
                Constraint::Min(0),         // Body (flexible)
                Constraint::Length(3),      // Footer
            ])
            .split(frame.size());

        self.render_header(frame, chunks[0]);

        // Clear registry before rendering
        self.interaction_registry.clear();

        if let Some(focused_app_id) = self.current_focus() {
            if let Some(app) = self.apps.get_mut(&focused_app_id) {
                app.render(frame, chunks[1], &self.theme, &mut self.interaction_registry);
            }
        }

        self.render_footer(frame, chunks[2]);
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let title = if let Some(focused_app_id) = self.current_focus() {
            if let Some(app) = self.apps.get(&focused_app_id) {
                let content = app.header_content();
                if let Some(status) = content.status {
                    format!("Dynamics CLI - {} | {}", content.title, status)
                } else {
                    format!("Dynamics CLI - {}", content.title)
                }
            } else {
                "Dynamics CLI".to_string()
            }
        } else {
            "Dynamics CLI".to_string()
        };

        let header = Paragraph::new(title)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(self.theme.text));

        frame.render_widget(header, area);
    }

    fn render_footer(&mut self, frame: &mut Frame, area: Rect) {
        let help_text = "F1: Help | Ctrl+Q: Quit | Mouse: Click to interact";

        let footer = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(self.theme.subtext1));

        frame.render_widget(footer, area);
    }
}