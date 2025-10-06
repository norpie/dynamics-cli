use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, FocusId, LayeredView};
use crate::tui::renderer::LayeredView as LV;
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};

pub struct DeadlinesFileSelectApp;

#[derive(Clone)]
pub struct State {
    environment_name: String,
}

impl State {
    fn new(environment_name: String) -> Self {
        Self {
            environment_name,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[derive(Clone)]
pub enum Msg {
    Back,
}

impl crate::tui::AppState for State {}

impl App for DeadlinesFileSelectApp {
    type State = State;
    type Msg = Msg;
    type InitParams = super::models::FileSelectParams;

    fn init(params: Self::InitParams) -> (State, Command<Msg>) {
        let state = State::new(params.environment_name);
        (state, Command::None)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Back => {
                Command::navigate_to(AppId::DeadlinesEnvironmentSelect)
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        let content = Element::styled_text(
            Line::from(vec![
                Span::styled(
                    format!("File selector for environment: {}", state.environment_name),
                    Style::default().fg(theme.text)
                ),
            ])
        ).build();

        let panel = Element::panel(content)
            .title("Select Excel File")
            .build();

        LayeredView::new(panel)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Go back", Msg::Back),
        ]
    }

    fn title() -> &'static str {
        "Deadlines - File Selection"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::styled("Environment: ", Style::default().fg(theme.subtext0)),
            Span::styled(state.environment_name.clone(), Style::default().fg(theme.lavender)),
        ]))
    }
}
