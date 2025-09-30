use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    widgets::{Block, Borders, Paragraph},
    text::Line,
    style::Style,
};
use async_trait::async_trait;
use anyhow::Result;
use crossterm::event::KeyCode;

use crate::tui::{
    theme::Theme,
    app::{App, AppId, TuiMessage, AppMessage, HeaderContent, InteractionRegistry, Interaction, StartupContext},
};

pub struct Example1App {
    button_hovered: bool,
}

impl Example1App {
    pub fn new() -> Self {
        Self {
            button_hovered: false,
        }
    }
}

#[async_trait]
impl App for Example1App {
    fn id(&self) -> AppId {
        AppId::Example1
    }

    fn name(&self) -> &str {
        "Example 1"
    }

    async fn startup(&mut self, _context: StartupContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) {
    }

    async fn handle_key(&mut self, key: KeyCode) -> Option<TuiMessage> {
        match key {
            KeyCode::Char('2') => Some(TuiMessage::SwitchFocus(AppId::Example2)),
            _ => None,
        }
    }

    async fn handle_interaction(&mut self, element_id: &str, interaction: Interaction) -> Option<TuiMessage> {
        match (element_id, interaction) {
            ("nav_button", Interaction::Click) => Some(TuiMessage::SwitchFocus(AppId::Example2)),
            ("nav_button", Interaction::Hover) => {
                self.button_hovered = true;
                None
            }
            ("nav_button", Interaction::HoverExit) => {
                self.button_hovered = false;
                None
            }
            _ => None,
        }
    }

    async fn handle_message(&mut self, _message: AppMessage) -> Option<TuiMessage> {
        None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, registry: &mut InteractionRegistry) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .margin(2)
            .split(area);

        // Register the button area for mouse interaction
        registry.register("nav_button", chunks[0]);

        // Navigation button with hover state
        let button_block = if self.button_hovered {
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.lavender))
        } else {
            Block::default().borders(Borders::ALL).border_style(Style::default().fg(theme.overlay0))
        };

        let button = Paragraph::new("[ Press 2 or click to go to Example 2 ]")
            .block(button_block)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.text));
        frame.render_widget(button, chunks[0]);

        // Content area
        let content = Paragraph::new(vec![
            Line::from("This is the first example app."),
            Line::from(""),
            Line::from("Press 2 or click the button to go to Example 2."),
            Line::from(""),
            Line::from("Notice how mouse hover changes the button border!"),
            Line::from(""),
            Line::from("The InteractionRegistry automatically handles"),
            Line::from("mapping mouse coordinates to element IDs."),
        ])
        .style(Style::default().fg(theme.text));
        frame.render_widget(content, chunks[1]);
    }

    fn header_content(&self) -> HeaderContent {
        HeaderContent {
            title: self.name().to_string(),
            status: Some("Active".to_string()),
        }
    }
}