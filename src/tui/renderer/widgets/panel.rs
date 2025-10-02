use ratatui::{Frame, style::Style, widgets::{Block, Borders}, layout::Rect};
use crate::tui::{Element, Theme};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry};

/// Check if an element or its descendants contain a focusable with the given ID
pub fn element_contains_focus<Msg>(element: &Element<Msg>, focused_id: &FocusId) -> bool {
    match element {
        Element::Button { id, .. } | Element::List { id, .. } | Element::TextInput { id, .. } | Element::Tree { id, .. } | Element::Scrollable { id, .. } | Element::Select { id, .. } | Element::Autocomplete { id, .. } => id == focused_id,
        Element::Column { items, .. } | Element::Row { items, .. } => {
            items.iter().any(|(_, child)| element_contains_focus(child, focused_id))
        }
        Element::Container { child, .. } | Element::Panel { child, .. } => {
            element_contains_focus(child, focused_id)
        }
        Element::Stack { layers } => {
            layers.iter().any(|layer| element_contains_focus(&layer.element, focused_id))
        }
        _ => false,
    }
}

/// Render Panel element
pub fn render_panel<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    child: &Element<Msg>,
    title: &Option<String>,
    area: Rect,
    inside_panel: bool,
    render_fn: impl Fn(&mut Frame, &Theme, &mut InteractionRegistry<Msg>, &mut FocusRegistry<Msg>, &mut DropdownRegistry<Msg>, Option<&FocusId>, &Element<Msg>, Rect, bool),
) {
    // Check if the child (or any descendant) contains the focused element
    let child_has_focus = focused_id
        .map(|fid| element_contains_focus(child, fid))
        .unwrap_or(false);

    // Use focus color for panel border if child is focused
    let border_color = if child_has_focus {
        theme.lavender
    } else {
        theme.overlay0
    };

    // Render border block with background
    let block = if let Some(title_text) = title {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.base))
            .title(title_text.as_str())
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(theme.base))
    };

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Render child in the inner area, marking it as inside a panel
    render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, inner_area, true);
}
