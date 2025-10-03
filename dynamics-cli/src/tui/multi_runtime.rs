use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use anyhow::Result;
use std::collections::HashMap;

use crate::tui::{AppId, Runtime, AppRuntime, apps::{AppLauncher, examples::{Example1, Example2, Example3, Example4, Example5, Example6, Example7, Example8, ExampleResourceMacro}, LoadingScreen, ErrorScreen, migration::{MigrationEnvironmentApp, MigrationComparisonSelectApp}}, Element, LayoutConstraint, Layer, Theme, ThemeVariant, App};
use crate::tui::element::{ColumnBuilder, RowBuilder, FocusId};
use crate::tui::widgets::ScrollableState;

/// Format a KeyCode for display (e.g., Char('i') → "i", F(1) → "F1")
fn format_key(key: &KeyCode) -> String {
    match key {
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Delete => "Del".to_string(),
        KeyCode::Insert => "Ins".to_string(),
        _ => format!("{:?}", key),
    }
}

/// Group keybindings by description and format as aliases (e.g., "n/N")
fn group_and_format_bindings(bindings: &[(KeyCode, String)]) -> Vec<(String, String)> {
    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();

    for (key, desc) in bindings {
        grouped.entry(desc.clone())
            .or_default()
            .push(format_key(key));
    }

    let mut result: Vec<(String, String)> = grouped.into_iter()
        .map(|(desc, mut keys)| {
            keys.sort();  // Consistent ordering
            let key_str = keys.join("/");
            (key_str, desc)
        })
        .collect();

    // Sort by key string for consistent display
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    /// All registered app runtimes, stored as trait objects for type erasure
    runtimes: HashMap<AppId, Box<dyn AppRuntime>>,

    /// Currently active app
    active_app: AppId,

    // Global UI state
    help_menu_open: bool,
    help_scroll_state: ScrollableState,
    quit_confirm_open: bool,
}

impl MultiAppRuntime {
    pub fn new() -> Self {
        let mut runtimes: HashMap<AppId, Box<dyn AppRuntime>> = HashMap::new();

        // Register all apps here - this is the ONLY place you need to add new apps!
        runtimes.insert(AppId::AppLauncher, Box::new(Runtime::<AppLauncher>::new()));
        runtimes.insert(AppId::Example1, Box::new(Runtime::<Example1>::new()));
        runtimes.insert(AppId::Example2, Box::new(Runtime::<Example2>::new()));
        runtimes.insert(AppId::Example3, Box::new(Runtime::<Example3>::new()));
        runtimes.insert(AppId::Example4, Box::new(Runtime::<Example4>::new()));
        runtimes.insert(AppId::Example5, Box::new(Runtime::<Example5>::new()));
        runtimes.insert(AppId::Example6, Box::new(Runtime::<Example6>::new()));
        runtimes.insert(AppId::Example7, Box::new(Runtime::<Example7>::new()));
        runtimes.insert(AppId::Example8, Box::new(Runtime::<Example8>::new()));
        runtimes.insert(AppId::ExampleResourceMacro, Box::new(Runtime::<ExampleResourceMacro>::new()));
        runtimes.insert(AppId::LoadingScreen, Box::new(Runtime::<LoadingScreen>::new()));
        runtimes.insert(AppId::ErrorScreen, Box::new(Runtime::<ErrorScreen>::new()));
        runtimes.insert(AppId::MigrationEnvironment, Box::new(Runtime::<MigrationEnvironmentApp>::new()));
        runtimes.insert(AppId::MigrationComparisonSelect, Box::new(Runtime::<MigrationComparisonSelectApp>::new()));

        Self {
            runtimes,
            active_app: AppId::AppLauncher,
            help_menu_open: false,
            help_scroll_state: ScrollableState::new(),
            quit_confirm_open: false,
        }
    }

