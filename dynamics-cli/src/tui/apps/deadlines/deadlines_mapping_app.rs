use crossterm::event::KeyCode;
use std::path::PathBuf;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayeredView, Resource};
use crate::tui::element::LayoutConstraint::*;
use crate::tui::widgets::{SelectField, SelectEvent, ListState};
use crate::{col, spacer};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};

use super::field_mappings;

pub struct DeadlinesMappingApp;

// Wrapper type for warnings to implement ListItem
#[derive(Clone)]
struct Warning(String);

impl crate::tui::widgets::ListItem for Warning {
    type Msg = Msg;

    fn to_element(&self, theme: &Theme, is_selected: bool, _is_hovered: bool) -> Element<Msg> {
        let (fg_color, bg_style) = if is_selected {
            (theme.lavender, Some(Style::default().bg(theme.surface0)))
        } else {
            (theme.text, None)
        };

        let mut builder = Element::styled_text(Line::from(vec![
            Span::styled("⚠ ", Style::default().fg(theme.yellow)),
            Span::styled(self.0.clone(), Style::default().fg(fg_color)),
        ]));

        if let Some(bg) = bg_style {
            builder = builder.background(bg);
        }

        builder.build()
    }
}

#[derive(Clone)]
pub struct State {
    environment_name: String,
    file_path: PathBuf,
    sheet_name: String,
    entities: Resource<Vec<String>>,
    detected_entity: Option<String>,
    manual_override: Option<String>,
    entity_selector: SelectField,
    warnings: Vec<Warning>,
    warnings_list_state: ListState,
}

