use crossterm::event::KeyCode;
use ratatui::text::{Line, Span};
use ratatui::style::Style;
use ratatui::prelude::Stylize;
use crate::tui::{App, Command, Element, Subscription, Theme, LayoutConstraint, FocusId};
use crate::tui::widgets::AutocompleteState;
use crate::tui::element::ColumnBuilder;

pub struct Example7;

#[derive(Clone)]
pub enum Msg {
    InputChanged(KeyCode),
    DropdownNavigate(KeyCode),
    EntitySelected(String),
    Back,
}

pub struct State {
    autocomplete_state: AutocompleteState,
    all_entities: Vec<String>,
    current_input: String,
    selected_entity: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        let all_entities = vec![
            "account".to_string(),
            "contact".to_string(),
            "opportunity".to_string(),
            "lead".to_string(),
            "systemuser".to_string(),
            "team".to_string(),
            "businessunit".to_string(),
            "invoice".to_string(),
            "quote".to_string(),
            "salesorder".to_string(),
            "product".to_string(),
            "pricelevel".to_string(),
            "contract".to_string(),
            "incident".to_string(),
            "case".to_string(),
            "email".to_string(),
            "phonecall".to_string(),
            "task".to_string(),
            "appointment".to_string(),
            "letter".to_string(),
            "fax".to_string(),
            "activitypointer".to_string(),
            "campaign".to_string(),
            "campaignresponse".to_string(),
            "list".to_string(),
            "service".to_string(),
            "resource".to_string(),
            "equipment".to_string(),
            "territory".to_string(),
            "queue".to_string(),
            "workflow".to_string(),
            "plugin".to_string(),
            "solution".to_string(),
            "publisher".to_string(),
            "connectionrole".to_string(),
            "role".to_string(),
            "privilege".to_string(),
            "securityrole".to_string(),
            "position".to_string(),
            "site".to_string(),
        ];

        Self {
            autocomplete_state: AutocompleteState::new(),
            all_entities,
            current_input: String::new(),
            selected_entity: None,
        }
    }
}

impl App for Example7 {
    type State = State;
    type Msg = Msg;

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::InputChanged(key) => {
                // Handle text input changes
                if let Some(new_value) = state.autocomplete_state.handle_input_key(
                    key,
                    &state.current_input,
                    None,  // No max length
                ) {
                    state.current_input = new_value;
                    // Update filtered options based on new input
                    state.autocomplete_state.update_filtered_options(
                        &state.current_input,
                        &state.all_entities,
                    );
                }
                Command::None
            }
            Msg::DropdownNavigate(key) => {
                // Handle dropdown navigation
                match key {
                    KeyCode::Up => {
                        state.autocomplete_state.navigate_prev();
                    }
                    KeyCode::Down => {
                        state.autocomplete_state.navigate_next();
                    }
                    KeyCode::Enter => {
                        // Select highlighted option
                        if let Some(selected) = state.autocomplete_state.get_highlighted_option() {
                            state.current_input = selected.clone();
                            state.selected_entity = Some(selected);
                            state.autocomplete_state.close();
                            // Move cursor to end after setting value
                            state.autocomplete_state.set_cursor_to_end(&state.current_input);
                        }
                    }
                    KeyCode::Esc => {
                        state.autocomplete_state.close();
                    }
                    _ => {}
                }
                Command::None
            }
            Msg::EntitySelected(entity) => {
                // Direct selection from click
                state.current_input = entity.clone();
                state.selected_entity = Some(entity);
                state.autocomplete_state.close();
                // Move cursor to end after setting value
                state.autocomplete_state.set_cursor_to_end(&state.current_input);
                Command::None
            }
            Msg::Back => Command::navigate_to(crate::tui::AppId::AppLauncher),
        }
    }

    fn view(state: &mut State, theme: &Theme) -> Element<Msg> {
        let title = Element::styled_text(Line::from(vec![
            Span::styled("Autocomplete Widget Demo", Style::default().fg(theme.mauve).bold())
        ])).build();

        let description = Element::text(
            "Start typing an entity name. Fuzzy matching will filter results. Use Up/Down to navigate, Enter to select."
        );

        let autocomplete = Element::autocomplete(
            FocusId::new("entity_autocomplete"),
            state.all_entities.clone(),
            state.current_input.clone(),
            &mut state.autocomplete_state,
        )
        .placeholder("Type entity name...")
        .on_input(Msg::InputChanged)
        .on_select(Msg::EntitySelected)
        .on_navigate(Msg::DropdownNavigate)
        .build();

        let selected_display = if let Some(ref entity) = state.selected_entity {
            Element::styled_text(Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(theme.subtext1)),
                Span::styled(entity.clone(), Style::default().fg(theme.green).bold()),
            ])).build()
        } else {
            Element::styled_text(Line::from(vec![
                Span::styled("No entity selected", Style::default().fg(theme.overlay1).italic()),
            ])).build()
        };

        let help = Element::styled_text(Line::from(vec![
            Span::styled("Press ", Style::default().fg(theme.subtext1)),
            Span::styled("Esc ", Style::default().fg(theme.blue).bold()),
            Span::styled("or ", Style::default().fg(theme.subtext1)),
            Span::styled("B ", Style::default().fg(theme.blue).bold()),
            Span::styled("to go back", Style::default().fg(theme.subtext1)),
        ])).build();

        ColumnBuilder::new()
            .add(title, LayoutConstraint::Length(1))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(description, LayoutConstraint::Length(2))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(autocomplete, LayoutConstraint::Length(3))
            .add(Element::text(""), LayoutConstraint::Length(1))
            .add(selected_display, LayoutConstraint::Length(1))
            .add(Element::text(""), LayoutConstraint::Fill(1))
            .add(help, LayoutConstraint::Length(1))
            .spacing(0)
            .build()
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![
            Subscription::keyboard(KeyCode::Esc, "Back to app launcher", Msg::Back),
            Subscription::keyboard(KeyCode::Char('b'), "Back to app launcher", Msg::Back),
            Subscription::keyboard(KeyCode::Char('B'), "Back to app launcher", Msg::Back),
        ]
    }

    fn title() -> &'static str {
        "Example 7: Autocomplete Widget"
    }

    fn status(_state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::styled("Fuzzy-matched autocomplete input", Style::default().fg(theme.text)),
        ]))
    }
}
