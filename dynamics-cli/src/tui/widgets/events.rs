use crossterm::event::KeyCode;

/// Event type for Autocomplete widget
#[derive(Clone, Debug)]
pub enum AutocompleteEvent {
    /// Input character typed (includes backspace, char input, etc.)
    Input(KeyCode),
    /// Navigation in dropdown (Up/Down/Enter/Esc)
    Navigate(KeyCode),
    /// Item selected from dropdown (via click)
    Select(String),
}

/// Event type for TextInput widget
#[derive(Clone, Debug)]
pub enum TextInputEvent {
    /// Input changed (includes typing, backspace, etc.)
    Changed(KeyCode),
    /// Submit action (Enter key)
    Submit,
}

/// Event type for List widget
#[derive(Clone, Debug)]
pub enum ListEvent {
    /// Navigation keys (Up/Down/PageUp/PageDown/Home/End)
    Navigate(KeyCode),
    /// Item selected (Enter or click)
    Select,
}

/// Event type for Tree widget
#[derive(Clone, Debug)]
pub enum TreeEvent {
    /// Navigation keys (Up/Down/Left/Right)
    Navigate(KeyCode),
    /// Toggle node expansion (Enter key)
    Toggle,
}

/// Event type for Select widget
#[derive(Clone, Debug)]
pub enum SelectEvent {
    /// Navigation in dropdown (Up/Down)
    Navigate(KeyCode),
    /// Option selected (Enter or click)
    Select(usize),
}
