use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId};
use crate::tui::renderer::LayeredView;
use crate::tui::widgets::list::{ListItem, ListState};
use crate::tui::apps::screens::ErrorScreenParams;
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};
use crate::{col, row, spacer, use_constraints};

pub struct EnvironmentSelectorApp;

#[derive(Clone)]
pub struct Environment {
    name: String,
    is_current: bool,
}

impl ListItem for Environment {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let indicator = if self.is_current { "● " } else { "  " };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled(
                format!("{}{}", indicator, self.name),
                Style::default().fg(fg_color)
            ),
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
    current_environment: Option<String>,
}

impl State {
    fn new() -> Self {
        Self {
            environments: Vec::new(),
            list_state: ListState::default(),
            current_environment: None,
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
    DataLoaded(Result<(Vec<String>, Option<String>), String>),
    SelectEnvironment(usize),
    EnvironmentChanged(Result<(), String>),
    ListNavigate(KeyCode),
}

impl crate::tui::AppState for State {}

impl App for EnvironmentSelectorApp {
    type State = State;
    type Msg = Msg;
    type InitParams = ();

    fn init(_params: ()) -> (State, Command<Msg>) {
        let state = State::default();
        let cmd = Command::batch(vec![
            Command::perform(
                async {
                    let config = crate::global_config();
                    let manager = crate::client_manager();

                    let envs = config.list_environments().await
                        .map_err(|e| e.to_string())?;
                    let current = manager.get_current_environment_name().await
                        .map_err(|e| e.to_string())?;

                    Ok((envs, current))
                },
                Msg::DataLoaded
            ),
            Command::set_focus(FocusId::new("environment-list")),
        ]);
        (state, cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::DataLoaded(Ok((envs, current))) => {
                state.current_environment = current.clone();
                state.environments = envs.into_iter().map(|name| Environment {
                    is_current: Some(&name) == current.as_ref(),
                    name,
                }).collect();
                Command::None
            }
            Msg::DataLoaded(Err(err)) => {
                log::error!("Failed to load environments: {}", err);
                Command::start_app(
                    AppId::ErrorScreen,
                    ErrorScreenParams {
                        message: format!("Failed to load environments: {}", err),
                        target: Some(AppId::EnvironmentSelector),
                    }
                )
            }
            Msg::SelectEnvironment(idx) => {
                if let Some(env) = state.environments.get(idx) {
                    let env_name = env.name.clone();
                    Command::perform(
                        async move {
                            let manager = crate::client_manager();
                            manager.set_current_environment_in_config(env_name).await
                                .map_err(|e| e.to_string())
                        },
                        Msg::EnvironmentChanged
                    )
                } else {
                    Command::None
                }
            }
            Msg::EnvironmentChanged(Ok(())) => {
                // Reload data to refresh the current indicator
                Command::perform(
                    async {
                        let config = crate::global_config();
                        let manager = crate::client_manager();

                        let envs = config.list_environments().await
                            .map_err(|e| e.to_string())?;
                        let current = manager.get_current_environment_name().await
                            .map_err(|e| e.to_string())?;

                        Ok((envs, current))
                    },
                    Msg::DataLoaded
                )
            }
            Msg::EnvironmentChanged(Err(err)) => {
                log::error!("Failed to change environment: {}", err);
                Command::start_app(
                    AppId::ErrorScreen,
                    ErrorScreenParams {
                        message: format!("Failed to change environment: {}", err),
                        target: Some(AppId::EnvironmentSelector),
                    }
                )
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
            .title("Select Environment")
            .build();

        LayeredView::new(main_ui)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Environment Selector"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        state.current_environment.as_ref().map(|env| {
            Line::from(vec![
                Span::styled("Current: ", Style::default().fg(theme.subtext0)),
                Span::styled(env.clone(), Style::default().fg(theme.green)),
            ])
        })
    }
}
