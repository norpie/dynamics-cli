use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug)]
pub struct LoadingModal {
    message: String,
    spinner_state: usize,
    error_message: Option<String>,
}

impl LoadingModal {
    const SPINNER_FRAMES: &'static [&'static str] = &[
        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
    ];

    pub fn new(message: String) -> Self {
        Self {
            message,
            spinner_state: 0,
            error_message: None,
        }
    }

    pub fn set_message(&mut self, message: String) {
        self.message = message;
        self.error_message = None;
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    pub fn tick(&mut self) {
        if self.error_message.is_none() {
            self.spinner_state = (self.spinner_state + 1) % Self::SPINNER_FRAMES.len();
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(60, 20, area);

        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .title("Loading")
            .borders(Borders::ALL);

        let inner_area = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let content = if let Some(error) = &self.error_message {
            vec![
                Line::from(vec![
                    Span::styled("✗ ", Style::default().fg(Color::Red)),
                    Span::styled("Error", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(""),
                Line::from(Span::styled(error, Style::default().fg(Color::Red))),
                Line::from(""),
                Line::from(Span::styled("Press any key to continue...", Style::default().fg(Color::Gray))),
            ]
        } else {
            vec![
                Line::from(vec![
                    Span::styled(
                        Self::SPINNER_FRAMES[self.spinner_state],
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(" Loading...", Style::default().fg(Color::Cyan)),
                ]),
                Line::from(""),
                Line::from(Span::styled(&self.message, Style::default().fg(Color::White))),
            ]
        };

        let paragraph = Paragraph::new(content)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        f.render_widget(paragraph, inner_area);
    }

    pub fn has_error(&self) -> bool {
        self.error_message.is_some()
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}