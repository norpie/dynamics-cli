use ratatui::{Frame, style::Style, widgets::{Block, Borders, Paragraph}, layout::{Rect, Alignment}};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, FocusableInfo};

/// Create on_key handler for buttons (Enter or Space activates)
pub fn button_on_key<Msg: Clone + Send + 'static>(on_press: Option<Msg>) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| match key {
        KeyCode::Enter | KeyCode::Char(' ') => on_press.clone(),
        _ => None,
    })
}

/// Render Button element
pub fn render_button<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    label: &str,
    on_press: &Option<Msg>,
    on_hover: &Option<Msg>,
    on_hover_exit: &Option<Msg>,
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    style: &Option<Style>,
    hover_style: &Option<Style>,
    area: Rect,
    inside_panel: bool,
) {
    // Register in focus registry
    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key: button_on_key(on_press.clone()),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    // Register interaction handlers
    if let Some(msg) = on_press {
        registry.register_click(area, msg.clone());
    }
    if let Some(msg) = on_hover {
        registry.register_hover(area, msg.clone());
    }
    if let Some(msg) = on_hover_exit {
        registry.register_hover_exit(area, msg.clone());
    }

    // Check if this button is focused
    let is_focused = focused_id == Some(id);

    // Check if this button is hovered (auto-hover tracking)
    let is_hovered = registry.is_hovered(area);

    // Render button widget
    let default_style = Style::default().fg(theme.text);

    // Determine text style: hover_style > style > default
    let text_style = if is_hovered && hover_style.is_some() {
        hover_style.unwrap()
    } else {
        style.unwrap_or(default_style)
    };

    // Always show focus border on button (unlike other widgets, buttons need clear visual focus)
    let border_style = if is_focused {
        Style::default().fg(theme.lavender)
    } else {
        Style::default().fg(theme.overlay0)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let widget = Paragraph::new(label)
        .block(block)
        .alignment(Alignment::Center)
        .style(text_style);
    frame.render_widget(widget, area);
}
