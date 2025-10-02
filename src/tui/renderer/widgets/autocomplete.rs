use ratatui::{Frame, style::{Style, Stylize}, widgets::{Block, Borders, Paragraph}, layout::Rect};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, DropdownInfo, DropdownCallback, FocusableInfo};

/// Create on_key handler for autocomplete elements
pub fn autocomplete_on_key<Msg: Clone + Send + 'static>(
    is_open: bool,
    on_input: Option<fn(KeyCode) -> Msg>,
    on_navigate: Option<fn(KeyCode) -> Msg>,
) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| {
        if is_open {
            // Dropdown open: Up/Down/Enter/Esc go to navigate, others to input
            match key {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {
                    on_navigate.map(|f| f(key))
                }
                _ => {
                    // All other keys go to input for typing
                    on_input.map(|f| f(key))
                }
            }
        } else {
            // Dropdown closed: all keys go to input
            on_input.map(|f| f(key))
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
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    area: Rect,
    inside_panel: bool,
) {
    // Register in focus registry
    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key: autocomplete_on_key(is_open, *on_input, *on_navigate),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    let is_focused = focused_id == Some(id);

    // Autocomplete uses 3 lines (border + content + border)
    let input_height = 3;
    let input_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: input_height.min(area.height),
    };

    // Determine border color
    let border_color = if is_focused && !inside_panel {
        theme.lavender
    } else {
        theme.overlay0
    };

    // Render input field
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.base));

    let inner_area = block.inner(input_area);
    frame.render_widget(block, input_area);

    // Calculate visible width
    let visible_width = inner_area.width.saturating_sub(2) as usize;

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

    let text_widget = Paragraph::new(display_text).style(text_style);
    frame.render_widget(text_widget, inner_area);

    // If open, register dropdown for overlay rendering
    if is_open && !filtered_options.is_empty() {
        dropdown_registry.register(DropdownInfo {
            select_area: input_area,
            options: filtered_options.to_vec(),
            selected: None,  // No checkmark for autocomplete
            highlight,
            on_select: DropdownCallback::Autocomplete(*on_select),
        });
    }
}
