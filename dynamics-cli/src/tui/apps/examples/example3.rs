use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayoutConstraint, FocusId, TextInputState};
use crate::tui::element::ColumnBuilder;

pub struct Example3;

#[derive(Clone)]
pub enum Msg {
    NameKeyPressed(KeyCode),
    NameSubmit,
    EmailKeyPressed(KeyCode),
    EmailSubmit,
    GoBack,
}

pub struct State {
    name: String,
    name_input_state: TextInputState,
    email: String,
    email_input_state: TextInputState,
    submitted: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: String::new(),
            name_input_state: TextInputState::new(),
            email: String::new(),
            email_input_state: TextInputState::new(),
            submitted: false,
        }
    }
}

impl App for Example3 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::NameKeyPressed(key) => {
                if let Some(new_value) = state.name_input_state.handle_key(key, &state.name, Some(50)) {
                    state.name = new_value;
                }
                Command::None
            }
            Msg::NameSubmit => {
                // Enter in name field focuses email field
                Command::set_focus(FocusId::new("email"))
            }
            Msg::EmailKeyPressed(key) => {
                if let Some(new_value) = state.email_input_state.handle_key(key, &state.email, Some(100)) {
                    state.email = new_value;
                }
                Command::None
            }
            Msg::EmailSubmit => {
                // Enter in email field submits the form
                if !state.name.is_empty() && !state.email.is_empty() {
                    state.submitted = true;
                }
                Command::None
            }
            Msg::GoBack => {
                Command::navigate_to(AppId::AppLauncher)
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let name_input = Element::panel(
            Element::text_input(
                FocusId::new("name"),
                &state.name,
                &state.name_input_state,
            )
            .placeholder("Enter your name...")
            .max_length(50)
            .on_change(Msg::NameKeyPressed)
            .on_submit(Msg::NameSubmit)
            .build()
        )
        .title("Name")
        .build();

        let email_input = Element::panel(
            Element::text_input(
                FocusId::new("email"),
                &state.email,
                &state.email_input_state,
            )
            .placeholder("Enter your email...")
            .max_length(100)
            .on_change(Msg::EmailKeyPressed)
            .on_submit(Msg::EmailSubmit)
            .build()
        )
        .title("Email")
        .build();

        let mut builder = ColumnBuilder::new();

        builder = builder.add(
            Element::styled_text(Line::from(vec![
                Span::styled("Text Input Example", Style::default().fg(theme.blue).bold()),
            ])).build(),
            LayoutConstraint::Length(1),
        );
        builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
        builder = builder.add(name_input, LayoutConstraint::Length(3));
        builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
        builder = builder.add(email_input, LayoutConstraint::Length(3));
        builder = builder.add(Element::text(""), LayoutConstraint::Length(1));

        if state.submitted {
            builder = builder.add(
                Element::styled_text(Line::from(vec![
                    Span::styled("✓ Form Submitted!", Style::default().fg(theme.green).bold()),
                ])).build(),
                LayoutConstraint::Length(1),
            );
            builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
            builder = builder.add(
                Element::text(format!("Name: {}", state.name)),
                LayoutConstraint::Length(1),
            );
            builder = builder.add(
                Element::text(format!("Email: {}", state.email)),
                LayoutConstraint::Length(1),
            );
            builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
        } else {
            builder = builder.add(
                Element::styled_text(Line::from(vec![
                    Span::styled("Fill both fields and press Enter in email to submit", Style::default().fg(theme.overlay1)),
                ])).build(),
                LayoutConstraint::Length(1),
            );
            builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
        }

        builder = builder.add(Element::text(""), LayoutConstraint::Length(1));
        builder = builder.add(
            Element::button(FocusId::new("back"), "[ Go Back ]")
                .on_press(Msg::GoBack)
                .build(),
            LayoutConstraint::Length(3),
        );

        builder.build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Go back", Msg::GoBack),
        ]
    }

    fn title() -> &'static str {
        "Example 3 - Text Input"
    }

    fn status(_state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(Span::styled("[TextInput Demo]", Style::default().fg(theme.green))))
    }
}
