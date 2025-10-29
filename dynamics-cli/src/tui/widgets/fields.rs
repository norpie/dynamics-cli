use crate::tui::command::Command;
use super::{AutocompleteState, TextInputState, SelectState};
use super::events::{AutocompleteEvent, TextInputEvent, SelectEvent};

/// Field that combines value + state for Autocomplete widget
#[derive(Clone, Default)]
pub struct AutocompleteField {
    pub value: String,
    pub state: AutocompleteState,
}

impl AutocompleteField {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle autocomplete event and return command (usually None)
    /// Pass in the available options for filtering
    pub fn handle_event<Msg>(&mut self, event: AutocompleteEvent, options: &[String]) -> Command<Msg> {
        match event {
            AutocompleteEvent::Input(key) => {
                if let Some(new_value) = self.state.handle_input_key(key, &self.value, None) {
                    self.value = new_value;
                    self.state.update_filtered_options(&self.value, options);
                }
            }
            AutocompleteEvent::Navigate(key) => {
                use crossterm::event::KeyCode;

                // Handle Enter specially - select highlighted option
                if key == KeyCode::Enter {
                    if let Some(selected) = self.state.get_highlighted_option() {
                        self.value = selected;
                        self.state.close();
                        self.state.set_cursor_to_end(&self.value);
                    }
                } else {
                    self.state.handle_navigate_key(key);
                }
            }
            AutocompleteEvent::Select(selected) => {
                self.value = selected;
                self.state.close();
                self.state.set_cursor_to_end(&self.value);
            }
        }
        Command::None
    }

    /// Get current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set value (useful for initialization)
    /// Cursor is positioned at the end of the value
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.state.set_cursor_to_end(&self.value);
    }

    /// Check if dropdown is open
    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }
}

/// Field that combines value + state for TextInput widget
#[derive(Clone, Default)]
pub struct TextInputField {
    pub value: String,
    pub state: TextInputState,
}

impl TextInputField {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle text input event and return command (usually None unless Submit)
    /// Returns Some(value) on Submit, None otherwise
    pub fn handle_event(&mut self, event: TextInputEvent, max_length: Option<usize>) -> Option<String> {
        match event {
            TextInputEvent::Changed(key) => {
                if let Some(new_value) = self.state.handle_key(key, &self.value, max_length) {
                    self.value = new_value;
                }
                None
            }
            TextInputEvent::Submit => {
                Some(self.value.clone())
            }
        }
    }

    /// Get current value
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Set value (useful for initialization)
    /// Cursor is positioned at the end of the value
    pub fn set_value(&mut self, value: String) {
        self.value = value;
        self.state.set_cursor_to_end(&self.value);
    }
}

/// Field that combines value + state for Select widget
#[derive(Clone, Default)]
pub struct SelectField {
    selected_option: Option<String>,
    pub state: SelectState,
}

impl SelectField {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle select event and update selected value
    /// Returns a SelectEvent::Select if an item was selected (for app notification)
    pub fn handle_event<Msg>(&mut self, event: SelectEvent, options: &[String]) -> (Command<Msg>, Option<SelectEvent>) {
        use crossterm::event::KeyCode;

        let mut selection_made = None;

        match event {
            SelectEvent::Navigate(key) => {
                if !self.state.is_open() {
                    // Closed: Enter/Space toggles open
                    match key {
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            self.state.toggle();
                        }
                        _ => {}
                    }
                } else {
                    // Open: handle navigation
                    match key {
                        KeyCode::Up => self.state.navigate_prev(),
                        KeyCode::Down => self.state.navigate_next(),
                        KeyCode::Enter => {
                            // Select highlighted and close
                            self.state.select_highlighted();
                            let idx = self.state.selected();
                            if idx < options.len() {
                                self.selected_option = Some(options[idx].clone());
                                selection_made = Some(SelectEvent::Select(idx));
                            }
                        }
                        KeyCode::Esc => {
                            self.state.close();
                        }
                        _ => {}
                    }
                }
            }
            SelectEvent::Select(idx) => {
                self.state.select(idx);
                if idx < options.len() {
                    self.selected_option = Some(options[idx].clone());
                    selection_made = Some(SelectEvent::Select(idx));
                }
            }
            SelectEvent::Blur => {
                // Close dropdown when losing focus
                self.state.handle_blur();
            }
        }
        (Command::None, selection_made)
    }

    /// Get selected value as Option
    pub fn value(&self) -> Option<&str> {
        self.selected_option.as_deref()
    }

    /// Set selected value (useful for initialization)
    /// If value is None, also clears the visual state
    pub fn set_value(&mut self, value: Option<String>) {
        // Check if clearing before moving value
        let is_clearing = value.is_none();
        self.selected_option = value;
        // If clearing value, also clear the state
        if is_clearing {
            self.state.clear();
        }
    }

    /// Set selected value and update state index to match (requires options list)
    pub fn set_value_with_options(&mut self, value: Option<String>, options: &[String]) {
        self.selected_option = value.clone();

        // Update option count first (needed for select() to work)
        self.state.update_option_count(options.len());

        // Update state index to match
        if let Some(val) = value {
            if let Some(idx) = options.iter().position(|opt| opt == &val) {
                self.state.select(idx);
            }
        }
    }

    /// Check if dropdown is open
    pub fn is_open(&self) -> bool {
        self.state.is_open()
    }
}
