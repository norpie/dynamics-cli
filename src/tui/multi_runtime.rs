use ratatui::Frame;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use anyhow::Result;

use crate::tui::{AppId, Runtime, apps::{Example1, Example2}, Element, LayoutConstraint, Layer, Theme, ThemeVariant, App};
use crate::tui::element::{ColumnBuilder, RowBuilder};

/// Manages multiple app runtimes and handles navigation between them
pub struct MultiAppRuntime {
    example1: Runtime<Example1>,
    example2: Runtime<Example2>,
    active_app: AppId,

    // Global UI state
    help_menu_open: bool,
    help_scroll_offset: usize,
}

impl MultiAppRuntime {
    pub fn new() -> Self {
        Self {
            example1: Runtime::new(),
            example2: Runtime::new(),
            active_app: AppId::Example1,
            help_menu_open: false,
            help_scroll_offset: 0,
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) -> Result<bool> {
        // Global keys: F1 toggles help menu
        if key_event.code == KeyCode::F(1) {
            self.help_menu_open = !self.help_menu_open;
            self.help_scroll_offset = 0; // Reset scroll when opening
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
                    self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
                    return Ok(true);
                }
                KeyCode::Down => {
                    self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
                    return Ok(true);
                }
                KeyCode::PageUp => {
                    self.help_scroll_offset = self.help_scroll_offset.saturating_sub(10);
                    return Ok(true);
                }
                KeyCode::PageDown => {
                    self.help_scroll_offset = self.help_scroll_offset.saturating_add(10);
                    return Ok(true);
                }
                _ => {
                    // Consume all other keys when help is open
                    return Ok(true);
                }
            }
        }

        // Normal: delegate to active app
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
        let (app_title, app_status) = match self.active_app {
            AppId::Example1 => (self.example1.get_title(), self.example1.get_status()),
            AppId::Example2 => (self.example2.get_title(), self.example2.get_status()),
        };
        self.render_header(frame, header_area, app_title, app_status, &theme);

        // Render active app content
        match self.active_app {
            AppId::Example1 => self.example1.render_to_area(frame, app_area),
            AppId::Example2 => self.example2.render_to_area(frame, app_area),
        }

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

        let header_left = Element::styled_text(title_line);
        let header_right = Element::styled_text(Line::from(vec![
            Span::styled("[?] F1 Help", Style::default().fg(theme.overlay1))
        ]));

        let header = Element::panel(
            RowBuilder::new()
                .add(header_left, LayoutConstraint::Fill(1))
                .add(header_right, LayoutConstraint::Length(15))
                .spacing(0)
                .build()
        )
        .build();

        use crate::tui::{Renderer, InteractionRegistry};
        let mut registry: InteractionRegistry<()> = InteractionRegistry::new();
        Renderer::render(frame, theme, &mut registry, &header, area);
    }

    fn render_help_menu(&self, frame: &mut Frame, area: ratatui::layout::Rect, theme: &Theme) {
        // Build global key bindings
        let global_bindings = vec![
            (KeyCode::F(1), "Toggle help menu".to_string()),
            (KeyCode::Esc, "Close help menu".to_string()),
        ];

        // Get all apps' key bindings and names
        let example1_bindings = self.example1.get_key_bindings();
        let example2_bindings = self.example2.get_key_bindings();

        // Determine current and other apps
        let (current_app_name, current_app_bindings, other_apps) = match self.active_app {
            AppId::Example1 => ("Example 1", &example1_bindings, vec![("Example 2", &example2_bindings)]),
            AppId::Example2 => ("Example 2", &example2_bindings, vec![("Example 1", &example1_bindings)]),
        };

        // Build help content
        let mut help_items = vec![
            Element::styled_text(Line::from(vec![
                Span::styled("Keyboard Shortcuts", Style::default().fg(theme.lavender).bold())
            ])),
            Element::text(""),
        ];

        // Skip items for scrolling
        let mut skipped = 0;
        let skip_target = self.help_scroll_offset;

        // Section 1: Global Keys (highest priority)
        if skipped < skip_target {
            skipped += 1;
        } else {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled("▼ Global", Style::default().fg(theme.peach).bold())
            ])));
        }

        for (key, description) in &global_bindings {
            if skipped < skip_target {
                skipped += 1;
                continue;
            }
            let key_str = format!("{:?}", key);
            let line = Line::from(vec![
                Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.mauve)),
                Span::raw("  "),
                Span::styled(description.clone(), Style::default().fg(theme.text)),
            ]);
            help_items.push(Element::styled_text(line));
        }

        if skipped < skip_target {
            skipped += 1;
        } else {
            help_items.push(Element::text(""));
        }

        // Section 2: Current App Keys
        if skipped < skip_target {
            skipped += 1;
        } else {
            help_items.push(Element::styled_text(Line::from(vec![
                Span::styled(format!("▼ {}", current_app_name), Style::default().fg(theme.blue).bold())
            ])));
        }

        for (key, description) in current_app_bindings {
            if skipped < skip_target {
                skipped += 1;
                continue;
            }
            let key_str = format!("{:?}", key);
            let line = Line::from(vec![
                Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.green)),
                Span::raw("  "),
                Span::styled(description.clone(), Style::default().fg(theme.text)),
            ]);
            help_items.push(Element::styled_text(line));
        }

        if skipped < skip_target {
            skipped += 1;
        } else {
            help_items.push(Element::text(""));
        }

        // Section 3: Other Apps
        for (app_name, app_bindings) in other_apps {
            if skipped < skip_target {
                skipped += 1;
            } else {
                help_items.push(Element::styled_text(Line::from(vec![
                    Span::styled(format!("▼ {}", app_name), Style::default().fg(theme.overlay1).bold())
                ])));
            }

            for (key, description) in app_bindings {
                if skipped < skip_target {
                    skipped += 1;
                    continue;
                }
                let key_str = format!("{:?}", key);
                let line = Line::from(vec![
                    Span::styled(format!("  {:13}", key_str), Style::default().fg(theme.overlay2)),
                    Span::raw("  "),
                    Span::styled(description.clone(), Style::default().fg(theme.subtext0)),
                ]);
                help_items.push(Element::styled_text(line));
            }

            if skipped < skip_target {
                skipped += 1;
            } else {
                help_items.push(Element::text(""));
            }
        }

        help_items.push(Element::styled_text(Line::from(vec![
            Span::styled("[ESC to close | ↑↓ to scroll]", Style::default().fg(theme.overlay1))
        ])));

        let help_content = Element::column(help_items).build();

        // Wrap in panel and center
        let help_modal = Element::panel(
            Element::container(help_content)
                .padding(1)
                .build()
        )
        .title("Help")
        .build();

        use crate::tui::{Renderer, InteractionRegistry};
        let mut registry: InteractionRegistry<()> = InteractionRegistry::new();

        // Calculate centered position for help modal
        let modal_width = area.width.min(60);
        let modal_height = area.height.min(20);
        let modal_area = ratatui::layout::Rect {
            x: area.x + (area.width.saturating_sub(modal_width)) / 2,
            y: area.y + (area.height.saturating_sub(modal_height)) / 2,
            width: modal_width,
            height: modal_height,
        };

        Renderer::render(frame, theme, &mut registry, &help_modal, modal_area);
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