use anyhow::Result;
use crossterm::{
    event::{self, poll, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Terminal,
};
use std::{collections::HashMap, io, time::Duration};
use tokio::sync::oneshot;
use log::debug;

use super::config::{DiscoveredEntity, EntityMapping, EnvironmentConfig, COMMON_ENTITY_TYPES};
use super::entity_discovery::EntityDiscovery;
use super::loading_modal::LoadingModal;
use crate::dynamics::DynamicsClient;

#[derive(Debug, Clone, PartialEq)]
enum SetupStep {
    PrefixInput,
    EntityMapping,
    Validation,
    Complete,
}

#[derive(Debug)]
struct SetupState {
    step: SetupStep,
    environment_name: String,
    prefix: String,
    discovered_entities: Vec<DiscoveredEntity>,
    entity_mappings: HashMap<String, Option<EntityMapping>>,
    selected_logical_type: usize,
    entity_selector_state: ListState,
    show_entity_selector: bool,
    validation_results: HashMap<String, bool>,
    status_message: String,
    loading_modal: Option<LoadingModal>,
    prefix_input: String,
    cursor_position: usize,
    discovery_task: Option<tokio::task::JoinHandle<()>>,
    discovery_receiver: Option<oneshot::Receiver<Result<Vec<DiscoveredEntity>>>>,
    validation_task: Option<tokio::task::JoinHandle<()>>,
    validation_receiver: Option<oneshot::Receiver<HashMap<String, bool>>>,
}

impl SetupState {
    fn new(environment_name: String, prefix: String) -> Self {
        let mut state = Self {
            step: SetupStep::PrefixInput,
            environment_name,
            prefix: prefix.clone(),
            discovered_entities: Vec::new(),
            entity_mappings: HashMap::new(),
            selected_logical_type: 0,
            entity_selector_state: ListState::default(),
            show_entity_selector: false,
            validation_results: HashMap::new(),
            status_message: String::new(),
            loading_modal: None,
            prefix_input: prefix,
            cursor_position: 0,
            discovery_task: None,
            discovery_receiver: None,
            validation_task: None,
            validation_receiver: None,
        };

        // Initialize entity mappings with None values
        for (logical_type, _) in COMMON_ENTITY_TYPES {
            state.entity_mappings.insert(logical_type.to_string(), None);
        }

        state
    }

    fn current_logical_type(&self) -> &str {
        COMMON_ENTITY_TYPES.get(self.selected_logical_type).map(|(name, _)| *name).unwrap_or("")
    }

    fn next_logical_type(&mut self) {
        if self.selected_logical_type < COMMON_ENTITY_TYPES.len() - 1 {
            self.selected_logical_type += 1;
        }
    }

    fn prev_logical_type(&mut self) {
        if self.selected_logical_type > 0 {
            self.selected_logical_type -= 1;
        }
    }

    fn start_discovery_task(&mut self, auth_config: crate::config::AuthConfig) {
        let prefix = self.prefix.clone();
        let (sender, receiver) = oneshot::channel();

        let task = tokio::spawn(async move {
            let dynamics_client = crate::dynamics::DynamicsClient::new(auth_config);
            let mut entity_discovery = crate::commands::deadlines::entity_discovery::EntityDiscovery::new(dynamics_client);
            let result = entity_discovery.discover_entities_with_prefix(&prefix).await;
            let _ = sender.send(result);
        });

        self.discovery_task = Some(task);
        self.discovery_receiver = Some(receiver);
    }

    fn start_validation_task(&mut self, auth_config: crate::config::AuthConfig) {
        let mappings: Vec<_> = self.entity_mappings.iter()
            .filter_map(|(logical_type, mapping)| {
                mapping.as_ref().map(|m| (logical_type.clone(), m.clone()))
            })
            .collect();
        let (sender, receiver) = oneshot::channel();

        let task = tokio::spawn(async move {
            let dynamics_client = crate::dynamics::DynamicsClient::new(auth_config);
            let mut entity_discovery = crate::commands::deadlines::entity_discovery::EntityDiscovery::new(dynamics_client);
            let mut results = HashMap::new();
            for (logical_type, mapping) in mappings {
                let is_valid = entity_discovery.test_entity_mapping(&mapping).await.unwrap_or(false);
                results.insert(logical_type, is_valid);
            }
            let _ = sender.send(results);
        });

        self.validation_task = Some(task);
        self.validation_receiver = Some(receiver);
    }

    fn check_discovery_result(&mut self) -> Option<Result<Vec<DiscoveredEntity>>> {
        if let Some(receiver) = &mut self.discovery_receiver {
            match receiver.try_recv() {
                Ok(result) => {
                    self.discovery_receiver = None;
                    self.discovery_task = None;
                    Some(result)
                }
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.discovery_receiver = None;
                    self.discovery_task = None;
                    Some(Err(anyhow::anyhow!("Discovery task was cancelled")))
                }
            }
        } else {
            None
        }
    }

    fn check_validation_result(&mut self) -> Option<HashMap<String, bool>> {
        if let Some(receiver) = &mut self.validation_receiver {
            match receiver.try_recv() {
                Ok(results) => {
                    self.validation_receiver = None;
                    self.validation_task = None;
                    Some(results)
                }
                Err(oneshot::error::TryRecvError::Empty) => None,
                Err(oneshot::error::TryRecvError::Closed) => {
                    self.validation_receiver = None;
                    self.validation_task = None;
                    Some(HashMap::new())
                }
            }
        } else {
            None
        }
    }
}

