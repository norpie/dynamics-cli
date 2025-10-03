use super::fetch_progress::{FetchProgress, FetchStatus};
use crate::commands::migration::ui::styles::STYLES;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct LoadingModalComponent {
    message: String,
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    frame_duration: Duration,
    progress: Arc<Mutex<FetchProgress>>,
    state: LoadingState,
}

#[derive(Debug, Clone)]
pub enum LoadingState {
    Fetching,
    Failed(Vec<String>),
}

impl LoadingModalComponent {
    pub fn new(message: String, progress: Arc<Mutex<FetchProgress>>) -> Self {
        Self {
            message,
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(100),
            progress,
            state: LoadingState::Fetching,
        }
    }

    pub fn set_state(&mut self, state: LoadingState) {
        self.state = state;
    }

    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.frame_duration {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(70, 50, area);

        f.render_widget(Clear, popup_area);

        let popup_block = Block::default()
            .title(match self.state {
                LoadingState::Fetching => "Fetching Data",
                LoadingState::Failed(_) => "❌ Failed to fetch data",
            })
            .borders(Borders::ALL)
            .style(STYLES.normal);

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        match &self.state {
            LoadingState::Fetching => self.render_fetching_state(f, inner_area),
            LoadingState::Failed(errors) => self.render_error_state(f, inner_area, errors),
        }
    }

    fn render_fetching_state(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Spinner
                Constraint::Length(3), // Message
                Constraint::Min(4),    // Progress list
            ])
            .split(area);

        let spinner_text = format!("{} Loading...", self.frames[self.current_frame]);
        let spinner = Paragraph::new(spinner_text)
            .style(STYLES.highlighted)
            .alignment(Alignment::Center);
        f.render_widget(spinner, chunks[0]);

        let message_widget = Paragraph::new(self.message.clone())
            .style(STYLES.info)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message_widget, chunks[1]);

        let progress = self.progress.lock().unwrap();
        let items = vec![
            self.create_progress_item("Source entity fields", &progress.source_fields),
            self.create_progress_item("Target entity fields", &progress.target_fields),
            self.create_progress_item("Source entity views", &progress.source_views),
            self.create_progress_item("Target entity views", &progress.target_views),
            self.create_progress_item("Source entity forms", &progress.source_forms),
            self.create_progress_item("Target entity forms", &progress.target_forms),
            self.create_progress_item("Example records", &progress.examples),
        ];

        let progress_list = List::new(items).style(STYLES.normal);
        f.render_widget(progress_list, chunks[2]);
    }

    fn render_error_state(&self, f: &mut Frame, area: Rect, errors: &[String]) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Error message
                Constraint::Min(4),    // Error list
                Constraint::Length(2), // Instructions
            ])
            .split(area);

        let error_message = Paragraph::new("The following errors occurred while fetching data:")
            .style(STYLES.error)
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(error_message, chunks[0]);

        let error_items: Vec<ListItem> = errors
            .iter()
            .map(|error| {
                ListItem::new(Line::from(vec![
                    Span::styled("• ", STYLES.error),
                    Span::styled(error, STYLES.error),
                ]))
            })
            .collect();

        let error_list = List::new(error_items).style(STYLES.error);
        f.render_widget(error_list, chunks[1]);

        let instructions = Paragraph::new("Press Esc to go back and try again.")
            .style(STYLES.info)
            .alignment(Alignment::Center);
        f.render_widget(instructions, chunks[2]);
    }

    fn create_progress_item<'a>(&self, text: &'a str, status: &FetchStatus) -> ListItem<'a> {
        let (symbol, color) = match status {
            FetchStatus::Pending => ("◯", STYLES.disabled),
            FetchStatus::InProgress => (self.frames[self.current_frame], STYLES.highlighted),
            FetchStatus::Completed => ("✓", STYLES.success),
            FetchStatus::Failed(_) => ("❌", STYLES.error),
        };

        ListItem::new(Line::from(vec![
            Span::styled(format!(" {} ", symbol), color),
            Span::styled(text, color),
        ]))
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
