use crossterm::event::KeyCode;
use crate::tui::{App, Command, Element, Subscription, Theme, FocusId};
use crate::tui::widgets::{TextInputField, SelectField, AutocompleteField, TextInputEvent, SelectEvent, AutocompleteEvent};
use crate::form_layout;

pub struct Example8;

#[derive(Clone)]
pub enum Msg {
    Initialize,
    NameEvent(TextInputEvent),
    SourceEvent(SelectEvent),
    EntityEvent(AutocompleteEvent),
    Cancel,
    Submit,
}

pub struct State {
    initialized: bool,
    form: FormData,
    environments: Vec<String>,
    entities: Vec<String>,
}

struct FormData {
    name: TextInputField,
    source: SelectField,
    entity: AutocompleteField,
    error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            initialized: false,
            form: FormData {
                name: TextInputField::new(),
                source: SelectField::new(),
                entity: AutocompleteField::new(),
                error: None,
            },
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

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize => {
                if !state.initialized {
                    state.initialized = true;
                    Command::set_focus(FocusId::new("name"))
                } else {
                    Command::None
                }
            }
            Msg::NameEvent(event) => {
                state.form.name.handle_event(event, None);
                Command::None
            }
            Msg::SourceEvent(event) => {
                state.form.source.handle_event::<Msg>(event, &state.environments);
                Command::None
            }
            Msg::EntityEvent(event) => {
                state.form.entity.handle_event::<Msg>(event, &state.entities);
                Command::None
            }
            Msg::Cancel => {
                state.form.name = TextInputField::new();
                state.form.source = SelectField::new();
                state.form.entity = AutocompleteField::new();
                state.form.error = None;
                Command::None
            }
            Msg::Submit => {
                // Validation
                if state.form.name.value().trim().is_empty() {
                    state.form.error = Some("Name is required".to_string());
                    return Command::None;
                }

                if state.form.source.value().is_none() {
                    state.form.error = Some("Source environment is required".to_string());
                    return Command::None;
                }

                if state.form.entity.value().trim().is_empty() {
                    state.form.error = Some("Entity is required".to_string());
                    return Command::None;
                }

                // Success - clear form
                state.form.error = Some(format!(
                    "Success! Created migration '{}' from {} for entity {}",
                    state.form.name.value(),
                    state.form.source.value().unwrap(),
                    state.form.entity.value()
                ));

                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        form_layout! {
            theme: theme,
            fields: [
                text("Migration Name", "migration-name", state.form.name.value().to_string(), &mut state.form.name.state, Msg::NameEvent, placeholder: "Enter migration name") => crate::tui::LayoutConstraint::Length(3),
                spacer => crate::tui::LayoutConstraint::Length(1),
                select("Source Environment", "source-env", &mut state.form.source.state, Msg::SourceEvent, state.environments.clone()) => crate::tui::LayoutConstraint::Length(10),
                spacer => crate::tui::LayoutConstraint::Length(1),
                autocomplete("Entity Type", "entity-type", state.form.entity.value.clone(), &mut state.form.entity.state, Msg::EntityEvent, state.entities.clone()) => crate::tui::LayoutConstraint::Length(10),
                spacer => crate::tui::LayoutConstraint::Length(1),
                error(state.form.error) => crate::tui::LayoutConstraint::Length(2),
            ],
            buttons: [
                ("cancel-btn", "Cancel", Msg::Cancel),
                ("submit-btn", "Submit", Msg::Submit),
            ]
        }
    }

    fn subscriptions(state: &State) -> Vec<Subscription<Msg>> {
        let mut subs = vec![];

        if !state.initialized {
            subs.push(Subscription::timer(
                std::time::Duration::from_millis(1),
                Msg::Initialize,
            ));
        }

        subs
    }

    fn title() -> &'static str {
        "Form Builder DSL Demo"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<ratatui::text::Line<'static>> {
        None
    }
}
