use crossterm::event::KeyCode;
use crate::tui::{App, Command, Element, Subscription, Theme, FocusId, LayoutConstraint};
use crate::tui::widgets::{TextInputField, SelectField, AutocompleteField};
use crate::tui::element::ColumnBuilder;
use dynamics_lib_macros::AppState;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;

pub struct Example8;

#[derive(Clone)]
pub enum Msg {
    Initialize,
    Cancel,
    Submit,
}

#[derive(AppState)]
pub struct State {
    initialized: bool,

    #[widget("migration-name")]
    name: TextInputField,

    #[widget("source-env", options = "self.environments")]
    source: SelectField,

    #[widget("entity-type", options = "self.entities")]
    entity: AutocompleteField,

    error: Option<String>,
    environments: Vec<String>,
    entities: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            initialized: false,
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

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Initialize => {
                if !state.initialized {
                    state.initialized = true;
                    Command::set_focus(FocusId::new("migration-name"))
                } else {
                    Command::None
                }
            }
            Msg::Cancel => {
                state.name = TextInputField::new();
                state.source = SelectField::new();
                state.entity = AutocompleteField::new();
                state.error = None;
                Command::None
            }
            Msg::Submit => {
                // Validation
                if state.name.value().trim().is_empty() {
                    state.error = Some("Name is required".to_string());
                    return Command::None;
                }

                if state.source.value().is_none() {
                    state.error = Some("Source environment is required".to_string());
                    return Command::None;
                }

                if state.entity.value().trim().is_empty() {
                    state.error = Some("Entity is required".to_string());
                    return Command::None;
                }

                // Success - show confirmation
                state.error = Some(format!(
                    "✓ Success! Created migration '{}' from {} for entity {}",
                    state.name.value(),
                    state.source.value().unwrap(),
                    state.entity.value()
                ));

                Command::None
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        // Title
        let title = Element::styled_text(Line::from(vec![
            Span::styled("Form Builder with Auto-Routing", Style::default().fg(theme.mauve).bold())
        ])).build();

        let description = Element::styled_text(Line::from(vec![
            Span::styled("This form uses ", Style::default().fg(theme.text)),
            Span::styled("#[derive(AppState)]", Style::default().fg(theme.green).bold()),
            Span::styled(" - all widget events are auto-routed!", Style::default().fg(theme.text)),
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

        // Buttons
        let buttons = Element::row(vec![
            Element::button(FocusId::new("cancel-btn"), "[ Cancel ]")
                .on_press(Msg::Cancel)
                .build(),
            Element::text("  "),
            Element::button(FocusId::new("submit-btn"), "[ Submit ]")
                .on_press(Msg::Submit)
                .build(),
        ]).build();

        // Build layout
        ColumnBuilder::new()
            .add(title, LayoutConstraint::Length(1))
            .add(description, LayoutConstraint::Length(1))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(name_input, LayoutConstraint::Length(3))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(source_select, LayoutConstraint::Length(10))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(entity_autocomplete, LayoutConstraint::Length(10))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(message, LayoutConstraint::Length(2))
            .add(Element::text(""), LayoutConstraint::Fill(1))
            .add(buttons, LayoutConstraint::Length(3))
            .build()
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
        "Example 8 - Auto-Routing Form"
    }

    fn status(_state: &State, _theme: &Theme) -> Option<ratatui::text::Line<'static>> {
        None
    }
}