pub async fn run_deadline_setup(environment_name: String) -> Result<Option<EnvironmentConfig>> {
    // Start with empty prefix - user will input their own
    let prefix = String::new();

    // Create DynamicsClient using the selected environment
    let config = crate::config::Config::load()?;
    let auth_config = config.environments.get(&environment_name)
        .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found in config", environment_name))?;

    let dynamics_client = DynamicsClient::new(auth_config.clone());
    let entity_discovery = EntityDiscovery::new(dynamics_client);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_setup_app(&mut terminal, environment_name, prefix, auth_config.clone()).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_setup_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    environment_name: String,
    prefix: String,
    auth_config: crate::config::AuthConfig,
) -> Result<Option<EnvironmentConfig>> {
    let mut state = SetupState::new(environment_name.clone(), prefix.clone());

    loop {
        terminal.draw(|f| {
            draw_ui(f, &mut state);
        })?;

        // Check for async task results and force redraw if state changed
        let mut state_changed = false;

        if let Some(discovery_result) = state.check_discovery_result() {
            state_changed = true;
            log::debug!("Discovery task completed with result");
            match discovery_result {
                Ok(discovered) => {
                    debug!("Discovery succeeded with {} entities", discovered.len());
                    if discovered.is_empty() {
                        if let Some(ref mut modal) = state.loading_modal {
                            modal.set_error(format!(
                                "No entities found with prefix '{}'. Please check your environment configuration.",
                                state.prefix
                            ));
                        }
                    } else {
                        debug!("Processing {} discovered entities", discovered.len());
                        state.discovered_entities = discovered;

                        // Auto-suggest mappings using a simple algorithm (no API calls, no client needed)
                        debug!("Starting auto-suggestion for {} logical types", COMMON_ENTITY_TYPES.len());
                        let mut suggestion_count = 0;
                        for (logical_type, search_terms) in COMMON_ENTITY_TYPES {
                            // Simple matching: find entities that contain any of the search terms
                            for entity in &state.discovered_entities {
                                if search_terms.iter().any(|term| entity.name.contains(term)) {
                                    debug!("Found entity '{}' matching logical type '{}', fields: {:?}", entity.name, logical_type, entity.fields);
                                    if let (Some(id_field), Some(name_field)) = (entity.guess_id_field(), entity.guess_name_field()) {
                                        let endpoint = crate::commands::deadlines::entity_discovery::EntityDiscovery::get_entity_endpoint_static(&entity.name);
                                        debug!("Mapping suggestion for '{}': id_field='{}', name_field='{}', endpoint='{}'",
                                            logical_type, id_field, name_field, endpoint);

                                        let mapping = EntityMapping::new(
                                            entity.name.clone(),
                                            id_field,
                                            name_field,
                                            endpoint,
                                        );

                                        state.entity_mappings.insert(logical_type.to_string(), Some(mapping));
                                        suggestion_count += 1;
                                        break; // Take the first match
                                    } else {
                                        debug!("Could not guess fields for entity '{}', skipping", entity.name);
                                    }
                                }
                            }
                        }

                        debug!("Auto-suggestion complete: {} suggestions made", suggestion_count);
                        state.step = SetupStep::EntityMapping;
                        state.status_message = format!("Discovered {} entities with {} suggested mappings",
                            state.discovered_entities.len(),
                            suggestion_count
                        );
                        debug!("Clearing loading modal and transitioning to EntityMapping step");
                        state.loading_modal = None;
                    }
                }
                Err(e) => {
                    if let Some(ref mut modal) = state.loading_modal {
                        modal.set_error(format!("Discovery failed: {}\n\nPlease verify:\n• Environment connection is working\n• API permissions are correct\n• Network connectivity is available", e));
                    }
                }
            }
        }

        if let Some(validation_results) = state.check_validation_result() {
            state_changed = true;
            state.validation_results = validation_results;
            state.step = SetupStep::Complete;
            let valid_count = state.validation_results.values().filter(|&&v| v).count();
            let total_count = state.validation_results.len();
            state.status_message = format!("Validation complete: {}/{} mappings valid", valid_count, total_count);
            state.loading_modal = None;
        }

        // Force redraw if state changed from async task completion
        if state_changed {
            continue;
        }

        // Handle loading state with spinner
        if state.loading_modal.is_some() {
            if poll(Duration::from_millis(100))? {
                if let Event::Key(_key) = event::read()? {
                    // Only allow dismissing error messages
                    if let Some(ref modal) = state.loading_modal {
                        if modal.has_error() {
                            state.loading_modal = None;
                        }
                    }
                }
            } else {
                // Tick the spinner
                if let Some(ref mut modal) = state.loading_modal {
                    modal.tick();
                }
            }
            continue;
        }

        if let Event::Key(key) = event::read()? {
            debug!("Key event received in step {:?}: {:?}", state.step, key.code);
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if state.show_entity_selector {
                        state.show_entity_selector = false;
                        continue;
                    } else {
                        return Ok(None);
                    }
                }
                KeyCode::Enter => {
                    match state.step {
                        SetupStep::PrefixInput => {
                            if !state.prefix_input.trim().is_empty() {
                                state.prefix = state.prefix_input.trim().to_string();
                                // Immediately start discovery - no separate step
                                state.loading_modal = Some(LoadingModal::new(
                                    format!("Discovering entities with prefix '{}'...", state.prefix)
                                ));
                                state.start_discovery_task(auth_config.clone());
                            } else {
                                state.status_message = "Please enter a valid prefix".to_string();
                            }
                        }
                        SetupStep::EntityMapping => {
                            if state.show_entity_selector {
                                // Select entity from the list
                                if let Some(selected_idx) = state.entity_selector_state.selected() {
                                    if selected_idx < state.discovered_entities.len() {
                                        let entity = &state.discovered_entities[selected_idx];
                                        let logical_type = state.current_logical_type().to_string();

                                        let mapping = EntityMapping::new(
                                            entity.name.clone(),
                                            entity.guess_id_field().unwrap_or_else(|| format!("{}id", entity.name)),
                                            entity.guess_name_field().unwrap_or_else(|| format!("{}_name", state.prefix)),
                                            crate::commands::deadlines::entity_discovery::EntityDiscovery::get_entity_endpoint_static(&entity.name)
                                        );

                                        state.entity_mappings.insert(logical_type, Some(mapping));
                                        state.show_entity_selector = false;
                                    }
                                }
                            } else {
                                // Show entity selector for current logical type
                                state.show_entity_selector = true;
                                state.entity_selector_state.select(Some(0));
                            }
                        }
                        SetupStep::Validation => {
                            // Check if there are mappings to validate
                            let mapping_count = state.entity_mappings.values().filter(|m| m.is_some()).count();
                            if mapping_count == 0 {
                                state.loading_modal = Some(LoadingModal::new(String::new()));
                                if let Some(ref mut modal) = state.loading_modal {
                                    modal.set_error("No entity mappings found to validate. Please configure at least one entity mapping.".to_string());
                                }
                            } else {
                                // Show loading modal and start validation task
                                state.loading_modal = Some(LoadingModal::new(
                                    "Validating entity mappings via API...".to_string()
                                ));
                                state.start_validation_task(auth_config.clone());
                            }
                        }
                        SetupStep::Complete => {
                            // Create and return the environment configuration
                            let mut env_config = EnvironmentConfig::new(
                                state.prefix.clone(),
                                format!("{}_deadline", state.prefix)
                            );

                            for (logical_type, mapping_opt) in &state.entity_mappings {
                                if let Some(mapping) = mapping_opt {
                                    env_config.add_entity_mapping(logical_type.clone(), mapping.clone());
                                }
                            }

                            return Ok(Some(env_config));
                        }
                    }
                }
                KeyCode::Up => {
                    if state.show_entity_selector {
                        let i = match state.entity_selector_state.selected() {
                            Some(i) => if i == 0 { state.discovered_entities.len() - 1 } else { i - 1 },
                            None => 0,
                        };
                        state.entity_selector_state.select(Some(i));
                    } else if state.step == SetupStep::EntityMapping {
                        state.prev_logical_type();
                    }
                }
                KeyCode::Down => {
                    if state.show_entity_selector {
                        let i = match state.entity_selector_state.selected() {
                            Some(i) => if i >= state.discovered_entities.len() - 1 { 0 } else { i + 1 },
                            None => 0,
                        };
                        state.entity_selector_state.select(Some(i));
                    } else if state.step == SetupStep::EntityMapping {
                        state.next_logical_type();
                    }
                }
                KeyCode::Char('n') => {
                    if state.step == SetupStep::EntityMapping {
                        state.step = SetupStep::Validation;
                    }
                }
                KeyCode::Char('s') => {
                    if state.step == SetupStep::EntityMapping {
                        // Skip current mapping
                        let logical_type = state.current_logical_type().to_string();
                        state.entity_mappings.insert(logical_type, None);
                        state.next_logical_type();
                    }
                }
                KeyCode::Char(c) => {
                    if state.step == SetupStep::PrefixInput && c != 'q' {
                        state.prefix_input.insert(state.cursor_position, c);
                        state.cursor_position += 1;
                    }
                }
                KeyCode::Backspace => {
                    if state.step == SetupStep::PrefixInput && state.cursor_position > 0 {
                        state.cursor_position -= 1;
                        state.prefix_input.remove(state.cursor_position);
                    }
                }
                KeyCode::Left => {
                    if state.step == SetupStep::PrefixInput && state.cursor_position > 0 {
                        state.cursor_position -= 1;
                    }
                }
                KeyCode::Right => {
                    if state.step == SetupStep::PrefixInput && state.cursor_position < state.prefix_input.len() {
                        state.cursor_position += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

fn draw_ui(f: &mut ratatui::Frame, state: &mut SetupState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.area());

    // Title
    let title = format!("Deadline Setup: {} Environment", state.environment_name);
    let title_widget = Paragraph::new(title)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title_widget, chunks[0]);

    // Status
    let status_text = match state.step {
        SetupStep::PrefixInput => "Step 1/4: Prefix Input",
        SetupStep::EntityMapping => "Step 2/4: Entity Mapping",
        SetupStep::Validation => "Step 3/4: Validation",
        SetupStep::Complete => "Step 4/4: Complete",
    };

    // Only show status with borders if there's a status message
    if state.status_message.is_empty() {
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(status, chunks[1]);
    } else {
        let status = Paragraph::new(vec![
            Line::from(Span::styled(status_text, Style::default().fg(Color::Yellow))),
            Line::from(Span::raw(&state.status_message)),
        ])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, chunks[1]);
    }

    // Main content area
    match state.step {
        SetupStep::PrefixInput => draw_prefix_input_step(f, chunks[2], state),
        SetupStep::EntityMapping => draw_entity_mapping_step(f, chunks[2], state),
        SetupStep::Validation => draw_validation_step(f, chunks[2], state),
        SetupStep::Complete => draw_complete_step(f, chunks[2], state),
    }

    // Instructions
    let instructions = match state.step {
        SetupStep::PrefixInput => "Type entity prefix, Press Enter to discover entities, Esc/q to quit",
        SetupStep::EntityMapping => "Use ↑/↓ to navigate, Enter to map, 's' to skip, 'n' for next step",
        SetupStep::Validation => "Press Enter to validate configuration",
        SetupStep::Complete => "Press Enter to save configuration",
    };

    let instructions_widget = Paragraph::new(instructions)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(instructions_widget, chunks[3]);

    // Draw entity selector modal if needed
    if state.show_entity_selector {
        draw_entity_selector_modal(f, state);
    }

    // Draw loading modal if needed
    if let Some(ref loading_modal) = state.loading_modal {
        loading_modal.render(f, f.area());
    }
}

fn draw_prefix_input_step(f: &mut ratatui::Frame, area: Rect, state: &SetupState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(area);

    // Explanation
    let explanation = Paragraph::new(
        "Enter the entity prefix for your Dynamics environment.\n\n\
         This prefix is used to identify your custom entities.\n\
         Common prefixes: cgk, nrq, test, demo\n\
         Example: if you have entities like 'cgk_category', enter 'cgk'"
    )
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("Entity Prefix"));

    f.render_widget(explanation, chunks[0]);

    // Input field
    let input_text = format!("Prefix: {}", state.prefix_input);
    let input_widget = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Input"));

    f.render_widget(input_widget, chunks[1]);

    // Set cursor position in the input field
    let cursor_x = chunks[1].x + 9 + state.cursor_position as u16; // "Prefix: " is 8 chars + 1 for border
    let cursor_y = chunks[1].y + 1; // 1 for border
    f.set_cursor_position((cursor_x, cursor_y));
}

