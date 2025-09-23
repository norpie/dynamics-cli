use crate::commands::migration::ui::{
    components::{
        list_component::{ListAction, ListComponent},
        modal_component::{ModalContent, ModalContentAction},
    },
    screens::comparison::data_models::ExamplePair,
};
use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

#[derive(Debug, Clone)]
pub enum ExamplesAction {
    Delete(String),              // example id to delete
    SetActive(String),          // example id to set as active
    AddExample(String, String), // source_uuid, target_uuid
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum InputMode {
    Browse,       // Browsing/selecting examples
    InputSource,  // Entering source UUID
    InputTarget,  // Entering target UUID
}

pub struct ExamplesModal {
    examples: Vec<ExamplePair>,
    active_example_id: Option<String>,
    list: ListComponent<String>,
    current_action: Option<ExamplesAction>,

    // Input mode state
    input_mode: InputMode,
    source_uuid_input: String,
    target_uuid_input: String,
    input_error: Option<String>,
}

impl ExamplesModal {
    pub fn new(examples: Vec<ExamplePair>, active_example_id: Option<String>) -> Self {
        // Create display items for the list
        let mut items: Vec<String> = examples
            .iter()
            .enumerate()
            .map(|(i, example)| {
                let active_marker = if active_example_id.as_ref() == Some(&example.id) {
                    "● "
                } else {
                    "○ "
                };
                format!("{}{}", active_marker, example.display_name())
            })
            .collect();

        // Add "Add New Example" option at the end
        items.push("+ Add New Example".to_string());

        let mut list = ListComponent::new(items);
        list = list.with_title("Examples".to_string());

        Self {
            examples,
            active_example_id,
            list,
            current_action: None,
            input_mode: InputMode::Browse,
            source_uuid_input: String::new(),
            target_uuid_input: String::new(),
            input_error: None,
        }
    }

    fn get_help_text(&self) -> Vec<Line<'static>> {
        match self.input_mode {
            InputMode::Browse => vec![
                Line::from(vec![
                    Span::styled("Enter: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Set Active", Style::default().fg(Color::Cyan)),
                    Span::styled(" | ", Style::default()),
                    Span::styled("d: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Delete", Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("a: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Add Example", Style::default().fg(Color::Cyan)),
                    Span::styled(" | ", Style::default()),
                    Span::styled("Esc: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Close", Style::default().fg(Color::Cyan)),
                ]),
            ],
            InputMode::InputSource | InputMode::InputTarget => vec![
                Line::from(vec![
                    Span::styled("Tab: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Next Field", Style::default().fg(Color::Cyan)),
                    Span::styled(" | ", Style::default()),
                    Span::styled("Enter: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Add", Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("Esc: ", Style::default().fg(Color::Yellow)),
                    Span::styled("Cancel", Style::default().fg(Color::Cyan)),
                ]),
            ],
        }
    }
}

impl ModalContent for ExamplesModal {
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.input_mode {
            InputMode::Browse => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(3), Constraint::Length(3)].as_ref())
                    .split(area);

                // Render the list
                self.list.render(f, chunks[0]);

