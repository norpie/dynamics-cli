use crossterm::event::KeyCode;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme};

pub struct Example2;

#[derive(Clone)]
pub enum Msg {
    RequestNavigate,
    ConfirmNavigate,
    CancelNavigate,
    ButtonHovered,
    ButtonUnhovered,
}

#[derive(Default)]
pub struct State {
    button_hovered: bool,
    show_confirm: bool,
}

impl App for Example2 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::RequestNavigate => {
                state.show_confirm = true;
                // Clear hover state when modal opens
                state.button_hovered = false;
                Command::None
            }
            Msg::ConfirmNavigate => {
                state.show_confirm = false;
                Command::navigate_to(AppId::Example1)
            }
            Msg::CancelNavigate => {
                state.show_confirm = false;
                Command::None
            }
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

        let main_ui = Element::column(vec![
            Element::text("Example 2 - Modal Confirmation Demo!"),
            Element::text(""),
            Element::button("[ Press 1 or click to go to Example 1 ]")
                .on_press(Msg::RequestNavigate)
                .on_hover(Msg::ButtonHovered)
                .on_hover_exit(Msg::ButtonUnhovered)
                .style(button_style)
                .build(),
            Element::text(""),
            Element::text("Now with confirmation modal!"),
            Element::text("Stack/Layer system in action."),
            Element::text(""),
            Element::text("Try navigating - you'll see a modal popup."),
        ])
        .build();

        if state.show_confirm {
            Element::modal_confirm(
                main_ui,
                "Confirm Navigation",
                "Are you sure you want to navigate to Example 1?",
                Msg::ConfirmNavigate,
                Msg::CancelNavigate,
            )
        } else {
            main_ui
        }
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![Subscription::keyboard(
            KeyCode::Char('1'),
            "Navigate to Example 1 (with confirmation)",
            Msg::RequestNavigate,
        )]
    }
}