use crossterm::event::KeyCode;
use crate::tui::{App, Command, Element, Subscription, Theme, FocusId};
use crate::tui::widgets::{TextInputField, SelectField, AutocompleteField};
use dynamics_lib_macros::{AppState, Validate};
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::{col, spacer, button_row, use_constraints};

pub struct Example8;

#[derive(Clone)]
pub enum Msg {
    Cancel,
    Submit,
}

#[derive(AppState, Validate)]
pub struct State {
    #[widget("migration-name")]
    #[validate(not_empty, message = "Name is required")]
    name: TextInputField,

    #[widget("source-env", options = "self.environments")]
    #[validate(required, message = "Source environment is required")]
    source: SelectField,

    #[widget("entity-type", options = "self.entities")]
    #[validate(not_empty, message = "Entity is required")]
    entity: AutocompleteField,

    error: Option<String>,
    environments: Vec<String>,
    entities: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            name: TextInputField::new(),
            source: SelectField::new(),
            entity: AutocompleteField::new(),
            error: None,
            environments: vec![
                "Development".to_string(),
                "Staging".to_string(),
                "Production".to_string(),
            ],
            entities: vec![
                "Contact".to_string(),
                "Account".to_string(),
                "Opportunity".to_string(),
                "Lead".to_string(),
                "Case".to_string(),
            ],
        }
    }
}

impl App for Example8 {
    type State = State;
    type Msg = Msg;

    fn init() -> (State, Command<Msg>) {
        (State::default(), Command::set_focus(FocusId::new("migration-name")))
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Cancel => {
                state.name = TextInputField::new();
                state.source = SelectField::new();
                state.entity = AutocompleteField::new();
                state.error = None;
                Command::None
            }
            Msg::Submit => {
                // Validate using generated macro method
                match state.validate() {
                    Ok(_) => {
                        // Success - show confirmation
                        state.error = Some(format!(
                            "✓ Success! Created migration '{}' from {} for entity {}",
                            state.name.value(),
                            state.source.value().unwrap(),
                            state.entity.value()
                        ));
                        Command::None
                    }
                    Err(validation_error) => {
                        state.error = Some(validation_error);
                        Command::None
                    }
                }
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        use_constraints!();

        // Title
        let title = Element::styled_text(Line::from(vec![
            Span::styled("Form Builder with Auto-Routing", Style::default().fg(theme.mauve).bold())
        ])).build();

        let description = Element::styled_text(Line::from(vec![
            Span::styled("This form uses ", Style::default().fg(theme.text)),
            Span::styled("#[derive(AppState, Validate)]", Style::default().fg(theme.green).bold()),
            Span::styled(" - zero boilerplate!", Style::default().fg(theme.text)),
        ])).build();

        let description2 = Element::styled_text(Line::from(vec![
            Span::styled("• AppState: auto-routes widget events | Validate: declarative validation", Style::default().fg(theme.subtext1)),
        ])).build();

        // Name input
        let name_input = Element::panel(
            Element::text_input(
                FocusId::new("migration-name"),
                state.name.value(),
                &state.name.state,
            )
            .placeholder("Enter migration name")
            .build()
        )
        .title("Migration Name")
        .build();

        // Source environment select
        let source_select = Element::panel(
            Element::select(
                FocusId::new("source-env"),
                state.environments.clone(),
                &mut state.source.state,
            )
            .build()
        )
        .title("Source Environment")
        .build();

        // Entity autocomplete
        let entity_autocomplete = Element::panel(
            Element::autocomplete(
                FocusId::new("entity-type"),
                state.entities.clone(),
                state.entity.value().to_string(),
                &mut state.entity.state,
            )
            .placeholder("Type entity name...")
            .build()
        )
        .title("Entity Type")
        .build();

        // Error/success message
        let message = if let Some(ref error) = state.error {
            let color = if error.starts_with("✓") { theme.green } else { theme.red };
            Element::styled_text(Line::from(vec![
                Span::styled(error.clone(), Style::default().fg(color).bold())
            ])).build()
        } else {
            Element::text("")
        };

        // Build layout using declarative macros
        col![
            title => Length(1),
            description => Length(1),
            description2 => Length(1),
            spacer!() => Length(1),
            name_input => Length(3),
            spacer!() => Length(1),
            source_select => Length(10),
            spacer!() => Length(1),
            entity_autocomplete => Length(10),
            spacer!() => Length(1),
            message => Length(2),
            spacer!() => Fill(1),
            button_row![
                ("cancel-btn", "[ Cancel ]", Msg::Cancel),
                ("submit-btn", "[ Submit ]", Msg::Submit)
            ] => Length(3)
        ]
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Example 8 - Auto-Routing Form"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<ratatui::text::Line<'static>> {
        None
    }
}
