use crossterm::event::KeyCode;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::tui::{App, AppId, Command, Element, Subscription, Theme, LayeredView, Resource};
use crate::tui::element::LayoutConstraint::*;
use crate::tui::widgets::{SelectField, SelectEvent, ListState};
use crate::{col, spacer};
use ratatui::text::{Line, Span};
use ratatui::style::{Style, Stylize};
use calamine::{Reader, open_workbook, Xlsx};

use super::field_mappings;

/// Fetch entity data from cache or API
async fn fetch_entity_data(
    environment_name: &str,
    entity_name: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let config = crate::global_config();
    let manager = crate::client_manager();

    // Try cache first (24 hours) - but force refresh if cache has 0 records
    match config.get_entity_data_cache(environment_name, entity_name, 24).await {
        Ok(Some(cached)) if !cached.is_empty() => {
            log::debug!("Using cached data for {} ({} records)", entity_name, cached.len());
            return Ok(cached);
        }
        Ok(Some(cached)) => {
            log::debug!("Cache for {} has 0 records, forcing refresh", entity_name);
        }
        Ok(None) => {
            log::debug!("No cache for {}, fetching from API", entity_name);
        }
        Err(e) => {
            log::warn!("Cache lookup failed for {}: {}", entity_name, e);
        }
    }

    // Fetch from API
    let client = manager
        .get_client(environment_name)
        .await
        .map_err(|e| e.to_string())?;

    // Pluralize entity name for Web API (entity sets are plural)
    let plural_name = crate::api::pluralization::pluralize_entity_name(entity_name);
    log::debug!("Fetching {} (plural: {})", entity_name, plural_name);

    // Fetch all records for this entity using query builder
    let query = crate::api::QueryBuilder::new(&plural_name).build();
    log::debug!("Executing query for {}: {:?}", plural_name, query);

    let result = client
        .execute_query(&query)
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", entity_name, e))?;

    let records = result.records()
        .cloned()
        .unwrap_or_default();

    log::debug!("Fetched {} records for {}", records.len(), entity_name);

    if records.is_empty() {
        log::warn!("Entity {} returned 0 records - entity might be empty or query might need adjustment", entity_name);
    } else if let Some(first) = records.first() {
        log::debug!("First record from {}: {:?}", entity_name, first);
    }

    // Cache for future use
    let _ = config.set_entity_data_cache(environment_name, entity_name, &records).await;

    Ok(records)
}

