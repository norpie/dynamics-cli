use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme};

pub struct Example1;

#[derive(Clone)]
pub enum Msg {
    NavigateToExample2,
    ButtonHovered,
    ButtonUnhovered,
    LoadData,
    DataLoaded(String),
}

#[derive(Default)]
pub struct State {
    button_hovered: bool,
    loading: bool,
    data: Option<String>,
}

impl App for Example1 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NavigateToExample2 => Command::navigate_to(AppId::Example2),
            Msg::ButtonHovered => {
                state.button_hovered = true;
                Command::None
            }
            Msg::ButtonUnhovered => {
                state.button_hovered = false;
                Command::None
            }
            Msg::LoadData => {
                state.loading = true;
                // Simulate an async API call
                Command::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        "Data loaded from async operation!".to_string()
                    },
                    Msg::DataLoaded,
                )
            }
            Msg::DataLoaded(data) => {
                state.loading = false;
                state.data = Some(data);
                Command::None
            }
        }
    }

    fn view(state: &State, theme: &Theme) -> Element<Msg> {
        let button_style = if state.button_hovered {
            ratatui::style::Style::default().fg(theme.lavender)
        } else {
            ratatui::style::Style::default().fg(theme.text)
        };

        let data_display = if state.loading {
            "Loading..."
        } else if let Some(ref data) = state.data {
            data.as_str()
        } else {
            "No data loaded yet"
        };

        Element::column(vec![
            Element::text("Example 1 - New Architecture!"),
            Element::text(""),
            Element::button("[ Press 2 or click to go to Example 2 ]")
                .on_press(Msg::NavigateToExample2)
                .on_hover(Msg::ButtonHovered)
                .on_hover_exit(Msg::ButtonUnhovered)
                .style(button_style)
                .build(),
            Element::text(""),
            Element::button("[ Press L to load data async ]")
                .on_press(Msg::LoadData)
                .build(),
            Element::text(""),
            Element::text(format!("Status: {}", data_display)),
            Element::text(""),
            Element::text("This demonstrates:"),
            Element::text("- Declarative UI with Element tree"),
            Element::text("- Message-driven state updates"),
            Element::text("- Co-located event handlers"),
            Element::text("- Async command execution"),
        ])
        .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Char('2'), Msg::NavigateToExample2),
            Subscription::keyboard(KeyCode::Char('l'), Msg::LoadData),
            Subscription::keyboard(KeyCode::Char('L'), Msg::LoadData),
        ]
    }
}