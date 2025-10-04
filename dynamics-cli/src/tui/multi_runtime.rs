use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use anyhow::Result;
use std::collections::HashMap;

use crate::tui::{AppId, Runtime, AppRuntime, apps::{AppLauncher, LoadingScreen, ErrorScreen, SettingsApp, migration::{MigrationEnvironmentApp, MigrationComparisonSelectApp}}, Element, LayoutConstraint, Layer, Theme, ThemeVariant, App, ModalState, KeyBinding, AppLifecycle};
use crate::tui::runtime::AppFactory;
use crate::tui::element::{ColumnBuilder, RowBuilder, FocusId};
use crate::tui::widgets::ScrollableState;

/// Group key bindings by description, combining keys with the same description into aliases
fn group_bindings_by_description(bindings: &[(KeyBinding, &str)]) -> Vec<(String, String)> {
    let mut grouped: HashMap<&str, Vec<String>> = HashMap::new();

    for (key, desc) in bindings {
        grouped.entry(*desc)
            .or_insert_with(Vec::new)
            .push(key.to_string());
    }

    let mut result: Vec<(String, String)> = grouped.into_iter()
        .map(|(desc, mut keys)| {
            // Sort keys to ensure consistent ordering (shorter keys first)
            keys.sort_by_key(|k| k.len());
            (keys.join("/"), desc.to_string())
        })
        .collect();

    // Sort by the first key in each group for consistent ordering
    result.sort_by(|a, b| a.0.cmp(&b.0));

    result
}

/// Messages for global UI elements (help menu, quit modal, etc.)
#[derive(Clone)]
enum GlobalMsg {
    // Quit modal
    QuitConfirm,
    QuitCancel,

    // Help modal
    HelpScroll(KeyCode),
    CloseHelp,
}

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    /// App factories for lazy creation
    factories: HashMap<AppId, Box<dyn AppFactory>>,

    /// Active app instances (lazily created)
    runtimes: HashMap<AppId, Box<dyn AppRuntime>>,

    /// Lifecycle state for each app
    lifecycles: HashMap<AppId, AppLifecycle>,

    /// Currently active app
    active_app: AppId,

    // Global UI state
    help_modal: ModalState<()>,
    help_scroll_state: ScrollableState,
    quit_modal: ModalState<()>,

    // Global focus system
    global_interaction_registry: crate::tui::InteractionRegistry<GlobalMsg>,
    global_focus_registry: crate::tui::renderer::FocusRegistry<GlobalMsg>,
    global_focused_id: Option<FocusId>,
}

impl MultiAppRuntime {
    pub fn new() -> Self {
        let mut factories: HashMap<AppId, Box<dyn AppFactory>> = HashMap::new();
        let mut lifecycles = HashMap::new();

        // Register all app factories here - this is the ONLY place you need to add new apps!
        factories.insert(AppId::AppLauncher, Box::new(std::marker::PhantomData::<AppLauncher>));
        factories.insert(AppId::LoadingScreen, Box::new(std::marker::PhantomData::<LoadingScreen>));
        factories.insert(AppId::ErrorScreen, Box::new(std::marker::PhantomData::<ErrorScreen>));
        factories.insert(AppId::Settings, Box::new(std::marker::PhantomData::<SettingsApp>));
        factories.insert(AppId::MigrationEnvironment, Box::new(std::marker::PhantomData::<MigrationEnvironmentApp>));
        factories.insert(AppId::MigrationComparisonSelect, Box::new(std::marker::PhantomData::<MigrationComparisonSelectApp>));

        // Mark all apps as NotCreated initially
        for app_id in factories.keys() {
            lifecycles.insert(*app_id, AppLifecycle::NotCreated);
        }

        let mut runtime = Self {
            factories,
            runtimes: HashMap::new(),
            lifecycles,
            active_app: AppId::AppLauncher,
            help_modal: ModalState::Closed,
            help_scroll_state: ScrollableState::new(),
            quit_modal: ModalState::Closed,
            global_interaction_registry: crate::tui::InteractionRegistry::new(),
            global_focus_registry: crate::tui::renderer::FocusRegistry::new(),
            global_focused_id: None,
        };

        // Eagerly create the AppLauncher since it's the starting app
        runtime.ensure_app_exists(AppId::AppLauncher, Box::new(())).ok();

        runtime
    }

