use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io;

use crate::config::Config;

pub async fn run_auth_selector() -> Result<Option<String>> {
    // Load configuration
    let config = Config::load()?;

    if config.environments.is_empty() {
        println!("No authentication environments configured. Run 'dynamics-cli auth setup' first.");
        return Ok(None);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_auth_app(&mut terminal, config).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_auth_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> Result<Option<String>> {
    let environments: Vec<String> = config.environments.keys().cloned().collect();
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Title
            let title = Paragraph::new("Select Authentication Environment")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // Environment list
            let items: Vec<ListItem> = environments
                .iter()
                .map(|env| {
                    ListItem::new(Line::from(Span::styled(
                        env.clone(),
                        Style::default().fg(Color::White),
                    )))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Environments"))
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("► ");
            f.render_stateful_widget(list, chunks[1], &mut list_state);

            // Instructions
            let instructions = Paragraph::new("Use ↑/↓ to navigate, Enter to select, Esc/q to quit")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(instructions, chunks[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = match list_state.selected() {
                        Some(i) => {
                            if i >= environments.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    list_state.select(Some(i));
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = match list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                environments.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    list_state.select(Some(i));
                }
                KeyCode::Enter => {
                    if let Some(i) = list_state.selected() {
                        let selected_env = &environments[i];
                        return Ok(Some(selected_env.clone()));
                    }
                }
                _ => {}
            }
        }
    }
}