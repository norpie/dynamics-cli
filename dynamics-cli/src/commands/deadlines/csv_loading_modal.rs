use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum CacheTaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug)]
pub struct CacheTask {
    pub name: String,
    pub status: CacheTaskStatus,
}

#[derive(Debug)]
pub struct CsvLoadingModal {
    title: String,
    tasks: Vec<CacheTask>,
    spinner_state: usize,
    error_message: Option<String>,
}

impl CsvLoadingModal {
    const SPINNER_FRAMES: &'static [&'static str] = &[
        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
    ];

    pub fn new(title: String, entity_names: &[String]) -> Self {
        let tasks = entity_names
            .iter()
            .map(|name| CacheTask {
                name: name.clone(),
                status: CacheTaskStatus::Pending,
            })
            .collect();

        Self {
            title,
            tasks,
            spinner_state: 0,
            error_message: None,
        }
    }

    pub fn update_task_status(&mut self, entity_name: &str, status: CacheTaskStatus) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.name == entity_name) {
            task.status = status;
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
    }

    pub fn tick(&mut self) {
        if self.error_message.is_none() {
            self.spinner_state = (self.spinner_state + 1) % Self::SPINNER_FRAMES.len();
        }
    }

    pub fn all_completed(&self) -> bool {
        self.tasks.iter().all(|task| matches!(task.status, CacheTaskStatus::Completed))
    }

    pub fn has_failures(&self) -> bool {
        self.tasks.iter().any(|task| matches!(task.status, CacheTaskStatus::Failed(_)))
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(70, 60, area);

        f.render_widget(Clear, popup_area);

        let title = if self.error_message.is_some() {
            "❌ Cache Refresh Failed"
        } else if self.all_completed() {
            "✅ Cache Refresh Complete"
        } else {
            &self.title
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let inner_area = block.inner(popup_area);
        f.render_widget(block, popup_area);

        if let Some(error) = &self.error_message {
            self.render_error_state(f, inner_area, error);
        } else {
            self.render_progress_state(f, inner_area);
        }
    }

    fn render_progress_state(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Min(4),    // Task list
                Constraint::Length(2), // Footer
            ])
            .split(area);

        // Header with spinner
        let header_text = if self.all_completed() {
            "All entities cached successfully!".to_string()
        } else {
            format!(
                "{} Fetching entity data...",
                Self::SPINNER_FRAMES[self.spinner_state]
            )
        };

        let header = Paragraph::new(header_text)
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center);
        f.render_widget(header, chunks[0]);

        // Task list
        let task_items: Vec<ListItem> = self
            .tasks
            .iter()
            .map(|task| self.create_task_item(task))
            .collect();

        let task_list = List::new(task_items)
            .style(Style::default());
        f.render_widget(task_list, chunks[1]);

        // Footer
        let footer_text = if self.all_completed() {
            "Press any key to continue..."
        } else if self.has_failures() {
            "Some entities failed to cache. Press any key to continue..."
        } else {
            "Please wait while data is being cached..."
        };

        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(footer, chunks[2]);
    }

    fn render_error_state(&self, f: &mut Frame, area: Rect, error: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Error message
                Constraint::Min(2),    // Error details
                Constraint::Length(2), // Instructions
            ])
            .split(area);

        let error_header = Paragraph::new("Cache refresh was cancelled or failed:")
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center);
        f.render_widget(error_header, chunks[0]);

        let error_details = Paragraph::new(error)
            .style(Style::default().fg(Color::Red))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(error_details, chunks[1]);

        let instructions = Paragraph::new("Press any key to continue...")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(instructions, chunks[2]);
    }

    fn create_task_item<'a>(&self, task: &'a CacheTask) -> ListItem<'a> {
        let (symbol, style) = match &task.status {
            CacheTaskStatus::Pending => {
                ("◯", Style::default().fg(Color::Gray))
            }
            CacheTaskStatus::InProgress => {
                (Self::SPINNER_FRAMES[self.spinner_state], Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            }
            CacheTaskStatus::Completed => {
                ("✓", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            }
            CacheTaskStatus::Failed(_err) => {
                ("❌", Style::default().fg(Color::Red))
            }
        };

        let line = Line::from(vec![
            Span::styled(format!(" {} ", symbol), style),
            Span::styled(&task.name, style),
        ]);

        ListItem::new(line)
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