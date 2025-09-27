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

use super::excel_parser::ExcelWorkbook;

pub async fn run_sheet_selector(file_path: String) -> Result<Option<(String, String)>> {
    // Load Excel file to get sheet names
    let workbook = match ExcelWorkbook::open(&file_path) {
        Ok(wb) => wb,
        Err(e) => {
            println!("Error opening Excel file: {}", e);
            return Ok(None);
        }
    };

    if workbook.sheets.is_empty() {
        println!("No sheets found in Excel file");
        return Ok(None);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_sheet_app(&mut terminal, workbook.sheets, file_path).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_sheet_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sheets: Vec<String>,
    file_path: String,
) -> Result<Option<(String, String)>> {
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(2),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(f.area());

            // Title
            let title = Paragraph::new("Select Excel Sheet")
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // File info
            let file_info = Paragraph::new(format!("File: {}", file_path))
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(file_info, chunks[1]);

            // Sheet list
            let items: Vec<ListItem> = sheets
                .iter()
                .map(|sheet| {
                    ListItem::new(Line::from(Span::styled(
                        sheet.clone(),
                        Style::default().fg(Color::White),
                    )))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Available Sheets"))
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                )
                .highlight_symbol("► ");
            f.render_stateful_widget(list, chunks[2], &mut list_state);

            // Instructions
            let instructions = Paragraph::new("Use ↑/↓ to navigate, Enter to select, Esc/q to quit")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(instructions, chunks[3]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                KeyCode::Down => {
                    let i = match list_state.selected() {
                        Some(i) => {
                            if i >= sheets.len() - 1 {
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
                                sheets.len() - 1
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
                        let selected_sheet = &sheets[i];
                        return Ok(Some((file_path, selected_sheet.clone())));
                    }
                }
                _ => {}
            }
        }
    }
}