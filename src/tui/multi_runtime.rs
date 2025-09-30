use ratatui::Frame;
use crossterm::event::{KeyEvent, MouseEvent};
use anyhow::Result;

use crate::tui::{AppId, Runtime, apps::{Example1, Example2}};

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    example1: Runtime<Example1>,
    example2: Runtime<Example2>,
    active_app: AppId,
}

impl MultiAppRuntime {
    pub fn new() -> Self {
        Self {
            example1: Runtime::new(),
            example2: Runtime::new(),
            active_app: AppId::Example1,
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        match self.active_app {
            AppId::Example1 => {
                let result = self.example1.handle_key(key_event)?;
                self.check_navigation()?;
                Ok(result)
            }
            AppId::Example2 => {
                let result = self.example2.handle_key(key_event)?;
                self.check_navigation()?;
                Ok(result)
            }
        }
    }

    pub fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        match self.active_app {
            AppId::Example1 => {
                let result = self.example1.handle_mouse(mouse_event)?;
                self.check_navigation()?;
                Ok(result)
            }
            AppId::Example2 => {
                let result = self.example2.handle_mouse(mouse_event)?;
                self.check_navigation()?;
                Ok(result)
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        match self.active_app {
            AppId::Example1 => self.example1.render(frame),
            AppId::Example2 => self.example2.render(frame),
        }
    }

    /// Poll async commands for all apps
    pub async fn poll_async(&mut self) -> Result<()> {
        // Poll both apps regardless of which is active
        self.example1.poll_async().await?;
        self.example2.poll_async().await?;
        Ok(())
    }

    /// Check if any navigation commands were issued
    fn check_navigation(&mut self) -> Result<()> {
        // Check if navigation was requested
        let nav_target = match self.active_app {
            AppId::Example1 => self.example1.take_navigation(),
            AppId::Example2 => self.example2.take_navigation(),
        };

        if let Some(target) = nav_target {
            self.active_app = target;
        }

        Ok(())
    }
}