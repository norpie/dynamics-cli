use crate::tui::command::Command;
use super::{AutocompleteState, TextInputState};
use super::events::{AutocompleteEvent, TextInputEvent};

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
    pub fn set_value(&mut self, value: String) {
        self.value = value;
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
    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }
}
