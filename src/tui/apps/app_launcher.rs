use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::element::ColumnBuilder;
use crate::tui::widgets::{ListItem, ListState};

pub struct AppLauncher;

#[derive(Clone)]
pub enum Msg {
    LaunchApp(usize),
    ListNavigate(KeyCode),
}

pub struct State {
    apps: Vec<AppInfo>,
    list_state: ListState,
}

impl Default for State {
    fn default() -> Self {
        Self {
            apps: vec![
                AppInfo {
                    id: AppId::Example1,
                    name: "Example 1".to_string(),
                    description: "Constraint layout system demo".to_string(),
                },
                AppInfo {
                    id: AppId::Example2,
                    name: "Example 2".to_string(),
                    description: "Modal confirmation demo".to_string(),
                },
            ],
            list_state: ListState::with_selection(),
        }
    }
}

#[derive(Clone)]
struct AppInfo {
    id: AppId,
    name: String,
    description: String,
}

impl ListItem for AppInfo {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let style = if is_selected {
            Style::default().bg(theme.surface0).fg(theme.lavender)
        } else {
            Style::default().fg(theme.text)
        };

        Element::styled_text(Line::from(vec![
            Span::styled(format!("  {} - {}", self.name, self.description), style),
        ]))
    }
}

impl App for AppLauncher {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::LaunchApp(idx) => {
                if let Some(app_info) = state.apps.get(idx) {
                    Command::navigate_to(app_info.id)
                } else {
                    Command::None
                }
            }
            Msg::ListNavigate(key) => {
                // Handle list navigation
                let visible_height = 20; // Approximate, will be corrected during render
                state.list_state.handle_key(key, state.apps.len(), visible_height);
                Command::None
            }
        }
    }

    fn view(state: &State, theme: &Theme) -> Element<Msg> {
        let list = Element::list(
            FocusId::new("app-list"),
            &state.apps,
            &state.list_state,
            theme,
        )
        .on_activate(Msg::LaunchApp)
        .on_navigate(Msg::ListNavigate)
        .build();

        // Wrap list in panel to ensure consistent border
        let list_panel = Element::panel(list)
            .title("Apps")
            .build();

        ColumnBuilder::new()
            .add(
                Element::styled_text(Line::from(vec![
                    Span::styled("App Launcher", Style::default().fg(theme.blue).bold()),
                ])),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text("Select an app to launch:"),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(list_panel, LayoutConstraint::Fill(1))
            .add(
                Element::text(""),
                LayoutConstraint::Length(1),
            )
            .add(
                Element::styled_text(Line::from(vec![
                    Span::styled("↑/↓: Navigate  ", Style::default().fg(theme.overlay1)),
                    Span::styled("Enter: Launch  ", Style::default().fg(theme.overlay1)),
                    Span::styled("Tab: Focus", Style::default().fg(theme.overlay1)),
                ])),
                LayoutConstraint::Length(1),
            )
            .spacing(0)
            .build()
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        // Navigation is handled via focus system - no keyboard subscriptions needed
        vec![]
    }

    fn title() -> &'static str {
        "App Launcher"
    }

    fn status(_state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(Span::styled("[Ready]", Style::default().fg(theme.green))))
    }
}
