use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;
use std::time::Duration;

use crate::{
    commands::migration::ui::{
        navigation::{NavigationManager, NavigationResult},
        screens::MigrationSelectScreen,
    },
    config::Config,
};

pub struct MigrationUI {
    navigation: NavigationManager,
    config: Config,
}

impl MigrationUI {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let initial_screen = Box::new(MigrationSelectScreen::new(config.clone()));
        let navigation = NavigationManager::new(initial_screen);

        Ok(Self { navigation, config })
    }

    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Main event loop
        let result = self.event_loop(&mut terminal).await;

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

    async fn event_loop(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<()> {
        loop {
            // Render current screen
            let mut render_result = NavigationResult::Continue;
            terminal.draw(|f| {
                let area = f.size();
                render_result = self.navigation.render(f, area);
            })?;

            // Handle navigation result from render
            match render_result {
                NavigationResult::Exit => break,
                NavigationResult::Continue => {
                    // Continue with event handling
                }
            }

            // Handle events
            if crossterm::event::poll(Duration::from_millis(100))? {
                let event = crossterm::event::read()?;

                match self.navigation.handle_event(event) {
                    NavigationResult::Exit => break,
                    NavigationResult::Continue => continue,
                }
            }
        }

        Ok(())
    }
}

/// New entry point for the component-based migration UI
pub async fn start_new_ui() -> Result<()> {
    let mut app = MigrationUI::new()?;
    app.run().await
}
