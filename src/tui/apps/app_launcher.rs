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
    Initialize,
    LaunchApp(usize),
    ListNavigate(KeyCode),
}

pub struct State {
    apps: Vec<AppInfo>,
    list_state: ListState,
    initialized: bool,
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
                AppInfo {
                    id: AppId::Example3,
                    name: "Example 3".to_string(),
                    description: "Text input form demo".to_string(),
                },
                AppInfo {
                    id: AppId::Example4,
                    name: "Example 4".to_string(),
                    description: "Tab pattern demo".to_string(),
                },
                AppInfo {
                    id: AppId::Example5,
                    name: "Example 5".to_string(),
                    description: "Tree widget demo - hierarchical navigation".to_string(),
                },
                AppInfo {
                    id: AppId::Example6,
                    name: "Example 6".to_string(),
                    description: "Select widget demo - dropdowns and options".to_string(),
                },
                AppInfo {
                    id: AppId::MigrationEnvironment,
                    name: "Migration Environments".to_string(),
                    description: "Migration environment selection".to_string(),
                },
            ],
            list_state: ListState::with_selection(),
            initialized: false,
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
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(format!("  {} - {}", self.name, self.description), Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

impl App for AppLauncher {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize => {
                // Auto-focus the list on app start
                if !state.initialized {
                    state.initialized = true;
                    Command::set_focus(FocusId::new("app-list"))
                } else {
                    Command::None
                }
            }
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

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let list = Element::list(
            FocusId::new("app-list"),
            &state.apps,
            &state.list_state,
            theme,
        )
        .on_activate(Msg::LaunchApp)
        .on_navigate(Msg::ListNavigate)
        .build();

        // Just the list in a panel, filling the entire area
        Element::panel(list)
            .title("Apps")
            .build()
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        let mut subs = vec![];

        // Fire initialization once on app start
        if !state.initialized {
            subs.push(Subscription::timer(std::time::Duration::from_millis(1), Msg::Initialize));
        }

        subs
    }

    fn title() -> &'static str {
        "App Launcher"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<Line<'static>> {
        None
    }
}