    /// Ensure an app exists (create it if not already created)
    fn ensure_app_exists(&mut self, app_id: AppId, params: Box<dyn std::any::Any + Send>) -> Result<()> {
        // If app already exists and is running or background, do nothing
        if matches!(self.lifecycles.get(&app_id), Some(AppLifecycle::Running) | Some(AppLifecycle::Background)) {
            return Ok(());
        }

        // Get the factory for this app
        let factory = self.factories.get(&app_id)
            .ok_or_else(|| anyhow::anyhow!("No factory registered for app {:?}", app_id))?;

        // Create the app instance
        let runtime = factory.create(params)?;

        // Store the runtime and mark as running
        self.runtimes.insert(app_id, runtime);
        self.lifecycles.insert(app_id, AppLifecycle::Running);

        log::info!("Created app {:?}", app_id);
        Ok(())
    }

    pub fn request_quit(&mut self) {
        self.quit_modal.open_empty();
        // Auto-focus the cancel button (first button in the quit modal)
        self.global_focused_id = Some(FocusId::new("quit-cancel-btn"));
    }

    /// Move focus within global UI elements (Tab/Shift-Tab)
    fn move_global_focus(&mut self, forward: bool) {
        self.global_focused_id = if forward {
            self.global_focus_registry.next_focus(self.global_focused_id.as_ref())
        } else {
            self.global_focus_registry.prev_focus(self.global_focused_id.as_ref())
        };
    }

    /// Handle keyboard input for global UI elements (when modals are open)
    /// Returns Some(should_continue) if the key was handled, None if it should pass through
    fn handle_global_key(&mut self, key_event: KeyEvent) -> Result<Option<bool>> {
        // Check if there's a focused element in the global focus registry
        if let Some(focused_id) = &self.global_focused_id {
            // Get the key handler for the focused element
            if let Some(msg) = self.global_focus_registry.dispatch_key(focused_id, key_event) {
                return Ok(Some(self.handle_global_msg(msg)?));
            }
        }

        // Key not handled by focused element
        Ok(None)
    }

