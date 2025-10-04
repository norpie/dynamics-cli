use ratatui::{Frame, style::Style, widgets::{Block, Borders}, layout::{Rect, Constraint, Direction, Layout}};
use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::{Element, Theme, LayoutConstraint};
use crate::tui::element::FocusId;
use crate::tui::command::DispatchTarget;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, FocusableInfo};

/// Create on_key handler for scrollable elements (scroll navigation)
pub fn scrollable_on_key<Msg: Clone + Send + 'static>(
    on_navigate: Option<fn(KeyCode) -> Msg>,
) -> Box<dyn Fn(KeyEvent) -> DispatchTarget<Msg> + Send> {
    Box::new(move |key_event| match key_event.code {
        // Scroll navigation keys
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
        | KeyCode::Home | KeyCode::End => {
            if let Some(f) = on_navigate {
                DispatchTarget::AppMsg(f(key_event.code))
            } else {
                // No callback - pass through to global subscriptions
                DispatchTarget::PassThrough
            }
        }
        _ => {
            // Unhandled key - pass through to global subscriptions
            DispatchTarget::PassThrough
        }
    })
}

/// Render Scrollable element
pub fn render_scrollable<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    child: &Element<Msg>,
    scroll_offset: usize,
    content_height: &Option<usize>,
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
        on_key: scrollable_on_key(on_navigate.clone()),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    // Check if this scrollable is focused
    let is_focused = focused_id == Some(id);

    // Calculate dimensions
    let viewport_height = area.height as usize;

    // Determine content height
    let detected_content_height = match child {
        Element::Column { items, spacing } => {
            // Account for spacing: N items need N + (N-1)*spacing lines
            items.len() + items.len().saturating_sub(1) * (*spacing as usize)
        }
        _ => content_height.unwrap_or(viewport_height),
    };
    let actual_content_height = content_height.unwrap_or(detected_content_height);

    // Reserve space for scrollbar if needed
    let needs_scrollbar = actual_content_height > viewport_height;
    let content_width = if needs_scrollbar {
        area.width.saturating_sub(1)
    } else {
        area.width
    };

    let content_area = Rect {
        x: area.x,
        y: area.y,
        width: content_width,
        height: area.height,
    };

    // Clamp scroll offset
    let max_scroll = actual_content_height.saturating_sub(viewport_height);
    let clamped_scroll = scroll_offset.min(max_scroll);

    // Render content based on type
    match child {
        Element::Column { items, spacing } => {
            // Virtual scrolling for Column - slice and iterate without cloning
            let start_idx = clamped_scroll;
            let end_idx = (start_idx + viewport_height).min(items.len());

            if start_idx < items.len() {
                let visible_items = &items[start_idx..end_idx];

                let constraints = visible_items
                    .iter()
                    .map(|(constraint, _)| match constraint {
                        LayoutConstraint::Length(n) => Constraint::Length(*n),
                        LayoutConstraint::Min(n) => Constraint::Min(*n),
                        LayoutConstraint::Fill(n) => Constraint::Ratio(*n as u32, visible_items.len() as u32),
                    })
                    .collect::<Vec<_>>();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .spacing(*spacing)
                    .split(content_area);

                // Render each visible item
                for ((_, item_child), chunk) in visible_items.iter().zip(chunks.iter()) {
                    render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, item_child, *chunk, inside_panel);
                }
            }
        }
        _ => {
            // For other element types, render normally (can't virtual scroll)
            render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, content_area, inside_panel);
        }
    }

    // Render scrollbar if needed
    if needs_scrollbar {
        let scrollbar_area = Rect {
            x: area.x + content_width,
            y: area.y,
            width: 1,
            height: area.height,
        };

        let scrollbar_position = if max_scroll > 0 {
            (clamped_scroll as f32 / max_scroll as f32 * (area.height - 1) as f32) as u16
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
    if is_focused && !inside_panel {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.lavender));
        frame.render_widget(border, area);
    }
}
