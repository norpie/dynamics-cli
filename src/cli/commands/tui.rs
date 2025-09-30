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

use crate::tui::MultiAppRuntime;

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

    // Create multi-app runtime
    let mut runtime = MultiAppRuntime::new();

    // Run the TUI loop
    let result = run_tui(&mut terminal, &mut runtime).await;

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
    runtime: &mut MultiAppRuntime,
) -> Result<()> {
    let mut needs_render = true;

    loop {
        // Determine frame budget based on activity
        let (should_render, poll_timeout) = if runtime.has_pending_async() {
            // Active: render at 60 FPS while async operations are pending
            (true, std::time::Duration::from_millis(16))
        } else if needs_render {
            // One-shot render after event
            (true, std::time::Duration::from_millis(16))
        } else {
            // Idle: don't render, use longer poll to save CPU
            (false, std::time::Duration::from_millis(80))
        };

        if should_render {
            let frame_start = std::time::Instant::now();

            // Render the TUI
            terminal.draw(|frame| {
                runtime.render(frame);
            })?;

            // Poll async commands
            runtime.poll_async().await?;

            needs_render = false;

            // Sleep remainder of frame time to maintain 60 FPS
            let elapsed = frame_start.elapsed();
            if let Some(remaining) = std::time::Duration::from_millis(16).checked_sub(elapsed) {
                std::thread::sleep(remaining);
            }
        }

        // Poll for events
        if event::poll(poll_timeout)? {
            let event_result = event::read()?;

            // Handle global shortcuts first
            if let Event::Key(key) = &event_result {
                if key.code == crossterm::event::KeyCode::Char('q')
                    && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    break;
                }

                // Pass key event to runtime
                if !runtime.handle_key(*key)? {
                    break;
                }
            }

            // Handle mouse events
            if let Event::Mouse(mouse) = &event_result {
                if !runtime.handle_mouse(*mouse)? {
                    break;
                }
            }

            // Mark that we need to render after handling event
            needs_render = true;
        }
    }

    Ok(())
}