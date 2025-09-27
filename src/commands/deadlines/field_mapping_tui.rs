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
use serde::{Serialize, Deserialize};

use super::config::EnvironmentConfig;
use super::excel_parser::SheetData;
use super::validation::identify_checkbox_columns;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    pub excel_column: String,
    pub target_field: String,
    pub field_type: FieldType,
    pub target_entity: Option<String>,
    pub junction_entity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldType {
    DirectField,
    LookupField,
    MultiSelect,
    Ignore,
}

#[derive(Debug)]
pub struct FieldMappingState {
    selected_excel_column: usize,
    excel_columns: Vec<String>,
    field_mappings: HashMap<String, Option<FieldMapping>>,
    main_list_state: ListState,
    show_field_selector: bool,
    field_selector_state: ListState,
    available_target_fields: Vec<TargetField>,
    show_junction_selector: bool,
    junction_selector_state: ListState,
    available_junction_entities: Vec<String>,
    current_mapping_in_progress: Option<FieldMapping>,
}

#[derive(Debug, Clone)]
pub struct TargetField {
    pub name: String,
    pub display_name: String,
    pub field_type: FieldType,
    pub target_entity: Option<String>,
}

impl FieldMappingState {
    pub fn new(excel_columns: Vec<String>, env_config: &EnvironmentConfig) -> Self {
        let mut field_mappings = HashMap::new();

        // Get checkbox columns using existing validation logic
        let checkbox_columns = identify_checkbox_columns(&excel_columns);

        for column in &excel_columns {
            if checkbox_columns.contains(column) {
                // Auto-map checkbox columns to MultiSelect (N:N relationships)
                field_mappings.insert(column.clone(), Some(FieldMapping {
                    excel_column: column.clone(),
                    target_field: "multi_select".to_string(),
                    field_type: FieldType::MultiSelect,
                    target_entity: None,
                    junction_entity: None, // Will be set when user selects from available options
                }));
            } else {
                field_mappings.insert(column.clone(), None);
            }
        }

        let available_target_fields = Self::discover_target_fields(env_config);

        let mut main_list_state = ListState::default();
        main_list_state.select(Some(0));

        Self {
            selected_excel_column: 0,
            excel_columns,
            field_mappings,
            main_list_state,
            show_field_selector: false,
            field_selector_state: ListState::default(),
            available_target_fields,
            show_junction_selector: false,
            junction_selector_state: ListState::default(),
            available_junction_entities: Vec::new(),
            current_mapping_in_progress: None,
        }
    }

    fn discover_target_fields(env_config: &EnvironmentConfig) -> Vec<TargetField> {
        let mut target_fields = Vec::new();

        // Main entity direct fields
        target_fields.push(TargetField {
            name: format!("{}_name", env_config.prefix),
            display_name: "Entity Name".to_string(),
            field_type: FieldType::DirectField,
            target_entity: None,
        });

        target_fields.push(TargetField {
            name: format!("{}_date", env_config.prefix),
            display_name: "Date".to_string(),
            field_type: FieldType::DirectField,
            target_entity: None,
        });

        target_fields.push(TargetField {
            name: format!("{}_info", env_config.prefix),
            display_name: "Information".to_string(),
            field_type: FieldType::DirectField,
            target_entity: None,
        });

        // Lookup fields from configured entities
        for (logical_type, mapping) in &env_config.entities {
            target_fields.push(TargetField {
                name: mapping.id_field.clone(),
                display_name: format!("{} (Lookup)", logical_type),
                field_type: FieldType::LookupField,
                target_entity: Some(mapping.entity.clone()),
            });
        }

        // Multi-select option
        target_fields.push(TargetField {
            name: "multi_select".to_string(),
            display_name: "Multiple Selection (N:N)".to_string(),
            field_type: FieldType::MultiSelect,
            target_entity: None,
        });

        // Ignore option
        target_fields.push(TargetField {
            name: "ignore".to_string(),
            display_name: "Ignore This Column".to_string(),
            field_type: FieldType::Ignore,
            target_entity: None,
        });

        target_fields
    }

