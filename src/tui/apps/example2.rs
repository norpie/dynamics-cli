use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme};

pub struct Example2;

#[derive(Clone)]
pub enum Msg {
    NavigateToExample1,
    ButtonHovered,
    ButtonUnhovered,
}

#[derive(Default)]
pub struct State {
    button_hovered: bool,
}

impl App for Example2 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NavigateToExample1 => Command::navigate_to(AppId::Example1),
            Msg::ButtonHovered => {
                state.button_hovered = true;
                Command::None
            }
            Msg::ButtonUnhovered => {
                state.button_hovered = false;
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

        Element::column(vec![
            Element::text("Example 2 - Still the new architecture!"),
            Element::text(""),
            Element::button("[ Press 1 or click to go to Example 1 ]")
                .on_press(Msg::NavigateToExample1)
                .on_hover(Msg::ButtonHovered)
                .on_hover_exit(Msg::ButtonUnhovered)
                .style(button_style)
                .build(),
            Element::text(""),
            Element::text("Navigation works seamlessly!"),
            Element::text("Each app is completely independent."),
            Element::text(""),
            Element::text("The runtime handles all the complexity."),
        ])
        .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![Subscription::keyboard(
            KeyCode::Char('1'),
            Msg::NavigateToExample1,
        )]
    }
}