use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::io;

use crate::config::Config;

pub async fn run_auth_selector() -> Result<AuthSelectorResult> {
    // Load configuration
    let config = Config::load()?;

    if config.environments.is_empty() {
        println!("No authentication environments configured. Run 'dynamics-cli auth setup' first.");
        return Ok(AuthSelectorResult::Cancelled);
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

pub enum AuthSelectorResult {
    SelectedEnvironment(String),
    RerunSetup(String),
    Cancelled,
}

async fn run_auth_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: Config,
) -> Result<AuthSelectorResult> {
    let environments: Vec<String> = config.environments.keys().cloned().collect();
    let mut list_state = ListState::default();
    list_state.select(Some(0));
    let mut show_confirmation = false;

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
            let instruction_text = if show_confirmation {
                "Press Y to confirm re-run setup, N to cancel"
            } else {
                "Use ↑/↓ to navigate, Enter to select, 'r' to re-run setup, Esc/q to quit"
            };
            let instructions = Paragraph::new(instruction_text)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(instructions, chunks[2]);

            // Confirmation dialog
            if show_confirmation {
                if let Some(i) = list_state.selected() {
                    let selected_env = &environments[i];
                    draw_confirmation_dialog(f, f.area(), selected_env);
                }
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if show_confirmation {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(i) = list_state.selected() {
                            let selected_env = &environments[i];
                            return Ok(AuthSelectorResult::RerunSetup(selected_env.clone()));
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        show_confirmation = false;
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(AuthSelectorResult::Cancelled),
                    KeyCode::Down => {
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
                    KeyCode::Up => {
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
                            return Ok(AuthSelectorResult::SelectedEnvironment(selected_env.clone()));
                        }
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        show_confirmation = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn draw_confirmation_dialog(f: &mut ratatui::Frame, area: Rect, environment: &str) {
    let popup_area = centered_rect(60, 25, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Re-run Setup")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    let inner_area = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠️  Warning",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("This will re-run the deadline setup for environment '{}'", environment),
            Style::default().fg(Color::White)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This will overwrite your current deadline configuration",
            Style::default().fg(Color::Red)
        )),
        Line::from(Span::styled(
            "and you will need to reconfigure entity mappings.",
            Style::default().fg(Color::Red)
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Are you sure you want to continue?",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        )),
        Line::from(""),
        Line::from(""),
        Line::from(vec![
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(" Y ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::styled("] Confirm   ", Style::default().fg(Color::Gray)),
            Span::styled("[", Style::default().fg(Color::Gray)),
            Span::styled(" N ", Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled("] Cancel", Style::default().fg(Color::Gray)),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, inner_area);
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