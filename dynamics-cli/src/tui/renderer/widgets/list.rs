use ratatui::{Frame, style::Style, widgets::{Block, Borders}, layout::{Rect, Constraint, Direction, Layout}};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme, LayoutConstraint};
use crate::tui::element::FocusId;
use crate::tui::command::DispatchTarget;
use crate::tui::widgets::ListEvent;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, FocusableInfo};

/// Create on_key handler for lists (navigation and activation)
pub fn list_on_key<Msg: Clone + Send + 'static>(
    selected: Option<usize>,
    on_navigate: Option<fn(KeyCode) -> Msg>,
    on_activate: Option<fn(usize) -> Msg>,
) -> Box<dyn Fn(KeyCode) -> DispatchTarget<Msg> + Send> {
    Box::new(move |key| match key {
        // Navigation keys - handled by on_navigate callback
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
        | KeyCode::Home | KeyCode::End => {
            if let Some(f) = on_navigate {
                DispatchTarget::AppMsg(f(key))
            } else {
                DispatchTarget::WidgetEvent(Box::new(ListEvent::Navigate(key)))
            }
        }
        // Enter activates selected item
        KeyCode::Enter => {
            if let (Some(idx), Some(activate)) = (selected, on_activate) {
                DispatchTarget::AppMsg(activate(idx))
            } else {
                DispatchTarget::WidgetEvent(Box::new(ListEvent::Select))
            }
        }
        _ => {
            // Unhandled key - pass through to global subscriptions without blurring
            DispatchTarget::PassThrough
        }
    })
}

/// Render List element
pub fn render_list<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    items: &[Element<Msg>],
    selected: Option<usize>,
    scroll_offset: usize,
    on_select: &Option<fn(usize) -> Msg>,
    on_activate: &Option<fn(usize) -> Msg>,
    on_navigate: &Option<fn(KeyCode) -> Msg>,
    on_focus: &Option<Msg>,
    on_blur: &Option<Msg>,
    area: Rect,
    inside_panel: bool,
    render_fn: impl Fn(&mut Frame, &Theme, &mut InteractionRegistry<Msg>, &mut FocusRegistry<Msg>, &mut DropdownRegistry<Msg>, Option<&FocusId>, &Element<Msg>, Rect, bool),
) {
    // Register in focus registry
    focus_registry.register_focusable(FocusableInfo {
        id: id.clone(),
        rect: area,
        on_key: list_on_key(selected, on_navigate.clone(), on_activate.clone()),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    // Check if this list is focused
    let is_focused = focused_id == Some(id);

    // Calculate visible height
    let visible_height = area.height as usize;

    // Virtual scrolling: only render visible items
    let start_idx = scroll_offset;
    let end_idx = (start_idx + visible_height).min(items.len());

    // Create layout for visible items
    let visible_items: Vec<_> = items[start_idx..end_idx]
        .iter()
        .map(|item| (LayoutConstraint::Length(1), item.clone()))
        .collect();

    if !visible_items.is_empty() {
        let constraints = visible_items
            .iter()
            .map(|(constraint, _)| match constraint {
                LayoutConstraint::Length(n) => Constraint::Length(*n),
                _ => Constraint::Length(1),
            })
            .collect::<Vec<_>>();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Render each visible item
        for ((_, child), chunk) in visible_items.iter().zip(chunks.iter()) {
            render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *chunk, inside_panel);
        }
    }

    // Render scrollbar if needed
    if items.len() > visible_height {
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y,
            width: 1,
            height: area.height,
        };

        let scrollbar_position = if items.len() > visible_height {
            (scroll_offset as f32 / (items.len() - visible_height) as f32 * (area.height - 1) as f32) as u16
        } else {
            0
        };

        // Render scrollbar thumb
        if scrollbar_position < area.height {
            let thumb_area = Rect {
                x: scrollbar_area.x,
                y: scrollbar_area.y + scrollbar_position,
                width: 1,
                height: 1,
            };
            let thumb = Block::default().style(Style::default().fg(theme.overlay1));
            frame.render_widget(thumb, thumb_area);
        }
    }

    // Only render focus border if NOT inside a panel
    // (panels will show focus on their border instead)
    if is_focused && !inside_panel {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.lavender));
        frame.render_widget(border, area);
    }
}