                // Render help text
                let help_paragraph = Paragraph::new(self.get_help_text())
                    .block(Block::default().borders(Borders::TOP))
                    .wrap(Wrap { trim: true });
                f.render_widget(help_paragraph, chunks[1]);
            }
            InputMode::InputSource | InputMode::InputTarget => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3), // Source UUID input
                        Constraint::Length(3), // Target UUID input
                        Constraint::Length(2), // Error message
                        Constraint::Length(3), // Help
                    ].as_ref())
                    .split(area);

                // Source UUID input
                let source_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Source UUID")
                    .border_style(if self.input_mode == InputMode::InputSource {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    });
                let source_paragraph = Paragraph::new(self.source_uuid_input.as_str())
                    .block(source_block);
                f.render_widget(source_paragraph, chunks[0]);

                // Target UUID input
                let target_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Target UUID")
                    .border_style(if self.input_mode == InputMode::InputTarget {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    });
                let target_paragraph = Paragraph::new(self.target_uuid_input.as_str())
                    .block(target_block);
                f.render_widget(target_paragraph, chunks[1]);

                // Error message
                if let Some(error) = &self.input_error {
                    let error_paragraph = Paragraph::new(error.as_str())
                        .style(Style::default().fg(Color::Red))
                        .wrap(Wrap { trim: true });
                    f.render_widget(error_paragraph, chunks[2]);
                }

                // Help text
                let help_paragraph = Paragraph::new(self.get_help_text())
                    .block(Block::default().borders(Borders::TOP))
                    .wrap(Wrap { trim: true });
                f.render_widget(help_paragraph, chunks[3]);

                // Show cursor in the active input field
                match self.input_mode {
                    InputMode::InputSource => {
                        f.set_cursor(
                            chunks[0].x + self.source_uuid_input.len() as u16 + 1,
                            chunks[0].y + 1,
                        );
                    }
                    InputMode::InputTarget => {
                        f.set_cursor(
                            chunks[1].x + self.target_uuid_input.len() as u16 + 1,
                            chunks[1].y + 1,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> ModalContentAction {
        match self.input_mode {
            InputMode::Browse => {
                match key {
                    KeyCode::Enter => {
                        if let Some(selected_index) = self.list.selected() {
                            if selected_index == self.examples.len() {
                                // Selected "Add New Example"
                                self.start_add_example();
                            } else {
                                // Selected an existing example - set it as active
                                self.set_active_selected();
                            }
                        }
                        ModalContentAction::None
                    }
                    KeyCode::Char('d') => {
                        self.delete_selected();
                        ModalContentAction::None
                    }
                    KeyCode::Char('a') => {
                        self.start_add_example();
                        ModalContentAction::None
                    }
                    KeyCode::Esc => ModalContentAction::Close,
                    _ => {
                        // Pass other keys to the list component
                        match self.list.handle_key(key) {
                            ListAction::None => ModalContentAction::None,
                            _ => ModalContentAction::None, // We handle selection ourselves
                        }
                    }
                }
            }
            InputMode::InputSource => {
                match key {
                    KeyCode::Char(c) => {
                        self.source_uuid_input.push(c);
                        self.input_error = None;
                        ModalContentAction::None
                    }
                    KeyCode::Backspace => {
                        self.source_uuid_input.pop();
                        self.input_error = None;
                        ModalContentAction::None
                    }
                    KeyCode::Tab => {
                        self.input_mode = InputMode::InputTarget;
                        ModalContentAction::None
                    }
                    KeyCode::Enter => {
                        if !self.target_uuid_input.is_empty() {
                            self.validate_and_add_example();
                        } else {
                            self.input_mode = InputMode::InputTarget;
                        }
                        ModalContentAction::None
                    }
                    KeyCode::Esc => {
                        self.cancel_input();
                        ModalContentAction::None
                    }
                    _ => ModalContentAction::None,
                }
            }
            InputMode::InputTarget => {
                match key {
                    KeyCode::Char(c) => {
                        self.target_uuid_input.push(c);
                        self.input_error = None;
                        ModalContentAction::None
                    }
                    KeyCode::Backspace => {
                        self.target_uuid_input.pop();
                        self.input_error = None;
                        ModalContentAction::None
                    }
                    KeyCode::Tab => {
                        self.input_mode = InputMode::InputSource;
                        ModalContentAction::None
                    }
                    KeyCode::Enter => {
                        self.validate_and_add_example();
                        ModalContentAction::None
                    }
                    KeyCode::Esc => {
                        self.cancel_input();
                        ModalContentAction::None
                    }
                    _ => ModalContentAction::None,
                }
            }
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> ModalContentAction {
        if self.input_mode == InputMode::Browse {
            match self.list.handle_mouse(event, area) {
                ListAction::Selected(_) => {
                    // Handle selection like Enter key
                    self.handle_key(KeyCode::Enter)
                }
                _ => ModalContentAction::None,
            }
        } else {
            ModalContentAction::None
        }
    }
}

impl ExamplesModal {
    /// Start adding a new example
    fn start_add_example(&mut self) {
        self.input_mode = InputMode::InputSource;
        self.source_uuid_input.clear();
        self.target_uuid_input.clear();
        self.input_error = None;
    }

    /// Cancel input mode and return to browse
    fn cancel_input(&mut self) {
        self.input_mode = InputMode::Browse;
        self.source_uuid_input.clear();
        self.target_uuid_input.clear();
        self.input_error = None;
    }

    /// Validate UUIDs and add example if valid
    fn validate_and_add_example(&mut self) {
        // Validate UUIDs
        if let Err(e) = self.validate_uuids() {
            self.input_error = Some(e);
            return;
        }

        // Set action for parent to handle
        self.current_action = Some(ExamplesAction::AddExample(
            self.source_uuid_input.clone(),
            self.target_uuid_input.clone(),
        ));

        self.cancel_input();
    }

    /// Validate that both UUIDs are valid format
    fn validate_uuids(&self) -> Result<(), String> {
        if self.source_uuid_input.trim().is_empty() {
            return Err("Source UUID is required".to_string());
        }

        if self.target_uuid_input.trim().is_empty() {
            return Err("Target UUID is required".to_string());
        }

        // Basic UUID format validation (36 chars with hyphens)
        if !self.is_valid_uuid_format(&self.source_uuid_input) {
            return Err("Invalid source UUID format".to_string());
        }

        if !self.is_valid_uuid_format(&self.target_uuid_input) {
            return Err("Invalid target UUID format".to_string());
        }

        Ok(())
    }

    /// Check if string looks like a UUID
    fn is_valid_uuid_format(&self, uuid_str: &str) -> bool {
        let trimmed = uuid_str.trim();
        // Basic check: 36 chars, hyphens in right places
        trimmed.len() == 36 &&
        trimmed.chars().nth(8) == Some('-') &&
        trimmed.chars().nth(13) == Some('-') &&
        trimmed.chars().nth(18) == Some('-') &&
        trimmed.chars().nth(23) == Some('-') &&
        trimmed.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    }

    /// Delete the currently selected example
    fn delete_selected(&mut self) {
        if let Some(selected) = self.list.selected() {
            if selected < self.examples.len() {
                let example_id = self.examples[selected].id.clone();

                // Set action for parent to handle
                self.current_action = Some(ExamplesAction::Delete(example_id));
            }
        }
    }

    /// Set the selected example as active
    fn set_active_selected(&mut self) {
        if let Some(selected) = self.list.selected() {
            if selected < self.examples.len() {
                let example_id = self.examples[selected].id.clone();

                // Set action for parent to handle
                self.current_action = Some(ExamplesAction::SetActive(example_id));
            }
        }
    }

    /// Refresh the list items
    fn refresh_list(&mut self) {
        let items: Vec<String> = self.examples
            .iter()
            .enumerate()
            .map(|(i, example)| {
                let active_marker = if self.active_example_id.as_ref() == Some(&example.id) {
                    "● "
                } else {
                    "○ "
                };
                format!(
                    "{}[{}] {} → {}",
                    active_marker,
                    i + 1,
                    &example.source_uuid[..8],  // First 8 chars of UUID
                    &example.target_uuid[..8]   // First 8 chars of UUID
                )
            })
            .collect();

        let mut all_items = items;
        all_items.push("+ Add New Example".to_string());

        self.list = ListComponent::new(all_items);
    }

    /// Get the currently selected examples
    pub fn get_examples(&self) -> &Vec<ExamplePair> {
        &self.examples
    }

    /// Get the active example ID
    pub fn get_active_example_id(&self) -> Option<&String> {
        self.active_example_id.as_ref()
    }

    /// Take any pending action from the modal
    pub fn take_action(&mut self) -> Option<ExamplesAction> {
        self.current_action.take()
    }
}