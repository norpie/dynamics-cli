use ratatui::{Frame, style::Style, widgets::{Block, Borders, Paragraph}, layout::Rect};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::widgets::SelectEvent;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, DropdownInfo, DropdownCallback, FocusableInfo};

/// Create on_key handler for select elements (dropdown navigation) - old pattern
pub fn select_on_key<Msg: Clone + Send + 'static>(
    is_open: bool,
    on_toggle: Option<Msg>,
    on_navigate: Option<fn(KeyCode) -> Msg>,
) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| {
        if !is_open {
            // Closed: Enter/Space toggles dropdown
            match key {
                KeyCode::Enter | KeyCode::Char(' ') => on_toggle.clone(),
                _ => None,
            }
        } else {
            // Open: Up/Down/Enter/Esc handled via on_navigate
            match key {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {
                    on_navigate.map(|f| f(key))
                }
                _ => None,
            }
        }
    })
}

/// Create on_key handler for select elements (new event pattern)
pub fn select_on_key_event<Msg: Clone + Send + 'static>(
    is_open: bool,
    on_event: fn(SelectEvent) -> Msg,
) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| {
        if !is_open {
            // Closed: Enter/Space toggles dropdown (but we don't have toggle in SelectEvent)
            // We'll handle this via Navigate event
            match key {
                KeyCode::Enter | KeyCode::Char(' ') => {
                    Some(on_event(SelectEvent::Navigate(key)))
                }
                _ => None,
            }
        } else {
            // Open: Up/Down/Enter/Esc handled via Navigate event
            match key {
                KeyCode::Up | KeyCode::Down | KeyCode::Enter | KeyCode::Esc => {
                    Some(on_event(SelectEvent::Navigate(key)))
                }
                _ => None,
            }
        }
    })
}

/// Render Select element
pub fn render_select<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    options: &[String],
    selected: usize,
    is_open: bool,
    highlight: usize,
    on_select: &Option<fn(usize) -> Msg>,
    on_toggle: &Option<Msg>,
    on_navigate: &Option<fn(KeyCode) -> Msg>,
    on_event: &Option<fn(SelectEvent) -> Msg>,
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    area: Rect,
    inside_panel: bool,
) {
    // Register in focus registry - prefer on_event if available
    let on_key_handler = if let Some(event_fn) = on_event {
        select_on_key_event(is_open, *event_fn)
    } else {
        select_on_key(is_open, on_toggle.clone(), on_navigate.clone())
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

    // Select should only use 3 lines (border + content + border), not the full allocated area
    let select_height = 3;
    let select_area = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: select_height.min(area.height),
    };

    // Determine border color
    let border_color = if is_focused && !inside_panel {
        theme.lavender
    } else {
        theme.overlay0
    };

    // Get selected option text
    let selected_text = if selected < options.len() {
        &options[selected]
    } else {
        ""
    };

    // Render closed state: Panel with selected value + arrow
    let arrow = if is_open { " ▲" } else { " ▼" };
    let display_text = format!("{}{}", selected_text, arrow);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.base));

    let inner_area = block.inner(select_area);
    frame.render_widget(block, select_area);

    // Render selected text
    let text_widget = Paragraph::new(display_text)
        .style(Style::default().fg(theme.text));
    frame.render_widget(text_widget, inner_area);

    // Register click handler for toggle
    if let Some(toggle_msg) = on_toggle {
        registry.register_click(select_area, toggle_msg.clone());
    }

    // If open, register dropdown for overlay rendering (rendered after main UI)
    if is_open && !options.is_empty() {
        let callback = if let Some(event_fn) = on_event {
            DropdownCallback::SelectEvent(Some(*event_fn))
        } else {
            DropdownCallback::Select(*on_select)
        };

        dropdown_registry.register(DropdownInfo {
            select_area,
            options: options.to_vec(),
            selected: Some(selected),
            highlight,
            on_select: callback,
        });
    }
}
