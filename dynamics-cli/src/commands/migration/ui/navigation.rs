use crate::commands::migration::ui::{
    components::FooterComponent,
    screens::{Screen, ScreenResult},
};
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub struct NavigationManager {
    current_screen: Box<dyn Screen>,
    footer: FooterComponent,
}

impl NavigationManager {
    pub fn new(initial_screen: Box<dyn Screen>) -> Self {
        let mut nav_manager = Self {
            current_screen: initial_screen,
            footer: FooterComponent::new(),
        };
        nav_manager.update_ui_components();
        nav_manager
    }

    pub fn navigate_to(&mut self, mut screen: Box<dyn Screen>) {
        self.current_screen.on_exit();
        screen.on_enter();
        self.current_screen = screen;
        self.update_ui_components();
    }

    pub fn handle_event(&mut self, event: Event) -> NavigationResult {
        match self.current_screen.handle_event(event) {
            ScreenResult::Continue => NavigationResult::Continue,
            ScreenResult::Back => NavigationResult::Exit, // Back from root goes to exit
            ScreenResult::Exit => NavigationResult::Exit,
            ScreenResult::Navigate(screen) => {
                self.navigate_to(screen);
                NavigationResult::Continue
            }
            ScreenResult::Replace(screen) => {
                self.navigate_to(screen);
                NavigationResult::Continue
            }
        }
    }

    fn handle_screen_result(&mut self, result: ScreenResult) -> NavigationResult {
        match result {
            ScreenResult::Continue => NavigationResult::Continue,
            ScreenResult::Back => NavigationResult::Exit, // Back from root goes to exit
            ScreenResult::Exit => NavigationResult::Exit,
            ScreenResult::Navigate(screen) => {
                self.navigate_to(screen);
                NavigationResult::Continue
            }
            ScreenResult::Replace(screen) => {
                self.navigate_to(screen);
                NavigationResult::Continue
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) -> NavigationResult {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer (boxed)
            ])
            .split(area);

        // Render current screen
        self.current_screen.render(f, chunks[0]);

        // Check for immediate navigation after render
        if let Some(navigation_result) = self.current_screen.check_navigation() {
            return self.handle_screen_result(navigation_result);
        }

        // Render footer with margin to match content
        let footer_area = Rect {
            x: chunks[1].x + 1,
            y: chunks[1].y,
            width: chunks[1].width.saturating_sub(2),
            height: chunks[1].height,
        };
        self.footer.render(f, footer_area);

        NavigationResult::Continue
    }

    pub fn get_current_screen(&self) -> &dyn Screen {
        self.current_screen.as_ref()
    }

    pub fn get_current_screen_mut(&mut self) -> &mut dyn Screen {
        self.current_screen.as_mut()
    }

    fn update_ui_components(&mut self) {
        // Update footer
        let footer_actions = self.current_screen.get_footer_actions();
        let mut footer = FooterComponent::new();
        for action in footer_actions {
            footer = footer.add_action_enabled(&action.key, &action.description, action.enabled);
        }
        self.footer = footer;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationResult {
    Continue,
    Exit,
}
