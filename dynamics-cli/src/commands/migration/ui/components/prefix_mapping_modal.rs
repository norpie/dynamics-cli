use crate::commands::migration::ui::{
    components::modal_component::{ModalContent, ModalContentAction},
    styles::STYLES,
};
use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum PrefixMappingAction {
    Add {
        source_prefix: String,
        target_prefix: String,
    },
    Delete(String),
    Close,
}

#[derive(Debug)]
enum ModalState {
    List,
    AddingSourcePrefix,
    AddingTargetPrefix { source_prefix: String },
}

pub struct PrefixMappingModal {
    mappings: HashMap<String, String>,
    selected_index: usize,
    list_state: ListState,
    state: ModalState,
    input_buffer: String,
    action_result: Option<PrefixMappingAction>,
}

impl PrefixMappingModal {
    pub fn new(mappings: HashMap<String, String>) -> Self {
        let mut modal = Self {
            mappings,
            selected_index: 0,
            list_state: ListState::default(),
            state: ModalState::List,
            input_buffer: String::new(),
            action_result: None,
        };

        if !modal.mappings.is_empty() {
            modal.list_state.select(Some(0));
        }

        modal
    }

    pub fn take_action(&mut self) -> Option<PrefixMappingAction> {
        self.action_result.take()
    }

    fn get_mapping_list(&self) -> Vec<(String, String)> {
        let mut mappings: Vec<_> = self
            .mappings
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        mappings.sort_by(|a, b| a.0.cmp(&b.0));
        mappings
    }

    fn handle_list_key(&mut self, key: KeyCode) -> ModalContentAction {
        match key {
            KeyCode::Esc => ModalContentAction::Close,
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.mappings.is_empty() {
                    let len = self.mappings.len();
                    self.selected_index = if self.selected_index == 0 {
                        len - 1
                    } else {
                        self.selected_index - 1
                    };
                    self.list_state.select(Some(self.selected_index));
                }
                ModalContentAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.mappings.is_empty() {
                    let len = self.mappings.len();
                    self.selected_index = (self.selected_index + 1) % len;
                    self.list_state.select(Some(self.selected_index));
                }
                ModalContentAction::None
            }
            KeyCode::Char('n') => {
                self.state = ModalState::AddingSourcePrefix;
                self.input_buffer.clear();
                ModalContentAction::None
            }
            KeyCode::Char('d') => {
                if !self.mappings.is_empty() {
                    let mappings_list = self.get_mapping_list();
                    if let Some((source_prefix, _)) = mappings_list.get(self.selected_index) {
                        self.action_result =
                            Some(PrefixMappingAction::Delete(source_prefix.clone()));
                        ModalContentAction::Close
                    } else {
                        ModalContentAction::None
                    }
                } else {
                    ModalContentAction::None
                }
            }
            _ => ModalContentAction::None,
        }
    }

    fn handle_input_key(&mut self, key: KeyCode) -> ModalContentAction {
        match key {
            KeyCode::Esc => {
                self.state = ModalState::List;
                self.input_buffer.clear();
                ModalContentAction::None
            }
            KeyCode::Enter => match &self.state {
                ModalState::AddingSourcePrefix => {
                    if !self.input_buffer.is_empty() {
                        let source_prefix = self.input_buffer.clone();
                        self.state = ModalState::AddingTargetPrefix { source_prefix };
                        self.input_buffer.clear();
                    }
                    ModalContentAction::None
                }
                ModalState::AddingTargetPrefix { source_prefix } => {
                    if !self.input_buffer.is_empty() {
                        self.action_result = Some(PrefixMappingAction::Add {
                            source_prefix: source_prefix.clone(),
                            target_prefix: self.input_buffer.clone(),
                        });
                        ModalContentAction::Close
                    } else {
                        ModalContentAction::None
                    }
                }
                _ => ModalContentAction::None,
            },
            KeyCode::Backspace => {
                self.input_buffer.pop();
                ModalContentAction::None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                ModalContentAction::None
            }
            _ => ModalContentAction::None,
        }
    }
}

