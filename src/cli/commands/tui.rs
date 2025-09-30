use anyhow::Result;
use clap::{Args, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::io;

use crate::tui::TuiOrchestrator;

#[derive(Args)]
pub struct TuiCommands {
    #[command(subcommand)]
    pub command: Option<TuiSubcommands>,
}

#[derive(Subcommand)]
pub enum TuiSubcommands {
    /// Launch the interactive TUI (default)
    Launch,
}

pub async fn tui_command(args: TuiCommands) -> Result<()> {
    match args.command {
        Some(TuiSubcommands::Launch) | None => {
            launch_tui().await?;
        }
    }
    Ok(())
}

async fn launch_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create TUI orchestrator
    let mut tui_orchestrator = TuiOrchestrator::new().await?;

    // Run the TUI loop
    let result = run_tui(&mut terminal, &mut tui_orchestrator).await;

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

async fn run_tui<B: Backend>(
    terminal: &mut Terminal<B>,
    tui_orchestrator: &mut TuiOrchestrator,
) -> Result<()> {
    loop {
        // Render the TUI
        terminal.draw(|frame| {
            tui_orchestrator.render(frame);
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            let event = event::read()?;

            // Handle global shortcuts first
            if let Event::Key(key) = &event {
                if key.code == crossterm::event::KeyCode::Char('q')
                    && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    break;
                }
            }

            // Pass event to orchestrator (now async)
            if !tui_orchestrator.handle_event(event).await? {
                break;
            }
        }
    }

    Ok(())
}