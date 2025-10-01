use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use anyhow::Result;
use std::collections::HashMap;

use crate::tui::{AppId, Runtime, AppRuntime, apps::{AppLauncher, Example1, Example2, Example3, Example4, Example5, Example6, LoadingScreen, ErrorScreen}, Element, LayoutConstraint, Layer, Theme, ThemeVariant, App};
use crate::tui::element::{ColumnBuilder, RowBuilder, FocusId};
use crate::tui::widgets::ScrollableState;

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    /// All registered app runtimes, stored as trait objects for type erasure
    runtimes: HashMap<AppId, Box<dyn AppRuntime>>,

    /// Currently active app
    active_app: AppId,

    // Global UI state
    help_menu_open: bool,
    help_scroll_state: ScrollableState,
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
        runtimes.insert(AppId::LoadingScreen, Box::new(Runtime::<LoadingScreen>::new()));
        runtimes.insert(AppId::ErrorScreen, Box::new(Runtime::<ErrorScreen>::new()));

        Self {
            runtimes,
            active_app: AppId::AppLauncher,
            help_menu_open: false,
            help_scroll_state: ScrollableState::new(),
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        // Global keys: F1 toggles help menu
        if key_event.code == KeyCode::F(1) {
            self.help_menu_open = !self.help_menu_open;
            if self.help_menu_open {
                self.help_scroll_state.scroll_to_top(); // Reset scroll when opening
            }
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
        self.check_navigation()?;
        Ok(result)
    }

    pub fn handle_mouse(&mut self, mouse_event: MouseEvent) -> Result<bool> {
        let result = self.runtimes
            .get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .handle_mouse(mouse_event)?;

        self.broadcast_events()?;
        self.check_navigation()?;
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

        for (key, description) in &global_bindings {
            let key_str = format!("{:?}", key);
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

        for (key, description) in &current_app_data.2 {
            let key_str = format!("{:?}", key);
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

            for (key, description) in app_bindings {
                let key_str = format!("{:?}", key);
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
    fn check_navigation(&mut self) -> Result<()> {
        // Check if navigation was requested from active app
        let nav_target = self.runtimes.get_mut(&self.active_app)
            .expect("Active app not found in runtimes")
            .take_navigation();

        if let Some(target) = nav_target {
            // Clear any stale navigation from the target app before switching to it
            // This prevents old navigation requests from when it was previously active
            self.runtimes.get_mut(&target)
                .expect("Target app not found in runtimes")
                .take_navigation();

            self.active_app = target;
        }

        Ok(())
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
        self.broadcast_events()?;
        self.check_navigation()?;
        Ok(())
    }
}