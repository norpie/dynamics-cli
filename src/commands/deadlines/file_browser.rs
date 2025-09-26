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
use std::{io, path::PathBuf, fs};

pub async fn run_file_browser(selected_env: String) -> Result<Option<String>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_file_app(&mut terminal, selected_env).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_file_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    selected_env: String,
) -> Result<Option<String>> {
    let mut current_path = std::env::current_dir()?;
    let mut list_state = ListState::default();
    list_state.select(Some(0));

    loop {
        let entries = get_directory_entries(&current_path)?;

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

            // Title with current path
            let title = Paragraph::new(format!("Select File - {}", current_path.display()))
                .style(Style::default().fg(Color::Cyan))
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(title, chunks[0]);

            // File/Directory list
            let items: Vec<ListItem> = entries
                .iter()
                .map(|(name, is_dir)| {
                    let display_name = if *is_dir {
                        format!("{}/", name)
                    } else {
                        name.clone()
                    };
                    let color = if *is_dir { Color::Blue } else { Color::White };
                    ListItem::new(Line::from(Span::styled(
                        display_name,
                        Style::default().fg(color),
                    )))
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Files & Directories"))
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
                    if !entries.is_empty() {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i >= entries.len() - 1 {
                                    0
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if !entries.is_empty() {
                        let i = match list_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    entries.len() - 1
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        list_state.select(Some(i));
                    }
                }
                KeyCode::Enter => {
                    if let Some(i) = list_state.selected() {
                        if i < entries.len() {
                            let (name, is_dir) = &entries[i];

                            if *is_dir {
                                if name == ".." {
                                    // Go up one directory
                                    if let Some(parent) = current_path.parent() {
                                        current_path = parent.to_path_buf();
                                    }
                                } else {
                                    // Enter directory
                                    current_path = current_path.join(name);
                                }
                                list_state.select(Some(0));
                            } else {
                                // File selected - return the full path
                                let file_path = current_path.join(name);
                                return Ok(Some(file_path.display().to_string()));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn get_directory_entries(path: &PathBuf) -> Result<Vec<(String, bool)>> {
    let mut entries = Vec::new();

    // Add parent directory entry if not at root
    if path.parent().is_some() {
        entries.push(("..".to_string(), true));
    }

    // Read directory entries
    let dir_entries = fs::read_dir(path)?;
    let mut dirs = Vec::new();
    let mut files = Vec::new();

    for entry in dir_entries {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type()?.is_dir();

        // Skip hidden files/directories
        if file_name.starts_with('.') && file_name != ".." {
            continue;
        }

        if is_dir {
            dirs.push((file_name, true));
        } else {
            files.push((file_name, false));
        }
    }

    // Sort directories and files separately
    dirs.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // Add sorted entries (directories first, then files)
    entries.extend(dirs);
    entries.extend(files);

    Ok(entries)
}