use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    event::{DisableMouseCapture, EnableMouseCapture},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::{io, time::{Duration, Instant}};
use tokio::time::sleep;

pub struct LoadingSpinner {
    message: String,
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    frame_duration: Duration,
}

impl LoadingSpinner {
    pub fn new(message: String) -> Self {
        Self {
            message,
            frames: vec!["▁", "▃", "▄", "▅", "▆", "▇", "█", "▇", "▆", "▅", "▄", "▃", "▁"],
            current_frame: 0,
            last_update: Instant::now(),
            frame_duration: Duration::from_millis(100),
        }
    }

    fn update(&mut self) {
        if self.last_update.elapsed() >= self.frame_duration {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    fn render(&mut self, f: &mut Frame) {
        let popup_area = centered_rect(60, 20, f.area());

        // Clear the background
        f.render_widget(Clear, popup_area);

        // Create the popup block
        let popup_block = Block::default()
            .title("Loading")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Black));

        f.render_widget(popup_block, popup_area);

        let inner_area = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width - 2,
            height: popup_area.height - 2,
        };

        // Center the spinner and message
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Spinner
                Constraint::Length(2), // Message
            ])
            .split(inner_area);

        // Spinner
        let spinner_text = format!("{} Loading...", self.frames[self.current_frame]);
        let spinner = Paragraph::new(spinner_text)
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(spinner, chunks[0]);

        // Message
        let message_widget = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::Gray))
            .alignment(ratatui::layout::Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message_widget, chunks[1]);
    }
}

pub async fn show_loading_while<F, Fut, T>(
    message: String,
    future: F,
) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = show_loading_while_with_terminal(&mut terminal, message, future).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

pub async fn show_loading_while_with_terminal<B, F, Fut, T>(
    terminal: &mut Terminal<B>,
    message: String,
    future: F,
) -> Result<T>
where
    B: ratatui::backend::Backend,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut spinner = LoadingSpinner::new(message);

    // Start the async operation
    let future = future();
    tokio::pin!(future);

    let result = loop {
        // Update spinner
        spinner.update();

        // Draw spinner
        terminal.draw(|f| spinner.render(f))?;

        // Check if future is ready
        match tokio::time::timeout(Duration::from_millis(50), &mut future).await {
            Ok(result) => break result,
            Err(_) => {
                // Future not ready yet, continue spinning
                sleep(Duration::from_millis(50)).await;
            }
        }
    };

    result
}

// Helper function to create a centered rectangle
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