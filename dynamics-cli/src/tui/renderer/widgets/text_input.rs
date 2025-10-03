use ratatui::{Frame, style::{Style, Stylize}, widgets::Paragraph, layout::Rect};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, FocusableInfo};

/// Create on_key handler for text inputs (all keys pass to on_change, Enter also fires on_submit)
pub fn text_input_on_key<Msg: Clone + Send + 'static>(
    on_change: Option<fn(KeyCode) -> Msg>,
    on_submit: Option<Msg>,
) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| match key {
        KeyCode::Enter => {
            // Enter fires on_submit (app handles whether to also send on_change)
            on_submit.clone()
        }
        _ => {
            // All other keys go to on_change for app to handle via TextInputState
            on_change.map(|f| f(key))
        }
    })
}

/// Render TextInput element
pub fn render_text_input<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    value: &str,
    cursor_pos: usize,
    scroll_offset: usize,
    placeholder: &Option<String>,
    max_length: &Option<usize>,
    on_change: &Option<fn(KeyCode) -> Msg>,
    on_submit: &Option<Msg>,
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    area: Rect,
    inside_panel: bool,
) {
    // Register in focus registry
    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key: text_input_on_key(on_change.clone(), on_submit.clone()),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    // Check if this input is focused
    let is_focused = focused_id == Some(id);

    // Calculate visible width (area width - 2 for minimal padding)
    let visible_width = area.width.saturating_sub(2) as usize;

    // Get visible portion of text
    let chars: Vec<char> = value.chars().collect();
    let start_idx = scroll_offset;
    let end_idx = (start_idx + visible_width).min(chars.len());
    let visible_text: String = chars[start_idx..end_idx].iter().collect();

    // Calculate cursor position in visible area
    let cursor_in_visible = cursor_pos.saturating_sub(start_idx);

    // Build display text with cursor
    let display_text = if value.is_empty() && !is_focused {
        // Show placeholder
        if let Some(ph) = placeholder {
            format!(" {}", ph)  // Add left padding
        } else {
            String::from(" ")
        }
    } else {
        // Show actual text with cursor if focused
        if is_focused && cursor_in_visible <= visible_text.len() {
            let mut chars: Vec<char> = visible_text.chars().collect();
            chars.insert(cursor_in_visible, '│');
            let text: String = chars.into_iter().collect();
            format!(" {}", text)  // Add left padding
        } else {
            format!(" {}", visible_text)  // Add left padding
        }
    };

    // Determine text style
    let text_style = if value.is_empty() && !is_focused {
        // Placeholder style: italic, dim color
        Style::default().fg(theme.overlay1).italic()
    } else {
        Style::default().fg(theme.text)
    };

    // Render text without border
    let widget = Paragraph::new(display_text)
        .style(text_style);

    frame.render_widget(widget, area);
}
