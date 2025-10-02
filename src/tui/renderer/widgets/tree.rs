use ratatui::{Frame, style::Style, widgets::{Block, Borders}, layout::{Rect, Constraint, Direction, Layout}};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme, LayoutConstraint};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry, FocusableInfo};

/// Create on_key handler for trees (navigation and toggle)
pub fn tree_on_key<Msg: Clone + Send + 'static>(
    selected: Option<String>,
    on_navigate: Option<fn(KeyCode) -> Msg>,
    on_toggle: Option<fn(String) -> Msg>,
) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
    Box::new(move |key| match key {
        // Navigation keys - handled by on_navigate callback
        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
        | KeyCode::Home | KeyCode::End => {
            on_navigate.map(|f| f(key))
        }
        // Left/Right for expand/collapse - also handled by on_navigate
        KeyCode::Left | KeyCode::Right => {
            on_navigate.map(|f| f(key))
        }
        // Enter toggles expansion
        KeyCode::Enter => {
            if let (Some(id), Some(toggle)) = (selected.as_ref(), on_toggle) {
                Some(toggle(id.clone()))
            } else {
                None
            }
        }
        _ => None,
    })
}

/// Render Tree element
pub fn render_tree<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    id: &FocusId,
    items: &[Element<Msg>],
    node_ids: &[String],
    selected: &Option<String>,
    scroll_offset: usize,
    on_select: &Option<fn(String) -> Msg>,
    on_toggle: &Option<fn(String) -> Msg>,
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
        on_key: tree_on_key(selected.clone(), on_navigate.clone(), on_toggle.clone()),
        on_focus: on_focus.clone(),
        on_blur: on_blur.clone(),
        inside_panel,
    });

    // Check if this tree is focused
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

        // Register click handlers for nodes
        if let Some(on_select_fn) = on_select {
            for (idx, chunk) in chunks.iter().enumerate() {
                let node_idx = start_idx + idx;
                if node_idx < node_ids.len() {
                    let node_id = node_ids[node_idx].clone();
                    registry.register_click(*chunk, on_select_fn(node_id));
                }
            }
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
    if is_focused && !inside_panel {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.lavender));
        frame.render_widget(border, area);
    }
}