/// Process Excel file: find header row, identify checkbox columns, validate mappings
fn process_excel_file(state: &mut State) -> Command<Msg> {
    state.excel_processed = true;
    state.warnings.clear();

    let file_path = state.file_path.clone();
    let sheet_name = state.sheet_name.clone();

    // Open the Excel file
    let mut workbook: Xlsx<_> = match open_workbook(&file_path) {
        Ok(wb) => wb,
        Err(e) => {
            state.warnings.push(Warning(format!("Failed to open Excel file: {}", e)));
            return Command::None;
        }
    };

    // Get the specified sheet
    let range = match workbook.worksheet_range(&sheet_name) {
        Ok(range) => range,
        Err(e) => {
            state.warnings.push(Warning(format!("Failed to read sheet '{}': {}", sheet_name, e)));
            return Command::None;
        }
    };

    // Find header row by looking for "Domein*" in first column
    let mut header_row_idx = None;
    let mut headers = Vec::new();

    for (row_idx, row) in range.rows().enumerate() {
        if let Some(first_cell) = row.first() {
            let cell_str = first_cell.to_string();
            if cell_str.contains("Domein") {
                header_row_idx = Some(row_idx);
                headers = row.iter().map(|cell| cell.to_string()).collect();
                break;
            }
        }
    }

    if header_row_idx.is_none() {
        state.warnings.push(Warning("Could not find header row (looking for 'Domein*' in first column)".to_string()));
        return Command::None;
    }

    let header_row_idx = header_row_idx.unwrap();
    log::debug!("Found header row at index {}", header_row_idx);
    log::debug!("Headers: {:?}", headers);

    // Find "Raad van Bestuur" column index
    let rvb_idx = headers.iter().position(|h| h.to_lowercase().contains("raad") && h.to_lowercase().contains("bestuur"));

    if rvb_idx.is_none() {
        state.warnings.push(Warning("Could not find 'Raad van Bestuur' column".to_string()));
        return Command::None;
    }

    let rvb_idx = rvb_idx.unwrap();
    log::debug!("Found 'Raad van Bestuur' at column index {}", rvb_idx);

    // All columns after RvB are checkbox columns, except "OPM"
    let checkbox_columns: Vec<String> = headers.iter()
        .skip(rvb_idx + 1)
        .filter(|h| !h.is_empty() && h.to_uppercase() != "OPM")
        .map(|h| h.to_string())
        .collect();

    log::debug!("Checkbox columns: {:?}", checkbox_columns);

    // Get the entity type to determine which entity prefix to use
    let entity_type = state.detected_entity.as_ref()
        .or(state.manual_override.as_ref());

    if entity_type.is_none() {
        state.warnings.push(Warning("No entity type selected".to_string()));
        return Command::None;
    }

    let entity_type = entity_type.unwrap();
    let entity_prefix = if entity_type == "cgk_deadline" { "cgk_" } else { "nrq_" };

    // Debug: log what's in the entity data cache
    log::debug!("Entity data cache keys: {:?}", state.entity_data_cache.keys().collect::<Vec<_>>());
    for (entity_name, records) in &state.entity_data_cache {
        log::debug!("Entity '{}' has {} records", entity_name, records.len());
        if let Some(first_record) = records.first() {
            log::debug!("First record keys: {:?}", first_record.as_object().map(|o| o.keys().collect::<Vec<_>>()));
        }
    }

    // Validate checkbox columns against entity data
    for checkbox_col in &checkbox_columns {
        // Try to find matching entity records
        let mut found = false;

        // Check all checkbox entity types for this environment
        let checkbox_entity_names = vec![
            format!("{}support", entity_prefix),
            format!("{}category", entity_prefix),
            if entity_prefix == "cgk_" { "cgk_length".to_string() } else { "nrq_subcategory".to_string() },
            format!("{}flemishshare", entity_prefix),
        ];

        log::debug!("Checking checkbox column '{}' against entities: {:?}", checkbox_col, checkbox_entity_names);

        for entity_name in checkbox_entity_names {
            if let Some(records) = state.entity_data_cache.get(&entity_name) {
                log::debug!("Checking {} records in entity '{}'", records.len(), entity_name);
                // Check if any record's name matches the checkbox column header
                for (idx, record) in records.iter().enumerate() {
                    if let Some(name_value) = extract_name_from_record(record) {
                        if idx < 3 {
                            log::debug!("Record {} name: '{}'", idx, name_value);
                        }
                        if name_value.to_lowercase() == checkbox_col.to_lowercase() {
                            found = true;
                            log::debug!("✓ Matched checkbox column '{}' to entity '{}'", checkbox_col, entity_name);
                            break;
                        }
                    } else if idx < 3 {
                        log::debug!("Record {} has no name field, record: {:?}", idx, record);
                    }
                }
                if found {
                    break;
                }
            } else {
                log::debug!("Entity '{}' not found in cache", entity_name);
            }
        }

        if !found {
            state.warnings.push(Warning(format!("Checkbox column '{}' not found in entity data", checkbox_col)));
        }
    }

    log::debug!("Excel processing complete. Found {} warnings", state.warnings.len());

    Command::None
}

/// Extract name field from a record (tries common name field patterns)
fn extract_name_from_record(record: &serde_json::Value) -> Option<String> {
    // Try common name field patterns
    let name_fields = vec![
        "name",
        "cgk_name",
        "nrq_name",
        "fullname",
        "cgk_fullname",
        "nrq_fullname",
    ];

    for field in name_fields {
        if let Some(value) = record.get(field) {
            if let Some(s) = value.as_str() {
                return Some(s.to_string());
            }
        }
    }

    None
}