    fn current_excel_column(&self) -> &str {
        self.excel_columns.get(self.selected_excel_column).map_or("", |v| v)
    }

    fn next_excel_column(&mut self) {
        if self.selected_excel_column < self.excel_columns.len() - 1 {
            self.selected_excel_column += 1;
            self.main_list_state.select(Some(self.selected_excel_column));
        }
    }

    fn prev_excel_column(&mut self) {
        if self.selected_excel_column > 0 {
            self.selected_excel_column -= 1;
            self.main_list_state.select(Some(self.selected_excel_column));
        }
    }


    fn get_available_junction_entities(_env_config: &EnvironmentConfig) -> Vec<String> {
        // TODO: This should be discovered from Dynamics metadata or user-configured
        // For now, return an empty list to indicate that junction entities need to be
        // properly discovered or configured
        vec![
            "[Manual Entry Required]".to_string(),
            "Enter junction entity name manually".to_string(),
        ]
    }

}

pub async fn run_field_mapping_tui(
    sheet_data: &SheetData,
    env_config: &EnvironmentConfig,
) -> Result<HashMap<String, FieldMapping>> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;

    let mut state = FieldMappingState::new(sheet_data.headers.clone(), env_config);

    loop {
        terminal.draw(|f| {
            render_field_mapping_screen(f, &mut state);
        })?;

        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => {
                        if state.show_junction_selector {
                            state.show_junction_selector = false;
                            state.current_mapping_in_progress = None;
                        } else if state.show_field_selector {
                            state.show_field_selector = false;
                        } else {
                            disable_raw_mode()?;
                            execute!(io::stdout(), LeaveAlternateScreen)?;
                            return Ok(HashMap::new());
                        }
                    }
                    KeyCode::Enter => {
                        if state.show_junction_selector {
                            if let Some(selected_idx) = state.junction_selector_state.selected() {
                                if let Some(junction_entity) = state.available_junction_entities.get(selected_idx) {
                                    if let Some(mut mapping) = state.current_mapping_in_progress.take() {
                                        mapping.junction_entity = Some(junction_entity.clone());
                                        let excel_column = state.current_excel_column().to_string();
                                        state.field_mappings.insert(excel_column, Some(mapping));
                                        state.show_junction_selector = false;
                                    }
                                }
                            }
                        } else if state.show_field_selector {
                            if let Some(selected_idx) = state.field_selector_state.selected() {
                                if let Some(target_field) = state.available_target_fields.get(selected_idx) {
                                    let excel_column = state.current_excel_column().to_string();

                                    let mapping = FieldMapping {
                                        excel_column: excel_column.clone(),
                                        target_field: target_field.name.clone(),
                                        field_type: target_field.field_type.clone(),
                                        target_entity: target_field.target_entity.clone(),
                                        junction_entity: None,
                                    };

                                    if matches!(target_field.field_type, FieldType::MultiSelect) {
                                        state.current_mapping_in_progress = Some(mapping);
                                        state.show_field_selector = false;
                                        state.show_junction_selector = true;
                                        state.junction_selector_state.select(Some(0));

                                        state.available_junction_entities = FieldMappingState::get_available_junction_entities(env_config);
                                    } else {
                                        state.field_mappings.insert(excel_column, Some(mapping));
                                        state.show_field_selector = false;
                                    }
                                }
                            }
                        } else {
                            state.show_field_selector = true;
                            state.field_selector_state.select(Some(0));
                        }
                    }
                    KeyCode::Up => {
                        if state.show_junction_selector {
                            let i = match state.junction_selector_state.selected() {
                                Some(i) => if i == 0 { state.available_junction_entities.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            state.junction_selector_state.select(Some(i));
                        } else if state.show_field_selector {
                            let i = match state.field_selector_state.selected() {
                                Some(i) => if i == 0 { state.available_target_fields.len() - 1 } else { i - 1 },
                                None => 0,
                            };
                            state.field_selector_state.select(Some(i));
                        } else {
                            state.prev_excel_column();
                        }
                    }
                    KeyCode::Down => {
                        if state.show_junction_selector {
                            let i = match state.junction_selector_state.selected() {
                                Some(i) => if i >= state.available_junction_entities.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            state.junction_selector_state.select(Some(i));
                        } else if state.show_field_selector {
                            let i = match state.field_selector_state.selected() {
                                Some(i) => if i >= state.available_target_fields.len() - 1 { 0 } else { i + 1 },
                                None => 0,
                            };
                            state.field_selector_state.select(Some(i));
                        } else {
                            state.next_excel_column();
                        }
                    }
                    KeyCode::Char('s') => {
                        let completed_mappings: HashMap<String, FieldMapping> = state.field_mappings
                            .into_iter()
                            .filter_map(|(k, v)| v.map(|mapping| (k, mapping)))
                            .collect();

                        disable_raw_mode()?;
                        execute!(io::stdout(), LeaveAlternateScreen)?;
                        return Ok(completed_mappings);
                    }
                    _ => {}
                }
            }
        }
    }
}


