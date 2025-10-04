use ratatui::{Frame, style::{Style, Stylize}, widgets::Paragraph, layout::Rect};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::command::DispatchTarget;
use crate::tui::widgets::AutocompleteEvent;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, DropdownInfo, DropdownCallback, FocusableInfo};

/// Create on_key handler for autocomplete elements (old pattern)
pub fn autocomplete_on_key<Msg: Clone + Send + 'static>(
    is_open: bool,
    on_input: Option<fn(KeyCode) -> Msg>,
    on_navigate: Option<fn(KeyCode) -> Msg>,
) -> Box<dyn Fn(KeyCode) -> DispatchTarget<Msg> + Send> {
    Box::new(move |key| {
        if is_open {
            // Dropdown open: Up/Down/Enter/Esc go to navigate, others to input
            match key {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {
                    if let Some(f) = on_navigate {
                        DispatchTarget::AppMsg(f(key))
                    } else {
                        DispatchTarget::WidgetEvent(Box::new(AutocompleteEvent::Navigate(key)))
                    }
                }
                _ => {
                    // All other keys go to input for typing
                    if let Some(f) = on_input {
                        DispatchTarget::AppMsg(f(key))
                    } else {
                        DispatchTarget::WidgetEvent(Box::new(AutocompleteEvent::Input(key)))
                    }
                }
            }
        } else {
            // Dropdown closed: Escape passes through for unfocus, others go to input
            match key {
                KeyCode::Esc => DispatchTarget::PassThrough,
                _ => {
                    if let Some(f) = on_input {
                        DispatchTarget::AppMsg(f(key))
                    } else {
                        DispatchTarget::WidgetEvent(Box::new(AutocompleteEvent::Input(key)))
                    }
                }
            }
        }
    })
}

/// Create on_key handler for autocomplete elements (new event pattern)
pub fn autocomplete_on_key_event<Msg: Clone + Send + 'static>(
    is_open: bool,
    on_event: fn(AutocompleteEvent) -> Msg,
) -> Box<dyn Fn(KeyCode) -> DispatchTarget<Msg> + Send> {
    Box::new(move |key| {
        if is_open {
            // Dropdown open: Up/Down/Enter/Esc go to navigate, others to input
            match key {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {
                    DispatchTarget::AppMsg(on_event(AutocompleteEvent::Navigate(key)))
                }
                _ => {
                    // All other keys go to input for typing
                    DispatchTarget::AppMsg(on_event(AutocompleteEvent::Input(key)))
                }
            }
        } else {
            // Dropdown closed: Escape passes through for unfocus, others go to input
            match key {
                KeyCode::Esc => DispatchTarget::PassThrough,
                _ => DispatchTarget::AppMsg(on_event(AutocompleteEvent::Input(key))),
            }
        }
    })
}

/// Render Autocomplete element
pub fn render_autocomplete<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    all_options: &[String],
    current_input: &str,
    placeholder: &Option<String>,
    is_open: bool,
    filtered_options: &[String],
    highlight: usize,
    on_input: &Option<fn(KeyCode) -> Msg>,
    on_select: &Option<fn(String) -> Msg>,
    on_navigate: &Option<fn(KeyCode) -> Msg>,
    on_event: &Option<fn(AutocompleteEvent) -> Msg>,
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    area: Rect,
    inside_panel: bool,
) {
    // Register in focus registry - prefer on_event if available
    let on_key_handler = if let Some(event_fn) = on_event {
        autocomplete_on_key_event(is_open, *event_fn)
    } else {
        autocomplete_on_key(is_open, *on_input, *on_navigate)
    };

    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key: on_key_handler,
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    let is_focused = focused_id == Some(id);

    // Calculate visible width
    let visible_width = area.width.saturating_sub(2) as usize;

    // Build display text
    let display_text = if current_input.is_empty() && !is_focused {
        // Show placeholder
        if let Some(ph) = placeholder {
            format!(" {}", ph)
        } else {
            String::from(" ")
        }
    } else {
        // Show current input with cursor if focused
        if is_focused {
            // Simple cursor at end (no scroll support for now)
            let visible_text: String = current_input.chars().take(visible_width - 2).collect();
            format!(" {}│", visible_text)
        } else {
            let visible_text: String = current_input.chars().take(visible_width - 1).collect();
            format!(" {}", visible_text)
        }
    };

    // Determine text style
    let text_style = if current_input.is_empty() && !is_focused {
        Style::default().fg(theme.overlay1).italic()
    } else {
        Style::default().fg(theme.text)
    };

    // Render text without border (like TextInput/Select)
    let text_widget = Paragraph::new(display_text).style(text_style);
    frame.render_widget(text_widget, area);

    // If open, register dropdown for overlay rendering
    if is_open && !filtered_options.is_empty() {
        let callback = if let Some(event_fn) = on_event {
            DropdownCallback::AutocompleteEvent(Some(*event_fn))
        } else {
            DropdownCallback::Autocomplete(*on_select)
        };

        dropdown_registry.register(DropdownInfo {
            select_area: area,
            options: filtered_options.to_vec(),
            selected: None,  // No checkmark for autocomplete
            highlight,
            on_select: callback,
        });
    }
}