impl State {
    fn new(environment_name: String, file_path: PathBuf, sheet_name: String) -> Self {
        Self {
            environment_name,
            file_path,
            sheet_name,
            entities: Resource::NotAsked,
            detected_entity: None,
            manual_override: None,
            entity_selector: SelectField::new(),
            warnings: Vec::new(),
            warnings_list_state: ListState::default(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(String::new(), PathBuf::new(), String::new())
    }
}

#[derive(Clone)]
pub enum Msg {
    EntitiesLoaded(Result<Vec<String>, String>),
    EntitySelectorEvent(SelectEvent),
    Back,
    Continue,
}

impl crate::tui::AppState for State {}

impl App for DeadlinesMappingApp {
    type State = State;
    type Msg = Msg;
    type InitParams = super::models::MappingParams;

    fn init(params: Self::InitParams) -> (State, Command<Msg>) {
        let mut state = State::new(
            params.environment_name.clone(),
            params.file_path,
            params.sheet_name,
        );
        state.entities = Resource::Loading;

        // Load entities from cache or API
        let cmd = Command::perform(
            {
                let environment_name = params.environment_name.clone();
                async move {
                    use crate::api::metadata::parse_entity_list;
                    let config = crate::global_config();
                    let manager = crate::client_manager();

                    // Try cache first (24 hours)
                    match config.get_entity_cache(&environment_name, 24).await {
                        Ok(Some(cached)) => Ok::<Vec<String>, String>(cached),
                        _ => {
                            // Fetch from API
                            let client = manager
                                .get_client(&environment_name)
                                .await
                                .map_err(|e| e.to_string())?;
                            let metadata_xml = client.fetch_metadata().await.map_err(|e| e.to_string())?;
                            let entities = parse_entity_list(&metadata_xml).map_err(|e| e.to_string())?;

                            // Cache for future use
                            let _ = config.set_entity_cache(&environment_name, entities.clone()).await;

                            Ok(entities)
                        }
                    }
                }
            },
            Msg::EntitiesLoaded,
        );

        (state, cmd)
    }

    fn update(state: &mut State, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::EntitiesLoaded(result) => {
                state.entities = Resource::from_result(result);

                // Detect which entity type we should use
                if let Resource::Success(ref entities) = state.entities {
                    state.detected_entity = field_mappings::detect_deadline_entity(entities);

                    // If no detection, initialize selector with cgk/nrq options
                    if state.detected_entity.is_none() {
                        let options = vec!["cgk_deadline".to_string(), "nrq_deadline".to_string()];
                        state.entity_selector.state.update_option_count(options.len());
                        state.entity_selector.state.select(0);
                        state.entity_selector.set_value(Some(options[0].clone()));
                        state.manual_override = Some(options[0].clone());
                        return Command::set_focus(crate::tui::FocusId::new("entity-selector"));
                    }
                }

                Command::None
            }
            Msg::EntitySelectorEvent(event) => {
                let options = vec!["cgk_deadline".to_string(), "nrq_deadline".to_string()];
                let cmd = state.entity_selector.handle_event(event, &options);
                state.manual_override = state.entity_selector.value().map(|s| s.to_string());
                cmd
            }
            Msg::Back => Command::start_app(
                AppId::DeadlinesFileSelect,
                super::models::FileSelectParams {
                    environment_name: state.environment_name.clone(),
                },
            ),
            Msg::Continue => {
                // TODO: Next app
                panic!("Continue to next app - not implemented yet");
            }
        }
    }

    fn view(state: &mut State, theme: &Theme) -> LayeredView<Msg> {
        let content = match &state.entities {
            Resource::NotAsked => {
                col![Element::styled_text(Line::from(vec![Span::styled(
                    "Waiting...",
                    Style::default().fg(theme.subtext0)
                )]))
                .build()]
            }
            Resource::Loading => {
                col![Element::styled_text(Line::from(vec![Span::styled(
                    "Loading entities...",
                    Style::default().fg(theme.blue)
                )]))
                .build()]
            }
            Resource::Failure(err) => {
                col![
                    Element::styled_text(Line::from(vec![Span::styled(
                        "Error loading entities:",
                        Style::default().fg(theme.red).bold()
                    )]))
                    .build(),
                    spacer!(),
                    Element::styled_text(Line::from(vec![Span::styled(
                        err.clone(),
                        Style::default().fg(theme.red)
                    )]))
                    .build(),
                    spacer!(),
                    Element::button("back-button", "Back")
                        .on_press(Msg::Back)
                        .build(),
                ]
            }
            Resource::Success(entities) => {
                use crate::tui::element::ColumnBuilder;
                let mut builder = ColumnBuilder::new();

                // Top section: info lines
                builder = builder.add(Element::styled_text(Line::from(vec![
                    Span::styled("Environment: ", Style::default().fg(theme.subtext0)),
                    Span::styled(
                        state.environment_name.clone(),
                        Style::default().fg(theme.lavender)
                    ),
                ]))
                .build(), Length(1));

                builder = builder.add(Element::styled_text(Line::from(vec![
                    Span::styled("File: ", Style::default().fg(theme.subtext0)),
                    Span::styled(
                        state.file_path.file_name().unwrap().to_string_lossy().to_string(),
                        Style::default().fg(theme.text)
                    ),
                ]))
                .build(), Length(1));

                builder = builder.add(Element::styled_text(Line::from(vec![
                    Span::styled("Sheet: ", Style::default().fg(theme.subtext0)),
                    Span::styled(state.sheet_name.clone(), Style::default().fg(theme.text)),
                ]))
                .build(), Length(1));

                builder = builder.add(spacer!(), Length(1));
                builder = builder.add(spacer!(), Length(1));

                // Mapping info based on detected entity or manual override
                let selected_entity = state.detected_entity.as_ref()
                    .or(state.manual_override.as_ref());

                match selected_entity {
                    Some(entity_type) => {
                        let mapping_count = field_mappings::get_mappings_for_entity(entity_type).len();

                        if state.detected_entity.is_some() {
                            builder = builder.add(Element::styled_text(Line::from(vec![
                                Span::styled("✓ ", Style::default().fg(theme.green)),
                                Span::styled(
                                    format!("Detected entity: "),
                                    Style::default().fg(theme.subtext0)
                                ),
                                Span::styled(
                                    entity_type.clone(),
                                    Style::default().fg(theme.lavender).bold()
                                ),
                            ]))
                            .build(), Length(1));
                        } else {
                            builder = builder.add(Element::styled_text(Line::from(vec![
                                Span::styled("⚠ ", Style::default().fg(theme.yellow)),
                                Span::styled(
                                    "Could not auto-detect entity. Using manual selection: ",
                                    Style::default().fg(theme.yellow)
                                ),
                                Span::styled(
                                    entity_type.clone(),
                                    Style::default().fg(theme.lavender).bold()
                                ),
                            ]))
                            .build(), Length(1));
                        }

                        builder = builder.add(spacer!(), Length(1));
                        builder = builder.add(Element::styled_text(Line::from(vec![
                            Span::styled(
                                format!("Will use {} static field mappings", mapping_count),
                                Style::default().fg(theme.text)
                            ),
                        ]))
                        .build(), Length(1));
                        builder = builder.add(spacer!(), Length(1));
                        builder = builder.add(Element::styled_text(Line::from(vec![
                            Span::styled(
                                "Checkbox columns will be detected dynamically from entity metadata",
                                Style::default().fg(theme.subtext0).italic()
                            ),
                        ]))
                        .build(), Length(1));
                    }
                    None => {
                        builder = builder.add(Element::styled_text(Line::from(vec![
                            Span::styled("⚠ ", Style::default().fg(theme.yellow)),
                            Span::styled(
                                "Could not detect cgk_deadline or nrq_deadline entity",
                                Style::default().fg(theme.yellow)
                            ),
                        ]))
                        .build(), Length(1));
                        builder = builder.add(spacer!(), Length(1));

                        // Add manual entity selector
                        let options = vec!["cgk_deadline".to_string(), "nrq_deadline".to_string()];
                        let selector = Element::select("entity-selector", options, &mut state.entity_selector.state)
                            .on_event(Msg::EntitySelectorEvent)
                            .build();

                        let selector_panel = Element::panel(selector)
                            .title("Select Entity Type")
                            .build();

                        builder = builder.add(selector_panel, Length(5));
                        builder = builder.add(spacer!(), Length(1));
                        builder = builder.add(Element::styled_text(Line::from(vec![Span::styled(
                            format!(
                                "Available entities ({}): {}",
                                entities.len(),
                                entities.join(", ")
                            ),
                            Style::default().fg(theme.subtext0)
                        )]))
                        .build(), Length(1));
                    }
                }

                // Warnings list (empty for now, will be filled with unmappable columns)
                let warnings_list = Element::list("warnings-list", &state.warnings, &state.warnings_list_state, theme)
                    .build();

                let warnings_panel = Element::panel(warnings_list)
                    .title("Mapping Warnings")
                    .build();

                builder = builder.add(warnings_panel, Fill(1));

                // Bottom section: buttons
                builder = builder.add(crate::row![
                    Element::button("back-button", "Back")
                        .on_press(Msg::Back)
                        .build(),
                    spacer!(),
                    Element::button(
                        "continue-button",
                        "Continue"
                    )
                    .on_press(Msg::Continue)
                    .build(),
                ], Length(3));

                builder.build()
            }
        };

        let outer_panel = Element::panel(content)
            .title("Deadlines - Field Mapping Configuration")
            .build();

        LayeredView::new(outer_panel)
    }

    fn subscriptions(_state: &State) -> Vec<Subscription<Msg>> {
        vec![]
    }

    fn title() -> &'static str {
        "Deadlines - Mapping"
    }

    fn status(state: &State, theme: &Theme) -> Option<Line<'static>> {
        Some(Line::from(vec![
            Span::styled("Environment: ", Style::default().fg(theme.subtext0)),
            Span::styled(
                state.environment_name.clone(),
                Style::default().fg(theme.lavender),
            ),
        ]))
    }
}
