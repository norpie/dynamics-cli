use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use serde_json::Value;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint};
use crate::tui::element::ColumnBuilder;

pub struct ErrorScreen;

#[derive(Clone)]
pub enum Msg {
    Initialize(Value),
    Continue,
}

#[derive(Default)]
pub struct State {
    error_message: String,
    target_app: Option<AppId>,
}

impl App for ErrorScreen {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize(data) => {
                // Reset ALL state from previous runs
                *state = State::default();

                // Parse initialization data
                state.error_message = data.get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("An error occurred")
                    .to_string();

                state.target_app = data
                    .get("target")
                    .and_then(|v| v.as_str())
                    .and_then(|s| match s {
                        "AppLauncher" => Some(AppId::AppLauncher),
                        "Example1" => Some(AppId::Example1),
                        "Example2" => Some(AppId::Example2),
                        "LoadingScreen" => Some(AppId::LoadingScreen),
                        "ErrorScreen" => Some(AppId::ErrorScreen),
                        _ => None,
                    });

                Command::None
            }

            Msg::Continue => {
                if let Some(target) = state.target_app {
                    Command::navigate_to(target)
                } else {
                    Command::None
                }
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let content = vec![
            Element::styled_text(Line::from(vec![
                Span::styled("❌ Error", Style::default().fg(theme.red).bold()),
            ])).build(),
            Element::text(""),
            Element::text(&state.error_message),
            Element::text(""),
            Element::styled_text(Line::from(vec![
                Span::styled("Press Enter to continue", Style::default().fg(theme.overlay1)),
            ])).build(),
        ];

        // Wrap in panel
        Element::panel(
            Element::container(
                ColumnBuilder::new()
                    .add(Element::column(content).build(), LayoutConstraint::Fill(1))
                    .build()
            )
            .padding(2)
            .build()
        )
        .title("Error")
        .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::subscribe("error:init", |data| Some(Msg::Initialize(data))),
            Subscription::keyboard(KeyCode::Enter, "Continue", Msg::Continue),
        ]
    }

    fn title() -> &'static str {
        "Error"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(Span::styled("[Error]", Style::default().fg(theme.red))))
    }
}