/// Start loading entity data in parallel
fn start_entity_data_loading(state: &mut State, entity_type: &str) -> Command<Msg> {
    let cache_entities = field_mappings::get_cache_entities(entity_type);

    if cache_entities.is_empty() {
        return Command::None;
    }

    state.entity_data_loading = true;
    state.entity_data_loaded_count = 0;
    state.entity_data_total_count = cache_entities.len();

    let environment_name = state.environment_name.clone();

    let mut builder = Command::perform_parallel();

    for (index, entity_name) in cache_entities.iter().enumerate() {
        let entity_name_clone = entity_name.clone();
        let env_name_clone = environment_name.clone();

        builder = builder.add_task(
            format!("Loading {} records", entity_name),
            async move {
                fetch_entity_data(&env_name_clone, &entity_name_clone).await
            }
        );
    }

    builder
        .with_title("Loading entity data for lookups")
        .on_complete(AppId::DeadlinesMapping)
        .cancellable(false)
        .build(move |task_index, result| {
            let typed_result = result.downcast::<Result<Vec<serde_json::Value>, String>>()
                .map(|boxed| *boxed)
                .unwrap_or_else(|_| Err("Type mismatch in task result".to_string()));
            Msg::EntityDataLoaded(task_index, typed_result)
        })
}

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
    entity_data_loading: bool,
    entity_data_loaded_count: usize,
    entity_data_total_count: usize,
    entity_data_cache: HashMap<String, Vec<serde_json::Value>>,
    excel_processed: bool,
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
            entity_data_loading: false,
            entity_data_loaded_count: 0,
            entity_data_total_count: 0,
            entity_data_cache: HashMap::new(),
            excel_processed: false,
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
    EntityDataLoaded(usize, Result<Vec<serde_json::Value>, String>),
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

                    // If we detected an entity, start loading entity data immediately
                    if let Some(entity_type) = state.detected_entity.clone() {
                        return start_entity_data_loading(state, &entity_type);
                    }

                    // If no detection, initialize selector with cgk/nrq options
                    if state.detected_entity.is_none() {
                        let options = vec!["cgk_deadline".to_string(), "nrq_deadline".to_string()];
                        state.entity_selector.state.update_option_count(options.len());
                        state.entity_selector.state.select(0);
                        state.entity_selector.set_value(Some(options[0].clone()));
                        state.manual_override = Some(options[0].clone());

                        // Start loading entity data for the default selection
                        return Command::batch(vec![
                            Command::set_focus(crate::tui::FocusId::new("entity-selector")),
                            start_entity_data_loading(state, &options[0]),
                        ]);
                    }
                }

                Command::None
            }
            Msg::EntitySelectorEvent(event) => {
                let options = vec!["cgk_deadline".to_string(), "nrq_deadline".to_string()];
                let cmd = state.entity_selector.handle_event(event, &options);

                // If the selection changed, update manual override and reload entity data
                if let Some(new_selection) = state.entity_selector.value() {
                    let new_selection_str = new_selection.to_string();
                    if state.manual_override.as_ref() != Some(&new_selection_str) {
                        state.manual_override = Some(new_selection_str.clone());
                        return Command::batch(vec![
                            cmd,
                            start_entity_data_loading(state, &new_selection_str),
                        ]);
                    }
                }

                state.manual_override = state.entity_selector.value().map(|s| s.to_string());
                cmd
            }
            Msg::EntityDataLoaded(task_index, result) => {
                state.entity_data_loaded_count += 1;

                match result {
                    Ok(records) => {
                        log::debug!("Loaded {} records for task {}", records.len(), task_index);

                        // Store the entity data in cache
                        let entity_type = state.detected_entity.as_ref()
                            .or(state.manual_override.as_ref());
                        if let Some(entity_type) = entity_type {
                            let cache_entities = field_mappings::get_cache_entities(entity_type);
                            if task_index < cache_entities.len() {
                                let entity_name = cache_entities[task_index].clone();
                                state.entity_data_cache.insert(entity_name, records);
                            }
                        }
                    }
                    Err(err) => {
                        state.warnings.push(Warning(format!("Failed to load entity data: {}", err)));
                    }
                }

                // Check if all tasks completed
                if state.entity_data_loaded_count >= state.entity_data_total_count {
                    state.entity_data_loading = false;

                    // Process Excel file now that we have all entity data
                    if !state.excel_processed {
                        return process_excel_file(state);
                    }
                }

                Command::None
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
