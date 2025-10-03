use crate::commands::migration::ui::components::FooterAction;
use crossterm::event::Event;
use ratatui::{Frame, layout::Rect};

pub trait Screen {
    fn render(&mut self, f: &mut Frame, area: Rect);
    fn handle_event(&mut self, event: Event) -> ScreenResult;
    fn get_footer_actions(&self) -> Vec<FooterAction>;
    fn get_title(&self) -> Option<String> {
        None
    }
    fn on_enter(&mut self) {}
    fn on_exit(&mut self) {}

    /// Check for immediate navigation that should happen regardless of events
    /// Called after every render cycle to allow async operations to trigger navigation
    fn check_navigation(&mut self) -> Option<ScreenResult> {
        None
    }
}

pub enum ScreenResult {
    Continue,
    Back,
    Exit,
    Navigate(Box<dyn Screen>),
    Replace(Box<dyn Screen>),
}

// Helper trait for creating screens with common patterns
pub trait ScreenBuilder {
    type Screen: Screen;

    fn build(self) -> Self::Screen;
}