    pub fn request_quit(&mut self) {
        self.quit_confirm_open = true;
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        // Handle quit confirmation modal (highest priority)
        if self.quit_confirm_open {
            match key_event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    return Ok(false);  // Quit
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.quit_confirm_open = false;
                    return Ok(true);  // Don't quit
                }
                _ => return Ok(true),  // Consume other keys
            }
        }
        // Global keys: F1 toggles help menu
        if key_event.code == KeyCode::F(1) {
            self.help_menu_open = !self.help_menu_open;
            if self.help_menu_open {
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
        if self.help_menu_open {
            match key_event.code {
                KeyCode::Esc => {
                    self.help_menu_open = false;
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

        // When quit confirmation is open, consume all mouse events
        if self.quit_confirm_open {
            return Ok(true);
        }

        // When help menu is open, intercept scroll wheel events
        if self.help_menu_open {
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
        if self.help_menu_open {
            self.render_help_menu(frame, full_area, &theme);
        }

        // If quit confirmation is open, overlay it on top (highest priority)
        if self.quit_confirm_open {
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

        // Build ALL help content items (no skipping - List widget handles scrolling)
        let mut help_items = vec![
            Element::styled_text(Line::from(vec![
                Span::styled("Keyboard Shortcuts", Style::default().fg(theme.lavender).bold())
            ])).build(),
            Element::text(""),
        ];

        // Section 1: Global Keys (highest priority)
        help_items.push(Element::styled_text(Line::from(vec![
            Span::styled("▼ Global", Style::default().fg(theme.peach).bold())
        ])).build());

        let formatted_global = group_and_format_bindings(&global_bindings);
        for (key_str, description) in &formatted_global {
            let line = Line::from(vec![
                Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.mauve)),
                Span::raw("  "),
                Span::styled(description.clone(), Style::default().fg(theme.text)),
            ]);
            help_items.push(Element::styled_text(line).build());
        }

        help_items.push(Element::text(""));

        // Section 2: Current App Keys
        help_items.push(Element::styled_text(Line::from(vec![
            Span::styled(format!("▼ {}", current_app_data.1), Style::default().fg(theme.blue).bold())
        ])).build());

        let formatted_current = group_and_format_bindings(&current_app_data.2);
        for (key_str, description) in &formatted_current {
            let line = Line::from(vec![
                Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.green)),
                Span::raw("  "),
                Span::styled(description.clone(), Style::default().fg(theme.text)),
            ]);
            help_items.push(Element::styled_text(line).build());
        }

        help_items.push(Element::text(""));

        // Section 3: Other Apps
        for (_, app_title, app_bindings) in other_apps {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled(format!("▼ {}", app_title), Style::default().fg(theme.overlay1).bold())
            ])).build());

            let formatted_other = group_and_format_bindings(app_bindings);
            for (key_str, description) in &formatted_other {
                let line = Line::from(vec![
                    Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.overlay2)),
                    Span::raw("  "),
                    Span::styled(description.clone(), Style::default().fg(theme.subtext0)),
                ]);
                help_items.push(Element::styled_text(line).build());
            }

            help_items.push(Element::text(""));
        }

        help_items.push(Element::text(""));
        help_items.push(Element::styled_text(Line::from(vec![
            Span::styled("[ESC to close | ↑↓/PgUp/PgDn/Home/End to scroll]", Style::default().fg(theme.overlay1))
        ])).build());

        // Calculate modal dimensions and position
        let modal_width = area.width.min(60);
        let modal_height = area.height.min(20);
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Calculate available content height (subtract panel borders + container padding)
        let content_height = modal_height.saturating_sub(4) as usize; // 2 for borders, 2 for padding

        // Update scroll state dimensions
        self.help_scroll_state.update_dimensions(help_items.len(), content_height);

        // Create scrollable column with all items (spacing=0 for dense packing)
        let mut column_builder = ColumnBuilder::new();
        for item in help_items {
            column_builder = column_builder.add(item, LayoutConstraint::Length(1));
        }
        let help_column = column_builder.spacing(0).build();

        let help_content = Element::scrollable(
            FocusId::new("help_scroll"),
            help_column,
            &self.help_scroll_state,
        ).build();

        // Wrap in panel and center
        let help_modal = Element::panel(
            Element::container(help_content)
                .padding(1)
                .build()
        )
        .title("Help")
        .build();

        use crate::tui::{Renderer, InteractionRegistry};
        use crate::tui::renderer::{FocusRegistry, DropdownRegistry};
        let mut registry: InteractionRegistry<()> = InteractionRegistry::new();
        let mut focus_registry: FocusRegistry<()> = FocusRegistry::new();
        let mut dropdown_registry: DropdownRegistry<()> = DropdownRegistry::new();
        Renderer::render(frame, theme, &mut registry, &mut focus_registry, &mut dropdown_registry, None, &help_modal, modal_area);
    }

    /// Poll async commands for all apps
    fn render_quit_confirm(&self, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
        use ratatui::widgets::Block;
        use ratatui::style::Style;

        // Render dim overlay
        let dim_block = Block::default()
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_block, area);

        // Create simple confirmation modal
        let modal_width = 50;
        let modal_height = 8;
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        let message_content = ColumnBuilder::<()>::new()
            .add(
                Element::styled_text(Line::from(vec![
                    Span::styled("Quit Application", Style::default().fg(theme.red).bold())
                ])).build(),
                LayoutConstraint::Length(1),
            )
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(
                Element::text("Are you sure you want to quit?"),
                LayoutConstraint::Length(1),
            )
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(
                Element::styled_text(Line::from(vec![
                    Span::styled("[Y/Enter]", Style::default().fg(theme.green)),
                    Span::raw(" Yes  "),
                    Span::styled("[N/Esc]", Style::default().fg(theme.red)),
                    Span::raw(" No"),
                ])).build(),
                LayoutConstraint::Length(1),
            )
            .spacing(0)
            .build();

        let quit_modal = Element::panel(
            Element::container(message_content)
                .padding(1)
                .build()
        )
        .build();

        // Render using element system
        let mut registry = crate::tui::renderer::InteractionRegistry::new();
        let mut focus_registry = crate::tui::renderer::FocusRegistry::new();
        let mut dropdown_registry = crate::tui::renderer::DropdownRegistry::new();
        crate::tui::Renderer::render(
            frame,
            theme,
            &mut registry,
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
            for runtime in self.runtimes.values_mut() {
                runtime.handle_publish(&topic, data.clone())?;
            }
        }

        Ok(())
    }

    /// Process side effects (navigation, events) from timers and async commands
    pub fn process_side_effects(&mut self) -> Result<()> {
        // IMPORTANT: Navigate BEFORE broadcasting events
        // This ensures that when we publish events (e.g. "migration:selected"),
        // the target app (e.g. MigrationComparisonSelect) has already been navigated to
        // and will receive the event immediately, triggering Initialize which may
        // create a PerformParallel command that shows the LoadingScreen.
        //
        // We loop until no more navigation happens because:
        // 1. Initial navigation (e.g. to MigrationComparisonSelect)
        // 2. Broadcast events
        // 3. Event handler sets new navigation (e.g. to LoadingScreen via PerformParallel)
        // 4. Loop again to process that navigation immediately
        //
        // This prevents rendering intermediate frames between navigations.
        // Keep processing navigation + events until everything settles
        // We need multiple iterations because:
        // - Iteration 1: Navigate to ComparisonSelect, broadcast "migration:selected"
        //   - ComparisonSelect.Initialize creates PerformParallel which sets navigation to LoadingScreen
        // - Iteration 2: Navigate to LoadingScreen, broadcast "loading:init"
        //   - LoadingScreen.Initialize sets up loading state
        // - Iteration 3: No more navigation, exits
        const MAX_LOOPS: usize = 5;
        for _ in 0..MAX_LOOPS {
            self.check_navigation()?;
            self.broadcast_events()?;
        }
        Ok(())
    }
}