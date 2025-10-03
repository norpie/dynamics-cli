use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use anyhow::Result;
use std::collections::HashMap;

use crate::tui::{AppId, Runtime, AppRuntime, apps::{AppLauncher, LoadingScreen, ErrorScreen, migration::{MigrationEnvironmentApp, MigrationComparisonSelectApp}}, Element, LayoutConstraint, Layer, Theme, ThemeVariant, App, ModalState};
use crate::tui::element::{ColumnBuilder, RowBuilder, FocusId};
use crate::tui::widgets::ScrollableState;
use crate::tui::modals::{HelpModal, ConfirmationModal};

/// Messages for quit confirmation modal
#[derive(Clone)]
enum QuitConfirmMsg {
    Confirm,
    Cancel,
}

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    /// All registered app runtimes, stored as trait objects for type erasure
    runtimes: HashMap<AppId, Box<dyn AppRuntime>>,

    /// Currently active app
    active_app: AppId,

    // Global UI state
    help_modal: ModalState<()>,
    help_scroll_state: ScrollableState,
    quit_modal: ModalState<()>,
    quit_registry: crate::tui::InteractionRegistry<QuitConfirmMsg>,
}

impl MultiAppRuntime {
    pub fn new() -> Self {
        let mut runtimes: HashMap<AppId, Box<dyn AppRuntime>> = HashMap::new();

        // Register all apps here - this is the ONLY place you need to add new apps!
        runtimes.insert(AppId::AppLauncher, Box::new(Runtime::<AppLauncher>::new()));
        runtimes.insert(AppId::LoadingScreen, Box::new(Runtime::<LoadingScreen>::new()));
        runtimes.insert(AppId::ErrorScreen, Box::new(Runtime::<ErrorScreen>::new()));
        runtimes.insert(AppId::MigrationEnvironment, Box::new(Runtime::<MigrationEnvironmentApp>::new()));
        runtimes.insert(AppId::MigrationComparisonSelect, Box::new(Runtime::<MigrationComparisonSelectApp>::new()));

        Self {
            runtimes,
            active_app: AppId::AppLauncher,
            help_modal: ModalState::Closed,
            help_scroll_state: ScrollableState::new(),
            quit_modal: ModalState::Closed,
            quit_registry: crate::tui::InteractionRegistry::new(),
        }
    }

