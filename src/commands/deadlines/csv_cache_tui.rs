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
use tokio::sync::oneshot;
use log::debug;

use super::config::EnvironmentConfig;
use super::csv_cache::{CacheStatus, CacheState, CsvCacheManager};
use super::loading_modal::LoadingModal;

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
    force_refresh: bool,
) -> Result<bool> {
    let cache_manager = CsvCacheManager::new(environment_name.clone());
    let initial_statuses = cache_manager.check_cache_status(env_config);

    debug!("Initial cache check complete. Found {} entities", initial_statuses.len());

    let needs_refresh = cache_manager.needs_refresh(&initial_statuses, force_refresh);

    if !needs_refresh {
        debug!("Cache is fresh, no refresh needed");
        return Ok(true);
    }

    debug!("Cache needs refresh, starting fetch process");

    let mut loading_modal = LoadingModal::new(format!("Checking CSV cache for {} entities...", initial_statuses.len()));
    let mut current_entity_index = 0;
    let mut refresh_complete = false;
    let mut refresh_receiver: Option<oneshot::Receiver<Vec<CacheStatus>>> = None;

    // Start refresh task
    let (sender, receiver) = oneshot::channel();
    refresh_receiver = Some(receiver);

    let cache_manager_clone = cache_manager;
    let env_config_clone = env_config.clone();
    let auth_config_clone = auth_config.clone();

    tokio::spawn(async move {
        if let Err(e) = cache_manager_clone.refresh_cache(&env_config_clone, &auth_config_clone, sender).await {
            debug!("Cache refresh task failed: {}", e);
        }
    });

    loop {
        terminal.draw(|f| {
            if refresh_complete {
                loading_modal.set_message("✅ CSV cache refresh complete!".to_string());
            } else if current_entity_index < initial_statuses.len() {
                let entity_name = &initial_statuses[current_entity_index].logical_type;
                loading_modal.set_message(format!("Fetching {} entity data... ({}/{})",
                    entity_name, current_entity_index + 1, initial_statuses.len()));
                current_entity_index = (current_entity_index + 1) % initial_statuses.len();
            }

            loading_modal.tick();
            loading_modal.render(f, f.area());
        })?;

        // Check for refresh completion
        if let Some(receiver) = &mut refresh_receiver {
            match receiver.try_recv() {
                Ok(final_statuses) => {
                    refresh_complete = true;
                    refresh_receiver = None;
                    debug!("Cache refresh completed");

                    // Auto-continue after a brief delay to show completion
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    return Ok(true);
                }
                Err(oneshot::error::TryRecvError::Empty) => {
                    // Still in progress
                }
                Err(oneshot::error::TryRecvError::Closed) => {
                    loading_modal.set_error("Cache refresh task was cancelled".to_string());
                    refresh_complete = true;
                    refresh_receiver = None;
                    debug!("Cache refresh task was cancelled");
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
                        if refresh_complete {
                            return Ok(true);
                        }
                    }
                    _ => {}
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