fn render_field_mapping_screen(f: &mut ratatui::Frame, state: &mut FieldMappingState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new("Field Mapping Configuration")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, chunks[0]);

    render_field_mapping_list(f, chunks[1], state);

    let instructions = Paragraph::new("↑↓: Navigate | Enter: Map Field | S: Save & Continue | Esc: Cancel")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Gray));
    f.render_widget(instructions, chunks[2]);

    if state.show_field_selector {
        render_field_selector_modal(f, state);
    }

    if state.show_junction_selector {
        render_junction_selector_modal(f, state);
    }
}

fn render_field_mapping_list(f: &mut ratatui::Frame, area: Rect, state: &mut FieldMappingState) {
    let items: Vec<ListItem> = state.excel_columns
        .iter()
        .map(|excel_column| {
            let mapping = state.field_mappings.get(excel_column).unwrap_or(&None);

            let (status, color) = match mapping {
                Some(mapping) => {
                    match &mapping.field_type {
                        FieldType::DirectField => (format!("→ {}", mapping.target_field), Color::Green),
                        FieldType::LookupField => (format!("→ {} (Lookup)", mapping.target_field), Color::Blue),
                        FieldType::MultiSelect => (format!("→ N:N via {}", mapping.junction_entity.as_ref().unwrap_or(&"?".to_string())), Color::Magenta),
                        FieldType::Ignore => ("→ [Ignored]".to_string(), Color::Gray),
                    }
                }
                None => ("→ [Not Mapped]".to_string(), Color::Red),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<20}", excel_column), Style::default().fg(color)),
                Span::styled(status, Style::default().fg(color)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().title("Excel Columns → Target Fields").borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, area, &mut state.main_list_state);
}

fn render_field_selector_modal(f: &mut ratatui::Frame, state: &mut FieldMappingState) {
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(format!("Select Target for '{}'", state.current_excel_column()))
        .borders(Borders::ALL);
    f.render_widget(block, area);

    let inner_area = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

    let items: Vec<ListItem> = state.available_target_fields
        .iter()
        .map(|field| {
            ListItem::new(Line::from(vec![
                Span::styled(&field.display_name, Style::default().fg(Color::White)),
                Span::styled(format!(" ({})", field.name), Style::default().fg(Color::Gray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, inner_area, &mut state.field_selector_state);
}

fn render_junction_selector_modal(f: &mut ratatui::Frame, state: &mut FieldMappingState) {
    let area = centered_rect(70, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title("Select Junction Entity for N:N Relationship")
        .borders(Borders::ALL);
    f.render_widget(block, area);

    let inner_area = area.inner(ratatui::layout::Margin { horizontal: 1, vertical: 1 });

    let items: Vec<ListItem> = state.available_junction_entities
        .iter()
        .map(|entity| {
            ListItem::new(Line::from(Span::styled(entity, Style::default().fg(Color::White))))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("► ");

    f.render_stateful_widget(list, inner_area, &mut state.junction_selector_state);
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