    pub fn request_quit(&mut self) {
        self.quit_modal.open_empty();
        self.quit_registry = crate::tui::InteractionRegistry::new(); // Reset registry
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        // Handle quit confirmation modal (highest priority)
        if self.quit_modal.is_open() {
            match key_event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    return Ok(false);  // Quit
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.quit_modal.close();
                    return Ok(true);  // Don't quit, close modal
                }
                _ => return Ok(true),  // Consume all other keys
            }
        }

        // Global keys: F1 toggles help menu
        if key_event.code == KeyCode::F(1) {
            if self.help_modal.is_open() {
                self.help_modal.close();
            } else {
                self.help_modal.open_empty();
                self.help_scroll_state.scroll_to_top(); // Reset scroll when opening
            }
            return Ok(true);
        }

        // Global keys: Ctrl+Space navigates to app launcher
        if key_event.code == KeyCode::Char(' ') && key_event.modifiers.contains(KeyModifiers::CONTROL) {
            // Clear any stale navigation from app launcher before switching to it
            self.runtimes.get_mut(&AppId::AppLauncher)
                .expect("AppLauncher not found in runtimes")
                .take_navigation();
            self.active_app = AppId::AppLauncher;
            return Ok(true);
        }

        // When help menu is open, intercept keys for help control
        if self.help_modal.is_open() {
            match key_event.code {
                KeyCode::Esc => {
                    self.help_modal.close();
                    return Ok(true);
                }
                KeyCode::Up => {
                    self.help_scroll_state.scroll_up(1);
                    return Ok(true);
                }
                KeyCode::Down => {
                    self.help_scroll_state.scroll_down(1);
                    return Ok(true);
                }
                KeyCode::PageUp => {
                    self.help_scroll_state.page_up();
                    return Ok(true);
                }
                KeyCode::PageDown => {
                    self.help_scroll_state.page_down();
                    return Ok(true);
                }
                KeyCode::Home => {
                    self.help_scroll_state.scroll_to_top();
                    return Ok(true);
                }
                KeyCode::End => {
                    self.help_scroll_state.scroll_to_bottom();
                    return Ok(true);
                }
                _ => {
                    // Consume all other keys when help is open
                    return Ok(true);
                }
            }
        }

        // Global Tab/Shift-Tab navigation (before app-specific handling)
        if key_event.code == KeyCode::Tab {
            let runtime = self.runtimes
                .get_mut(&self.active_app)
                .expect("Active app not found in runtimes");

            if key_event.modifiers.contains(KeyModifiers::SHIFT) {
                runtime.focus_previous()?;
            } else {
                runtime.focus_next()?;
            }

            return Ok(true);
        }

        // Normal: delegate to active app
        let result = self.runtimes
            .get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .handle_key(key_event)?;

        self.broadcast_events()?;
        let _ = self.check_navigation()?;
        Ok(result)
    }

    pub fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        use crossterm::event::MouseEventKind;

        // When quit confirmation is open, check for button clicks
        if self.quit_modal.is_open() {
            if let MouseEventKind::Down(_) = mouse_event.kind {
                // Check if any button was clicked
                if let Some(msg) = self.quit_registry.find_click(mouse_event.column, mouse_event.row) {
                    match msg {
                        QuitConfirmMsg::Confirm => {
                            return Ok(false); // Quit
                        }
                        QuitConfirmMsg::Cancel => {
                            self.quit_modal.close();
                            return Ok(true);
                        }
                    }
                }
            }
            return Ok(true); // Consume all mouse events when modal is open
        }

        // When help menu is open, intercept scroll wheel events
        if self.help_modal.is_open() {
            match mouse_event.kind {
                MouseEventKind::ScrollUp => {
                    self.help_scroll_state.scroll_up(3);  // 3 lines per scroll
                    return Ok(true);
                }
                MouseEventKind::ScrollDown => {
                    self.help_scroll_state.scroll_down(3);  // 3 lines per scroll
                    return Ok(true);
                }
                _ => {
                    // Other mouse events (clicks, moves) are ignored when help menu is open
                    return Ok(true);
                }
            }
        }

        let result = self.runtimes
            .get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .handle_mouse(mouse_event)?;

        self.broadcast_events()?;
        let _ = self.check_navigation()?;
        Ok(result)
    }

    pub fn render(&mut self, frame: &mut Frame) {
        let theme = Theme::new(ThemeVariant::default());
        let full_area = frame.size();

        // Calculate header area (3 lines) and app content area
        let header_height = 3;
        let header_area = ratatui::layout::Rect {
            x: full_area.x,
            y: full_area.y,
            width: full_area.width,
            height: header_height,
        };
        let app_area = ratatui::layout::Rect {
            x: full_area.x,
            y: full_area.y + header_height,
            width: full_area.width,
            height: full_area.height.saturating_sub(header_height),
        };

        // Render global header
        let active_runtime = self.runtimes.get(&self.active_app)
            .expect("Active app not found in runtimes");
        let app_title = active_runtime.get_title();
        let app_status = active_runtime.get_status();
        self.render_header(frame, header_area, app_title, app_status, &theme);

        // Render active app content
        self.runtimes.get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .render_to_area(frame, app_area);

        // If help menu is open, overlay it on top
        if self.help_modal.is_open() {
            self.render_help_menu(frame, full_area, &theme);
        }

        // If quit confirmation is open, overlay it on top (highest priority)
        if self.quit_modal.is_open() {
            self.render_quit_confirm(frame, full_area, &theme);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: ratatui::layout::Rect, title: &str, status: Option<Line<'static>>, theme: &Theme) {
        // Build title line with optional status
        let title_line = if let Some(status_line) = status {
            // Combine title and status
            let mut spans = vec![
                Span::styled(String::from(title), Style::default().fg(theme.blue).bold()),
                Span::raw(" "),
            ];
            spans.extend(status_line.spans);
            Line::from(spans)
        } else {
            // Just title
            Line::from(Span::styled(String::from(title), Style::default().fg(theme.blue).bold()))
        };

        let header_left = Element::styled_text(title_line).build();
        let header_right = Element::styled_text(Line::from(vec![
            Span::styled("[?] F1 Help", Style::default().fg(theme.overlay1))
        ])).build();

        let header = Element::panel(
            RowBuilder::new()
                .add(header_left, LayoutConstraint::Fill(1))
                .add(header_right, LayoutConstraint::Length(15))
                .spacing(0)
                .build()
        )
        .build();

        use crate::tui::{Renderer, InteractionRegistry};
        use crate::tui::renderer::{FocusRegistry, DropdownRegistry};
        let mut registry: InteractionRegistry<()> = InteractionRegistry::new();
        let mut focus_registry: FocusRegistry<()> = FocusRegistry::new();
        let mut dropdown_registry: DropdownRegistry<()> = DropdownRegistry::new();
        Renderer::render(frame, theme, &mut registry, &mut focus_registry, &mut dropdown_registry, None, &header, area);
    }

    fn render_help_menu(&mut self, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
        // First, render a dim overlay over the entire area
        use ratatui::widgets::Block;
        use ratatui::style::Style;
        let dim_block = Block::default()
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_block, area);

        // Build global key bindings
        let global_bindings = vec![
            (KeyCode::F(1), "Toggle help menu".to_string()),
            (KeyCode::Char(' '), "Go to app launcher (hold Ctrl)".to_string()),
            (KeyCode::Esc, "Close help menu".to_string()),
        ];

        // Get all apps' key bindings
        let mut all_app_bindings: Vec<(AppId, &'static str, Vec<(KeyCode, String)>)> = vec![];
        for (app_id, runtime) in &self.runtimes {
            let title = runtime.get_title();
            let bindings = runtime.get_key_bindings();
            all_app_bindings.push((*app_id, title, bindings));
        }

        // Separate current app from others
        let current_app_data = all_app_bindings.iter()
            .find(|(id, _, _)| *id == self.active_app)
            .expect("Active app not found");

        let other_apps: Vec<_> = all_app_bindings.iter()
            .filter(|(id, _, _)| *id != self.active_app)
            .collect();

        // Build help modal using HelpModal builder
        let mut modal_builder = HelpModal::new()
            .global_bindings(global_bindings.clone())
            .current_app(current_app_data.1, current_app_data.2.clone())
            .scroll_state(&self.help_scroll_state);

        // Add other apps
        for (_, app_title, app_bindings) in &other_apps {
            modal_builder = modal_builder.add_app(*app_title, app_bindings.clone());
        }

        let help_modal = modal_builder.build(theme);

        // Calculate modal dimensions and position
        let modal_width = area.width.min(60);
        let modal_height = area.height.min(20);
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Calculate available content height and update scroll state dimensions
        // This must happen BEFORE rendering since ScrollableState needs it for scroll calculations
        let content_height = modal_height.saturating_sub(4) as usize; // 2 for borders, 2 for padding

        // Count total help items for scroll state
        // 2 (title + blank) + global section + current app section + other apps sections
        let mut total_items = 2; // title + blank
        if !global_bindings.is_empty() {
            total_items += 2 + global_bindings.len(); // section header + items + blank
        }
        total_items += 2 + current_app_data.2.len(); // section header + items + blank
        for (_, _, app_bindings) in &other_apps {
            total_items += 1 + app_bindings.len() + 1; // section header + items + blank
        }
        total_items += 2; // blank + footer

        self.help_scroll_state.update_dimensions(total_items, content_height);

        // Render the modal
        use crate::tui::{Renderer, InteractionRegistry};
        use crate::tui::renderer::{FocusRegistry, DropdownRegistry};
        let mut registry: InteractionRegistry<()> = InteractionRegistry::new();
        let mut focus_registry: FocusRegistry<()> = FocusRegistry::new();
        let mut dropdown_registry: DropdownRegistry<()> = DropdownRegistry::new();
        Renderer::render(frame, theme, &mut registry, &mut focus_registry, &mut dropdown_registry, None, &help_modal, modal_area);
    }

    /// Render quit confirmation modal
    fn render_quit_confirm(&mut self, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
        use ratatui::widgets::Block;
        use ratatui::style::Style;

        // Render dim overlay
        let dim_block = Block::default()
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_block, area);

        // Build quit confirmation modal using builder
        let quit_modal = ConfirmationModal::new("Quit Application")
            .message("Are you sure you want to quit?")
            .confirm_text("Yes")
            .cancel_text("No")
            .confirm_hotkey("Y")
            .cancel_hotkey("N/Esc")
            .on_confirm(QuitConfirmMsg::Confirm)
            .on_cancel(QuitConfirmMsg::Cancel)
            .width(50)
            .height(10)
            .build(theme);

        // Calculate modal position
        let modal_width = 50;
        let modal_height = 10;
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Render modal and capture interactions in quit_registry
        let mut focus_registry = crate::tui::renderer::FocusRegistry::new();
        let mut dropdown_registry = crate::tui::renderer::DropdownRegistry::new();
        crate::tui::Renderer::render(
            frame,
            theme,
            &mut self.quit_registry,
            &mut focus_registry,
            &mut dropdown_registry,
            None,
            &quit_modal,
            modal_area,
        );
    }

    pub async fn poll_async(&mut self) -> Result<()> {
        // Poll all apps regardless of which is active
        for runtime in self.runtimes.values_mut() {
            runtime.poll_async().await?;
        }
        Ok(())
    }

    /// Poll timer subscriptions for all apps
    pub fn poll_timers(&mut self) -> Result<()> {
        // Poll all apps regardless of which is active
        for runtime in self.runtimes.values_mut() {
            runtime.poll_timers()?;
        }
        Ok(())
    }

    /// Check if any navigation commands were issued
    fn check_navigation(&mut self) -> Result<bool> {
        // Check if navigation was requested from active app
        let nav_target = self.runtimes.get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .take_navigation();

        if let Some(target) = nav_target {
            // Don't clear target's navigation - it may have been set by event handlers
            // during broadcast_events() (e.g., PerformParallel setting LoadingScreen)
            self.active_app = target;
            Ok(true) // Navigation happened
        } else {
            Ok(false) // No navigation
        }
    }

    /// Broadcast events globally to all apps
    fn broadcast_events(&mut self) -> Result<()> {
        // Collect all pending publishes from all apps
        let mut all_events = Vec::new();
        for runtime in self.runtimes.values_mut() {
            all_events.extend(runtime.take_publishes());
        }

        // Broadcast each event to all apps
        for (topic, data) in all_events {
            log::debug!("Broadcasting event '{}' to all apps", topic);
            for runtime in self.runtimes.values_mut() {
                runtime.handle_publish(&topic, data.clone())?;
            }
        }

        Ok(())
    }

    /// Process side effects (navigation, events) from timers and async commands
    pub fn process_side_effects(&mut self) -> Result<()> {
        // IMPORTANT: Broadcast events FIRST, then check navigation ONCE
        // This ensures:
        // 1. Parallel task completion events are broadcast to LoadingScreen before we navigate away
        // 2. Initial navigation still works because we loop until things settle
        //
        // We loop until no more navigation happens:
        // - Iteration 1: Broadcast "migration:selected", then navigate to ComparisonSelect
        //   - ComparisonSelect.Initialize creates PerformParallel which sets navigation to LoadingScreen
        // - Iteration 2: Broadcast "loading:init", then navigate to LoadingScreen
        //   - LoadingScreen.Initialize sets up loading state
        // - Iteration 3: Broadcast parallel completion events, then navigate to target
        //   - LoadingScreen received all events before navigation
        const MAX_LOOPS: usize = 5;
        for _ in 0..MAX_LOOPS {
            // Broadcast events first so LoadingScreen gets completion events before we navigate
            self.broadcast_events()?;

            // Then check if we need to navigate
            let navigated = self.check_navigation()?;

            // If no navigation happened, we're done
            if !navigated {
                break;
            }
        }
        Ok(())
    }
}