fn draw_discovery_step(f: &mut ratatui::Frame, area: Rect) {
    let content = Paragraph::new(
        "This will discover available entities in your Dynamics environment.\n\n\
         The system will:\n\
         • Query entity metadata\n\
         • Find entities with the specified prefix\n\
         • Analyze field structures\n\
         • Suggest entity mappings\n\n\
         Press Enter to begin discovery."
    )
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("Entity Discovery"));

    f.render_widget(content, area);
}

fn draw_entity_mapping_step(f: &mut ratatui::Frame, area: Rect, state: &SetupState) {
    let mut items = Vec::new();

    for (i, (logical_type, aliases)) in COMMON_ENTITY_TYPES.iter().enumerate() {
        let mapping = state.entity_mappings.get(*logical_type).unwrap_or(&None);

        let (status, color) = match mapping {
            Some(mapping) => (format!("→ {}", mapping.entity), Color::Green),
            None => ("→ [Not Mapped]".to_string(), Color::Red),
        };

        let style = if i == state.selected_logical_type {
            Style::default().fg(color).add_modifier(Modifier::BOLD).bg(Color::DarkGray)
        } else {
            Style::default().fg(color)
        };

        // Show aliases if there are multiple search terms
        let display_name = if aliases.len() > 1 {
            format!("{} ({})", logical_type, aliases.iter().skip(1).cloned().collect::<Vec<_>>().join("/"))
        } else {
            logical_type.to_string()
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{:<20}", display_name), style),
            Span::styled(status, style),
        ])));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Entity Mappings"))
        .highlight_symbol("► ");

    f.render_widget(list, area);
}

