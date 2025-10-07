use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint, FocusId, LayeredView};
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
                    id: AppId::EnvironmentSelector,
                    name: "Environment Selector".to_string(),
                    description: "Select and manage Dynamics 365 environments".to_string(),
                },
                AppInfo {
                    id: AppId::MigrationEnvironment,
                    name: "Migration Environments".to_string(),
                    description: "Manage Dynamics 365 migrations".to_string(),
                },
                AppInfo {
                    id: AppId::DeadlinesFileSelect,
                    name: "Deadlines".to_string(),
                    description: "Process Excel deadlines for migration".to_string(),
                },
                AppInfo {
                    id: AppId::OperationQueue,
                    name: "Operation Queue".to_string(),
                    description: "Manage and execute API operation batches".to_string(),
                },
                AppInfo {
                    id: AppId::Settings,
                    name: "Settings".to_string(),
                    description: "Configure application options".to_string(),
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

impl crate::tui::AppState for State {}

impl App for AppLauncher {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        (State::default(), Command::set_focus(FocusId::new("app-list")))
    }

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

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        let list = Element::list(
            FocusId::new("app-list"),
            &state.apps,
            &state.list_state,
            theme,
        )
        .on_select(Msg::LaunchApp)
        .on_activate(Msg::LaunchApp)
        .on_navigate(Msg::ListNavigate)
        .build();

        // Just the list in a panel, filling the entire area
        let panel = Element::panel(list)
            .title("Apps")
            .build();

        LayeredView::new(panel)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "App Launcher"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<Line<'static>> {
        None
    }
}