impl ModalContent for PrefixMappingModal {
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Instructions
                Constraint::Min(5),    // List or input
                Constraint::Length(2), // Status/input prompt
            ])
            .split(area);

        // Instructions
        let instructions = match self.state {
            ModalState::List => {
                "Keys: [n] New mapping, [d] Delete selected, [↑↓/jk] Navigate, [Esc] Close"
            }
            ModalState::AddingSourcePrefix => {
                "Enter source prefix (e.g., 'cgk_'): [Enter] Continue, [Esc] Cancel"
            }
            ModalState::AddingTargetPrefix { .. } => {
                "Enter target prefix (e.g., 'nrq_'): [Enter] Save, [Esc] Cancel"
            }
        };

        let instruction_paragraph = Paragraph::new(instructions)
            .block(Block::default().borders(Borders::ALL).title("Instructions"))
            .style(STYLES.info);
        f.render_widget(instruction_paragraph, chunks[0]);

        match &self.state {
            ModalState::List => {
                // Render mapping list
                let mappings_list = self.get_mapping_list();

                if mappings_list.is_empty() {
                    let empty_paragraph =
                        Paragraph::new("No prefix mappings defined. Press 'n' to add one.")
                            .block(
                                Block::default()
                                    .borders(Borders::ALL)
                                    .title("Prefix Mappings"),
                            )
                            .style(STYLES.normal);
                    f.render_widget(empty_paragraph, chunks[1]);
                } else {
                    let items: Vec<ListItem> = mappings_list
                        .iter()
                        .map(|(source, target)| {
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    format!("{:<15}", source),
                                    Style::default().fg(Color::Cyan),
                                ),
                                Span::raw(" → "),
                                Span::styled(target.clone(), Style::default().fg(Color::Green)),
                            ]))
                        })
                        .collect();

                    let list = List::new(items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("Prefix Mappings"),
                        )
                        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                        .highlight_symbol("► ");

                    f.render_stateful_widget(list, chunks[1], &mut self.list_state);
                }

                // Status
                let status = if mappings_list.is_empty() {
                    "No mappings"
                } else {
                    &format!("{} mapping(s)", mappings_list.len())
                };
                let status_paragraph = Paragraph::new(status).style(STYLES.normal);
                f.render_widget(status_paragraph, chunks[2]);
            }
            ModalState::AddingSourcePrefix => {
                // Input for source prefix
                let input_paragraph = Paragraph::new(self.input_buffer.as_str())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Source Prefix"),
                    )
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input_paragraph, chunks[1]);

                let prompt_paragraph =
                    Paragraph::new("Type the source prefix to map from").style(STYLES.info);
                f.render_widget(prompt_paragraph, chunks[2]);
            }
            ModalState::AddingTargetPrefix { source_prefix } => {
                // Input for target prefix
                let input_paragraph = Paragraph::new(self.input_buffer.as_str())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Target Prefix"),
                    )
                    .style(Style::default().fg(Color::Yellow));
                f.render_widget(input_paragraph, chunks[1]);

                let prompt_paragraph =
                    Paragraph::new(format!("Mapping '{}' to:", source_prefix)).style(STYLES.info);
                f.render_widget(prompt_paragraph, chunks[2]);
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> ModalContentAction {
        match self.state {
            ModalState::List => self.handle_list_key(key),
            ModalState::AddingSourcePrefix | ModalState::AddingTargetPrefix { .. } => {
                self.handle_input_key(key)
            }
        }
    }

    fn handle_mouse(&mut self, _event: MouseEvent, _area: Rect) -> ModalContentAction {
        ModalContentAction::None
    }

    fn get_title(&self) -> Option<String> {
        Some("Prefix Mappings".to_string())
    }
}