    /// Handle global messages from global UI elements
    fn handle_global_msg(&mut self, msg: GlobalMsg) -> Result<bool> {
        match msg {
            GlobalMsg::QuitConfirm => {
                return Ok(false); // Quit application
            }
            GlobalMsg::QuitCancel => {
                self.quit_modal.close();
                self.global_focused_id = None; // Clear focus when closing modal
                return Ok(true);
            }
            GlobalMsg::HelpScroll(key) => {
                // Calculate content height (same as in render_help_menu)
                let global_bindings = vec![
                    (KeyBinding::new(KeyCode::F(1)), "Toggle help menu"),
                    (KeyBinding::ctrl(KeyCode::Char(' ')), "Go to app launcher"),
                    (KeyBinding::new(KeyCode::Esc), "Close help menu"),
                ];

                let mut all_app_bindings: Vec<(AppId, &'static str, Vec<(KeyBinding, String)>)> = vec![];
                for (app_id, runtime) in &self.runtimes {
                    let title = runtime.get_title();
                    let bindings = runtime.get_key_bindings();
                    all_app_bindings.push((*app_id, title, bindings));
                }

                let current_app_data = all_app_bindings.iter()
                    .find(|(id, _, _)| *id == self.active_app)
                    .expect("Active app not found");

                let other_apps: Vec<_> = all_app_bindings.iter()
                    .filter(|(id, _, bindings)| *id != self.active_app && !bindings.is_empty())
                    .collect();

                // Calculate total items using grouped bindings
                let global_grouped = group_bindings_by_description(&global_bindings);
                let current_bindings_ref: Vec<(KeyBinding, &str)> = current_app_data.2.iter()
                    .map(|(key, desc)| (*key, desc.as_str()))
                    .collect();
                let current_grouped = group_bindings_by_description(&current_bindings_ref);

                let other_apps_grouped_count: usize = other_apps.iter()
                    .map(|(_, _, bindings)| {
                        let bindings_ref: Vec<(KeyBinding, &str)> = bindings.iter()
                            .map(|(key, desc)| (*key, desc.as_str()))
                            .collect();
                        let grouped = group_bindings_by_description(&bindings_ref);
                        2 + grouped.len() // section header + bindings
                    })
                    .sum();

                let total_items = 2 + // title + blank
                    (if !global_grouped.is_empty() { 2 + global_grouped.len() } else { 0 }) +
                    (if !current_grouped.is_empty() { 2 + current_grouped.len() } else { 0 }) +
                    other_apps_grouped_count +
                    2; // blank + footer

                // Get viewport height from last render (approximation: 20 lines for modal)
                let content_height = 20usize.saturating_sub(4);
                self.help_scroll_state.handle_key(key, total_items, content_height);
                return Ok(true);
            }
            GlobalMsg::CloseHelp => {
                self.help_modal.close();
                self.global_focused_id = None; // Clear focus when closing modal
                return Ok(true);
            }
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        // Priority 1: Global modal keyboard handling (Tab, focused elements)
        if self.quit_modal.is_open() || self.help_modal.is_open() {
            // Tab/Shift-Tab: Move focus within global modal
            if KeyBinding::new(KeyCode::Tab).matches(&key_event) {
                self.move_global_focus(true);
                return Ok(true);
            }
            if KeyBinding::shift(KeyCode::Tab).matches(&key_event) || key_event.code == KeyCode::BackTab {
                self.move_global_focus(false);
                return Ok(true);
            }

            // Dispatch key to focused element
            if let Some(should_continue) = self.handle_global_key(key_event)? {
                return Ok(should_continue);
            }
        }

        // Priority 2: Quit modal hotkeys (Y/N/Esc)
        if self.quit_modal.is_open() {
            match key_event.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    return self.handle_global_msg(GlobalMsg::QuitConfirm);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    return self.handle_global_msg(GlobalMsg::QuitCancel);
                }
                _ => return Ok(true),  // Consume all other keys
            }
        }

        // Priority 3: Help modal Esc to close
        if self.help_modal.is_open() {
            if key_event.code == KeyCode::Esc {
                return self.handle_global_msg(GlobalMsg::CloseHelp);
            }
            return Ok(true);  // Consume all other keys (except Tab, handled above)
        }

        // Priority 4: F1 toggles help menu
        if key_event.code == KeyCode::F(1) {
            self.help_modal.open_empty();
            self.help_scroll_state.scroll_to_top();
            self.global_focused_id = Some(FocusId::new("help-scroll")); // Auto-focus scrollable
            return Ok(true);
        }

        // Priority 5: Ctrl+Space navigates to app launcher
        if KeyBinding::ctrl(KeyCode::Char(' ')).matches(&key_event) {
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
        if KeyBinding::new(KeyCode::Tab).matches(&key_event) {
            let runtime = self.runtimes
                .get_mut(&self.active_app)
                .expect("Active app not found in runtimes");
            runtime.focus_next()?;
            return Ok(true);
        }
        if KeyBinding::shift(KeyCode::Tab).matches(&key_event) || key_event.code == KeyCode::BackTab {
            let runtime = self.runtimes
                .get_mut(&self.active_app)
                .expect("Active app not found in runtimes");
            runtime.focus_previous()?;
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

        // When any global modal is open, check for interactions
        if self.quit_modal.is_open() || self.help_modal.is_open() {
            match mouse_event.kind {
                MouseEventKind::Down(_) => {
                    // Check for button clicks in global interaction registry
                    if let Some(msg) = self.global_interaction_registry.find_click(mouse_event.column, mouse_event.row) {
                        return self.handle_global_msg(msg);
                    }
                }
                MouseEventKind::ScrollUp => {
                    if self.help_modal.is_open() {
                        self.help_scroll_state.scroll_up(3);  // 3 lines per scroll
                    }
                    return Ok(true);
                }
                MouseEventKind::ScrollDown => {
                    if self.help_modal.is_open() {
                        self.help_scroll_state.scroll_down(3);  // 3 lines per scroll
                    }
                    return Ok(true);
                }
                _ => {}
            }
            return Ok(true); // Consume all mouse events when modal is open
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
        let config = crate::global_runtime_config();
        let theme = &config.theme;
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
        use ratatui::widgets::Paragraph;
        use ratatui::style::Style;
        let dim_overlay = Paragraph::new("")
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_overlay, area);

        // Build help content directly as Element<GlobalMsg>
        let global_bindings = vec![
            (KeyBinding::new(KeyCode::F(1)), "Toggle help menu"),
            (KeyBinding::ctrl(KeyCode::Char(' ')), "Go to app launcher"),
            (KeyBinding::new(KeyCode::Esc), "Close help menu"),
        ];

        // Get all apps' key bindings
        let mut all_app_bindings: Vec<(AppId, &'static str, Vec<(KeyBinding, String)>)> = vec![];
        for (app_id, runtime) in &self.runtimes {
            let title = runtime.get_title();
            let bindings = runtime.get_key_bindings();
            all_app_bindings.push((*app_id, title, bindings));
        }

        let current_app_data = all_app_bindings.iter()
            .find(|(id, _, _)| *id == self.active_app)
            .expect("Active app not found");

        let other_apps: Vec<_> = all_app_bindings.iter()
            .filter(|(id, _, bindings)| *id != self.active_app && !bindings.is_empty())
            .collect();

        // Pre-group all bindings to calculate max width across entire help menu
        let global_grouped = group_bindings_by_description(&global_bindings);

        let current_bindings_ref: Vec<(KeyBinding, &str)> = current_app_data.2.iter()
            .map(|(key, desc)| (*key, desc.as_str()))
            .collect();
        let current_grouped = group_bindings_by_description(&current_bindings_ref);

        let other_apps_grouped: Vec<(AppId, &'static str, Vec<(String, String)>)> = other_apps.iter()
            .map(|(id, title, bindings)| {
                let bindings_ref: Vec<(KeyBinding, &str)> = bindings.iter()
                    .map(|(key, desc)| (*key, desc.as_str()))
                    .collect();
                let grouped = group_bindings_by_description(&bindings_ref);
                (*id, *title, grouped)
            })
            .collect();

        // Calculate max key width across ALL sections
        let max_key_width = global_grouped.iter()
            .chain(current_grouped.iter())
            .chain(other_apps_grouped.iter().flat_map(|(_, _, grouped)| grouped.iter()))
            .map(|(keys, _)| keys.len())
            .max()
            .unwrap_or(0);

        // Build help content
        let mut help_items: Vec<Element<GlobalMsg>> = vec![
            Element::styled_text(Line::from(vec![
                Span::styled("Keyboard Shortcuts", Style::default().fg(theme.lavender).bold())
            ])).build(),
            Element::text(""),
        ];

        // Global section
        if !global_grouped.is_empty() {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled("▼ Global", Style::default().fg(theme.peach).bold())
            ])).build());

            for (keys, desc) in global_grouped {
                let padding = " ".repeat(max_key_width - keys.len());
                let line = Line::from(vec![
                    Span::styled(format!("  {}{}", keys, padding), Style::default().fg(theme.mauve)),
                    Span::raw("  "),
                    Span::styled(desc, Style::default().fg(theme.text)),
                ]);
                help_items.push(Element::styled_text(line).build());
            }
            help_items.push(Element::text(""));
        }

        // Current app section (only show if it has bindings)
        if !current_grouped.is_empty() {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled(format!("▼ {}", current_app_data.1), Style::default().fg(theme.blue).bold())
            ])).build());

            for (keys, desc) in current_grouped {
                let padding = " ".repeat(max_key_width - keys.len());
                let line = Line::from(vec![
                    Span::styled(format!("  {}{}", keys, padding), Style::default().fg(theme.green)),
                    Span::raw("  "),
                    Span::styled(desc, Style::default().fg(theme.text)),
                ]);
                help_items.push(Element::styled_text(line).build());
            }
            help_items.push(Element::text(""));
        }

        // Other apps sections
        for (_, app_title, grouped) in other_apps_grouped {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled(format!("▼ {}", app_title), Style::default().fg(theme.overlay1).bold())
            ])).build());

            for (keys, desc) in grouped {
                let padding = " ".repeat(max_key_width - keys.len());
                let line = Line::from(vec![
                    Span::styled(format!("  {}{}", keys, padding), Style::default().fg(theme.overlay2)),
                    Span::raw("  "),
                    Span::styled(desc, Style::default().fg(theme.subtext0)),
                ]);
                help_items.push(Element::styled_text(line).build());
            }
            help_items.push(Element::text(""));
        }

        help_items.push(Element::text(""));
        help_items.push(Element::styled_text(Line::from(vec![
            Span::styled("[ESC to close | ↑↓/PgUp/PgDn/Home/End to scroll]", Style::default().fg(theme.overlay1))
        ])).build());

        // Build column with all items
        let mut column_builder = ColumnBuilder::new();
        for item in help_items {
            column_builder = column_builder.add(item, LayoutConstraint::Length(1));
        }
        let help_column = column_builder.spacing(0).build();

        // Calculate modal dimensions
        let modal_width = area.width.min(60);
        let modal_height = area.height.min(20);
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Clear the modal area to prevent bleed-through from dim overlay
        use ratatui::widgets::Clear;
        frame.render_widget(Clear, modal_area);

        let content_height = modal_height.saturating_sub(4) as usize;

        // Count total items for scroll state
        let total_items = 2 + // title + blank
            (if !global_bindings.is_empty() { 2 + global_bindings.len() } else { 0 }) +
            2 + current_app_data.2.len() +
            other_apps.iter().map(|(_, _, bindings)| 2 + bindings.len()).sum::<usize>() +
            2; // blank + footer

        self.help_scroll_state.update_dimensions(total_items, content_height);

        // Wrap in scrollable with on_navigate
        let scrollable = Element::scrollable("help-scroll", help_column, &self.help_scroll_state)
            .on_navigate(GlobalMsg::HelpScroll)
            .build();

        let modal = Element::panel(scrollable)
            .title("Help")
            .build();

        // Render with global registries
        use crate::tui::Renderer;
        use crate::tui::renderer::DropdownRegistry;
        let mut dropdown_registry: DropdownRegistry<GlobalMsg> = DropdownRegistry::new();

        // Sync runtime's focus to active layer BEFORE clearing registries
        self.global_focus_registry.save_layer_focus(self.global_focused_id.clone());

        // Clear and render to global registries
        self.global_interaction_registry = crate::tui::InteractionRegistry::new();
        self.global_focus_registry = crate::tui::renderer::FocusRegistry::new();
        Renderer::render(frame, theme, &mut self.global_interaction_registry, &mut self.global_focus_registry, &mut dropdown_registry, self.global_focused_id.as_ref(), &modal, modal_area);

        // Check if focused element still exists in the tree
        if let Some(focused_id) = &self.global_focused_id {
            if !self.global_focus_registry.contains(focused_id) {
                // Clear stale focus first
                self.global_focused_id = None;

                // Try to restore from layer stack (only valid IDs)
                self.global_focused_id = self.global_focus_registry.restore_focus_from_layers();
            }
        } else {
            // No focus currently - try to restore from layer stack if there are focusables
            self.global_focused_id = self.global_focus_registry.restore_focus_from_layers();
        }

        // Save validated/restored focus to the active layer for next frame
        self.global_focus_registry.save_layer_focus(self.global_focused_id.clone());
    }

    /// Render quit confirmation modal
    fn render_quit_confirm(&mut self, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
        use ratatui::widgets::Paragraph;
        use ratatui::style::Style;

        // Render dim overlay
        let dim_overlay = Paragraph::new("")
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_overlay, area);

        // Build quit confirmation modal using Element<GlobalMsg>
        let button_row = RowBuilder::new()
            .add(
                Element::button("quit-cancel-btn", "No (N/Esc)")
                    .on_press(GlobalMsg::QuitCancel)
                    .build(),
                LayoutConstraint::Fill(1),
            )
            .add(
                Element::text("  "),
                LayoutConstraint::Length(2),
            )
            .add(
                Element::button("quit-confirm-btn", "Yes (Y)")
                    .on_press(GlobalMsg::QuitConfirm)
                    .build(),
                LayoutConstraint::Fill(1),
            )
            .spacing(0)
            .build();

        let modal_content = ColumnBuilder::new()
            .add(
                Element::text("Quit Application"),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text("Are you sure you want to quit?"),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                button_row,
                LayoutConstraint::Length(3),
            )
            .spacing(0)
            .build();

        let quit_modal = Element::panel(
            Element::container(modal_content)
                .padding(1)
                .build()
        )
        .title("Confirmation")
        .width(50)
        .height(11)
        .build();

        // Calculate modal position
        let modal_width = 50;
        let modal_height = 11;
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        // Clear the modal area to prevent bleed-through from dim overlay
        use ratatui::widgets::Clear;
        frame.render_widget(Clear, modal_area);

        // Render with global registries
        use crate::tui::Renderer;
        use crate::tui::renderer::DropdownRegistry;
        let mut dropdown_registry: DropdownRegistry<GlobalMsg> = DropdownRegistry::new();

        // Sync runtime's focus to active layer BEFORE clearing registries
        self.global_focus_registry.save_layer_focus(self.global_focused_id.clone());

        // Clear and render to global registries
        self.global_interaction_registry = crate::tui::InteractionRegistry::new();
        self.global_focus_registry = crate::tui::renderer::FocusRegistry::new();
        Renderer::render(frame, theme, &mut self.global_interaction_registry, &mut self.global_focus_registry, &mut dropdown_registry, self.global_focused_id.as_ref(), &quit_modal, modal_area);

        // Check if focused element still exists in the tree
        if let Some(focused_id) = &self.global_focused_id {
            if !self.global_focus_registry.contains(focused_id) {
                // Clear stale focus first
                self.global_focused_id = None;

                // Try to restore from layer stack (only valid IDs)
                self.global_focused_id = self.global_focus_registry.restore_focus_from_layers();
            }
        } else {
            // No focus currently - try to restore from layer stack if there are focusables
            self.global_focused_id = self.global_focus_registry.restore_focus_from_layers();
        }

        // Save validated/restored focus to the active layer for next frame
        self.global_focus_registry.save_layer_focus(self.global_focused_id.clone());
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
        let nav_target = if let Some(runtime) = self.runtimes.get_mut(&self.active_app) {
            runtime.take_navigation()
        } else {
            None
        };

        if let Some(target) = nav_target {
            // Suspend current app if it has a Sleep policy
            if let Some(current_runtime) = self.runtimes.get(&self.active_app) {
                let policy = self.factories.get(&self.active_app)
                    .map(|f| f.quit_policy())
                    .unwrap_or(crate::tui::QuitPolicy::Sleep);

                match policy {
                    crate::tui::QuitPolicy::Sleep => {
                        // Keep app in background
                        self.lifecycles.insert(self.active_app, AppLifecycle::Background);
                        if let Some(runtime) = self.runtimes.get_mut(&self.active_app) {
                            runtime.on_suspend().ok();
                        }
                    }
                    crate::tui::QuitPolicy::QuitOnExit => {
                        // Remove app immediately
                        if let Some(mut runtime) = self.runtimes.remove(&self.active_app) {
                            runtime.on_destroy().ok();
                        }
                        self.lifecycles.insert(self.active_app, AppLifecycle::Dead);
                    }
                    crate::tui::QuitPolicy::QuitOnIdle(_) => {
                        // For now, treat like Sleep
                        self.lifecycles.insert(self.active_app, AppLifecycle::Background);
                        if let Some(runtime) = self.runtimes.get_mut(&self.active_app) {
                            runtime.on_suspend().ok();
                        }
                    }
                }
            }

            // Ensure target app exists (create with default params if needed)
            self.ensure_app_exists(target, Box::new(())).ok();

            // Resume target app if it was backgrounded
            if matches!(self.lifecycles.get(&target), Some(AppLifecycle::Background)) {
                if let Some(runtime) = self.runtimes.get_mut(&target) {
                    runtime.on_resume().ok();
                }
                self.lifecycles.insert(target, AppLifecycle::Running);
            }

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