use crate::commands::migration::ui::components::modal_component::{ModalContent, ModalContentAction};
use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
};

#[derive(Debug, Clone)]
pub enum ConfirmationAction {
    Confirmed,
    Cancelled,
}

pub struct ConfirmationDialog {
    title: String,
    message: String,
    confirm_button: String,
    cancel_button: String,
    selected_button: usize, // 0 = confirm, 1 = cancel
    action: Option<ConfirmationAction>,
}

impl ConfirmationDialog {
    pub fn new(title: String, message: String) -> Self {
        Self {
            title,
            message,
            confirm_button: "Yes".to_string(),
            cancel_button: "No".to_string(),
            selected_button: 1, // Default to "No" (safer)
            action: None,
        }
    }

    pub fn with_buttons(mut self, confirm: String, cancel: String) -> Self {
        self.confirm_button = confirm;
        self.cancel_button = cancel;
        self
    }

    pub fn take_action(&mut self) -> Option<ConfirmationAction> {
        self.action.take()
    }
}

impl ModalContent for ConfirmationDialog {
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3), // Message area
                Constraint::Length(3), // Button area
            ])
            .split(area);

        // Render message
        let message_paragraph = Paragraph::new(self.message.clone())
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        f.render_widget(message_paragraph, chunks[0]);

        // Render buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(chunks[1]);

        // Confirm button
        let confirm_style = if self.selected_button == 0 {
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let confirm_button = Paragraph::new(self.confirm_button.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(confirm_style),
            )
            .style(confirm_style)
            .alignment(Alignment::Center);
        f.render_widget(confirm_button, button_chunks[0]);

        // Cancel button
        let cancel_style = if self.selected_button == 1 {
            Style::default()
                .fg(Color::White)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cancel_button = Paragraph::new(self.cancel_button.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(cancel_style),
            )
            .style(cancel_style)
            .alignment(Alignment::Center);
        f.render_widget(cancel_button, button_chunks[1]);
    }

    fn handle_key(&mut self, key: KeyCode) -> ModalContentAction {
        match key {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                self.selected_button = if self.selected_button == 0 { 1 } else { 0 };
                ModalContentAction::None
            }
            KeyCode::Enter => {
                self.action = if self.selected_button == 0 {
                    Some(ConfirmationAction::Confirmed)
                } else {
                    Some(ConfirmationAction::Cancelled)
                };
                ModalContentAction::Close
            }
            KeyCode::Esc => {
                self.action = Some(ConfirmationAction::Cancelled);
                ModalContentAction::Close
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.action = Some(ConfirmationAction::Confirmed);
                ModalContentAction::Close
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.action = Some(ConfirmationAction::Cancelled);
                ModalContentAction::Close
            }
            _ => ModalContentAction::None,
        }
    }

    fn handle_mouse(&mut self, _event: MouseEvent, _area: Rect) -> ModalContentAction {
        // Basic mouse support could be added here
        ModalContentAction::None
    }

    fn get_title(&self) -> Option<String> {
        Some(self.title.clone())
    }
}