use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId};
use crate::tui::renderer::LayeredView;
use crate::tui::widgets::list::{ListItem, ListState};
use crate::tui::apps::screens::ErrorScreenParams;
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};
use crate::{col, row, spacer, use_constraints};

pub struct DeadlinesEnvironmentSelectApp;

#[derive(Clone)]
pub struct Environment {
    name: String,
}

impl ListItem for Environment {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(format!("  {}", self.name), Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

#[derive(Clone)]
pub struct State {
    environments: Vec<Environment>,
    list_state: ListState,
}

impl State {
    fn new() -> Self {
        Self {
            environments: Vec::new(),
            list_state: ListState::default(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub enum Msg {
    EnvironmentsLoaded(Result<Vec<String>, String>),
    SelectEnvironment(usize),
    ListNavigate(KeyCode),
}

impl crate::tui::AppState for State {}

impl App for DeadlinesEnvironmentSelectApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        let state = State::default();
        let cmd = Command::batch(vec![
            Command::perform(
                async {
                    let config = crate::global_config();
                    config.list_environments().await
                        .map_err(|e| e.to_string())
                },
                Msg::EnvironmentsLoaded
            ),
            Command::set_focus(FocusId::new("environment-list")),
        ]);
        (state, cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::EnvironmentsLoaded(Ok(envs)) => {
                state.environments = envs.into_iter().map(|name| Environment { name }).collect();
                Command::None
            }
            Msg::EnvironmentsLoaded(Err(err)) => {
                log::error!("Failed to load environments: {}", err);
                Command::start_app(
                    AppId::ErrorScreen,
                    ErrorScreenParams {
                        message: format!("Failed to load environments: {}", err),
                        target: Some(AppId::DeadlinesEnvironmentSelect),
                    }
                )
            }
            Msg::SelectEnvironment(idx) => {
                if let Some(env) = state.environments.get(idx) {
                    Command::start_app(
                        AppId::DeadlinesFileSelect,
                        super::models::FileSelectParams {
                            environment_name: env.name.clone(),
                        }
                    )
                } else {
                    Command::None
                }
            }
            Msg::ListNavigate(key) => {
                let visible_height = 20;
                state.list_state.handle_key(key, state.environments.len(), visible_height);
                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        let list = Element::list("environment-list", &state.environments, &state.list_state, theme)
            .on_select(Msg::SelectEnvironment)
            .on_activate(Msg::SelectEnvironment)
            .on_navigate(Msg::ListNavigate)
            .build();

        let main_ui = Element::panel(list)
            .title("Select Environment for Deadlines")
            .build();

        LayeredView::new(main_ui)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Deadlines - Select Environment"
    }
}