fn draw_validation_step(f: &mut ratatui::Frame, area: Rect, state: &SetupState) {
    let mut items = Vec::new();

    for (logical_type, _) in COMMON_ENTITY_TYPES {
        let (status, color) = match state.validation_results.get(*logical_type) {
            Some(true) => ("✓ Valid", Color::Green),
            Some(false) => ("✗ Invalid", Color::Red),
            None => ("⚬ Not validated", Color::Yellow),
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{:<15}", logical_type), Style::default()),
            Span::styled(status, Style::default().fg(color)),
        ])));
    }

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Validation Results"));

    f.render_widget(list, area);
}

fn draw_complete_step(f: &mut ratatui::Frame, area: Rect, _state: &SetupState) {
    let content = Paragraph::new(
        "Configuration setup is complete!\n\n\
         The deadline.toml file will be created with your entity mappings.\n\
         You can now use the deadlines command to process Excel files.\n\n\
         Press Enter to save the configuration."
    )
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("Setup Complete"));

    f.render_widget(content, area);
}

fn draw_entity_selector_modal(f: &mut ratatui::Frame, state: &mut SetupState) {
    let area = centered_rect(80, 60, f.area());

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("Select Entity for '{}'", state.current_logical_type()))
        .borders(Borders::ALL);
    f.render_widget(block, area);

    let inner_area = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

    let items: Vec<ListItem> = state.discovered_entities
        .iter()
        .map(|entity| {
            ListItem::new(Line::from(vec![
                Span::styled(&entity.name, Style::default().fg(Color::White)),
                Span::styled(format!(" ({} records)", entity.record_count), Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, inner_area, &mut state.entity_selector_state);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// These functions are now handled by EntityDiscovery
// simulate_entity_discovery and suggest_mappings have been replaced with real API calls