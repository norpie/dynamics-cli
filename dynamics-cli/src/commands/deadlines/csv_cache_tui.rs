use anyhow::Result;
use crossterm::{
    event::{self, poll, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use log::debug;

use super::config::EnvironmentConfig;
use super::csv_cache::{CacheStatus, CsvCacheManager, CacheProgressUpdate};
use super::csv_loading_modal::{CsvLoadingModal, CacheTaskStatus};

#[derive(Debug)]
enum CacheUserAction {
    Continue,
    Cancel,
    Refresh,
}

pub async fn run_csv_cache_check(
    environment_name: String,
    env_config: &EnvironmentConfig,
    auth_config: &crate::config::AuthConfig,
    force_refresh: bool,
) -> Result<bool> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_cache_app(&mut terminal, environment_name, env_config, auth_config, force_refresh).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_cache_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    environment_name: String,
    env_config: &EnvironmentConfig,
    auth_config: &crate::config::AuthConfig,
    mut force_refresh: bool,
) -> Result<bool> {
    loop {
        let cache_manager = CsvCacheManager::new(environment_name.clone());
        let initial_statuses = cache_manager.check_cache_status(env_config);

        debug!("Initial cache check complete. Found {} entities", initial_statuses.len());

        let needs_refresh = cache_manager.needs_refresh(&initial_statuses, force_refresh);

        if !needs_refresh {
            debug!("Cache is fresh, showing status screen");
            // Show cache status screen even when fresh
            let user_action = show_cache_status_screen(terminal, &initial_statuses).await?;
            match user_action {
                CacheUserAction::Continue => return Ok(true),
                CacheUserAction::Cancel => return Ok(false),
                CacheUserAction::Refresh => {
                    force_refresh = true;
                    continue; // Restart the loop with force refresh
                }
            }
        }

    debug!("Cache needs refresh, starting fetch process");

    // Collect entity names for the loading modal
    let entity_names: Vec<String> = initial_statuses.iter()
        .map(|status| status.entity_name.clone())
        .collect();

    let mut loading_modal = CsvLoadingModal::new(
        "📁 Refreshing CSV Cache".to_string(),
        &entity_names
    );

    let mut refresh_complete = false;

    // Create progress channel
    let (progress_sender, mut progress_receiver) = mpsc::unbounded_channel();

    let cache_manager_clone = cache_manager;
    let env_config_clone = env_config.clone();
    let auth_config_clone = auth_config.clone();

    tokio::spawn(async move {
        if let Err(e) = cache_manager_clone.refresh_cache_with_progress(&env_config_clone, &auth_config_clone, progress_sender).await {
            debug!("Cache refresh task failed: {}", e);
        }
    });

    loop {
        terminal.draw(|f| {
            loading_modal.tick();
            loading_modal.render(f, f.area());
        })?;

        // Check for progress updates
        while let Ok(update) = progress_receiver.try_recv() {
            match update {
                CacheProgressUpdate::EntityStarted(entity_name) => {
                    loading_modal.update_task_status(&entity_name, CacheTaskStatus::InProgress);
                    debug!("Started processing entity: {}", entity_name);
                }
                CacheProgressUpdate::EntityCompleted(entity_name) => {
                    loading_modal.update_task_status(&entity_name, CacheTaskStatus::Completed);
                    debug!("Completed processing entity: {}", entity_name);
                }
                CacheProgressUpdate::EntityFailed(entity_name, error) => {
                    loading_modal.update_task_status(&entity_name, CacheTaskStatus::Failed(error.clone()));
                    debug!("Failed processing entity {}: {}", entity_name, error);
                }
                CacheProgressUpdate::AllCompleted(_final_statuses) => {
                    refresh_complete = true;
                    debug!("All cache refresh tasks completed");

                    // Give a moment to show all completed checkboxes
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    return Ok(true); // Continue with fresh cache
                }
            }
        }

        // Handle user input
        if poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(false);
                    }
                    KeyCode::Enter => {
                        if refresh_complete || loading_modal.has_error() {
                            return Ok(true);
                        }
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
        }
    } // end of main loop
}

async fn show_cache_status_screen(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    statuses: &[CacheStatus],
) -> Result<CacheUserAction> {
    use ratatui::{
        layout::{Alignment, Constraint, Direction, Layout, Margin},
        style::{Color, Style},
        text::{Line, Span},
        widgets::{Block, Borders, List, ListItem, Paragraph},
    };

    loop {
        terminal.draw(|f| {
            let size = f.area();

            // Main block
            let main_block = Block::default()
                .title("📁 CSV Cache Status")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Green));

            f.render_widget(main_block, size);

            // Inner area
            let inner_area = size.inner(Margin::new(2, 1));

            // Split into sections
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(1),    // Entity list
                    Constraint::Length(3), // Instructions
                ])
                .split(inner_area);

            // Header
            let header_text = vec![
                Line::from(vec![
                    Span::styled("✅ All ", Style::default().fg(Color::Green)),
                    Span::styled(format!("{}", statuses.len()), Style::default().fg(Color::White)),
                    Span::styled(" entities are cached and fresh", Style::default().fg(Color::Green)),
                ]),
                Line::from(""),
            ];

            let header = Paragraph::new(header_text)
                .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Entity list with dates
            let entity_items: Vec<ListItem> = statuses
                .iter()
                .map(|status| {
                    let age_text = if let Some(file_path) = &status.file_path {
                        if let Ok(metadata) = std::fs::metadata(file_path) {
                            if let Ok(modified) = metadata.modified() {
                                let age = modified.elapsed().unwrap_or_default();
                                if age.as_secs() < 60 {
                                    format!("{}s ago", age.as_secs())
                                } else if age.as_secs() < 3600 {
                                    format!("{}m ago", age.as_secs() / 60)
                                } else if age.as_secs() < 86400 {
                                    format!("{}h ago", age.as_secs() / 3600)
                                } else {
                                    format!("{}d ago", age.as_secs() / 86400)
                                }
                            } else {
                                "unknown".to_string()
                            }
                        } else {
                            "missing".to_string()
                        }
                    } else {
                        "no file".to_string()
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled("• ", Style::default().fg(Color::Green)),
                        Span::styled(format!("{:<20}", status.entity_name), Style::default()),
                        Span::styled(format!("{:>6} records", status.record_count), Style::default().fg(Color::Cyan)),
                        Span::styled(format!("  ({})", age_text), Style::default().fg(Color::Gray)),
                    ]))
                })
                .collect();

            let entity_list = List::new(entity_items)
                .block(
                    Block::default()
                        .title("Cached Entities")
                        .borders(Borders::ALL)
                );
            f.render_widget(entity_list, chunks[1]);

            // Instructions
            let instructions = vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", Style::default()),
                    Span::styled("Enter", Style::default().fg(Color::Green)),
                    Span::styled(" to continue, ", Style::default()),
                    Span::styled("R", Style::default().fg(Color::Yellow)),
                    Span::styled(" to refresh cache, or ", Style::default()),
                    Span::styled("Esc", Style::default().fg(Color::Red)),
                    Span::styled(" to cancel", Style::default()),
                ]),
            ];

            let instructions_para = Paragraph::new(instructions)
                .alignment(Alignment::Center);
            f.render_widget(instructions_para, chunks[2]);
        })?;

        // Handle user input
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        return Ok(CacheUserAction::Continue);
                    }
                    KeyCode::Char('r') | KeyCode::Char('R') => {
                        return Ok(CacheUserAction::Refresh);
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        return Ok(CacheUserAction::Cancel);
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

