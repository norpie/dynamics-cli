use ratatui::{Frame, style::Style, widgets::{Block, Clear}, layout::Rect};
use crate::tui::{Element, Theme, Layer, Alignment as LayerAlignment};
use crate::tui::element::FocusId;
use crate::tui::renderer::{InteractionRegistry, FocusRegistry, DropdownRegistry};

/// Render a semi-transparent dim overlay
pub fn render_dim_overlay(frame: &mut Frame, theme: &Theme, area: Rect) {
    // First, clear the area to prevent bleed-through
    frame.render_widget(Clear, area);

    // Then render the dim overlay
    let dim_block = Block::default()
        .style(Style::default().bg(theme.surface0));
    frame.render_widget(dim_block, area);
}

/// Calculate the position of a layer based on its alignment
pub fn calculate_layer_position<Msg>(
    element: &Element<Msg>,
    alignment: LayerAlignment,
    container: Rect,
    estimate_fn: impl Fn(&Element<Msg>, Rect) -> (u16, u16),
) -> Rect {
    // Estimate element size (for centered modal, use reasonable defaults)
    let (width, height) = estimate_fn(element, container);

    match alignment {
        LayerAlignment::TopLeft => Rect {
            x: container.x,
            y: container.y,
            width,
            height,
        },
        LayerAlignment::TopCenter => Rect {
            x: container.x + (container.width.saturating_sub(width)) / 2,
            y: container.y,
            width,
            height,
        },
        LayerAlignment::TopRight => Rect {
            x: container.x + container.width.saturating_sub(width),
            y: container.y,
            width,
            height,
        },
        LayerAlignment::Center => Rect {
            x: container.x + (container.width.saturating_sub(width)) / 2,
            y: container.y + (container.height.saturating_sub(height)) / 2,
            width,
            height,
        },
        LayerAlignment::BottomLeft => Rect {
            x: container.x,
            y: container.y + container.height.saturating_sub(height),
            width,
            height,
        },
        LayerAlignment::BottomCenter => Rect {
            x: container.x + (container.width.saturating_sub(width)) / 2,
            y: container.y + container.height.saturating_sub(height),
            width,
            height,
        },
        LayerAlignment::BottomRight => Rect {
            x: container.x + container.width.saturating_sub(width),
            y: container.y + container.height.saturating_sub(height),
            width,
            height,
        },
    }
}

/// Render Stack element
pub fn render_stack<Msg: Clone + Send + 'static>(
    frame: &mut Frame,
    theme: &Theme,
    registry: &mut InteractionRegistry<Msg>,
    focus_registry: &mut FocusRegistry<Msg>,
    dropdown_registry: &mut DropdownRegistry<Msg>,
    focused_id: Option<&FocusId>,
    layers: &[Layer<Msg>],
    area: Rect,
    inside_panel: bool,
    render_fn: impl Fn(&mut Frame, &Theme, &mut InteractionRegistry<Msg>, &mut FocusRegistry<Msg>, &mut DropdownRegistry<Msg>, Option<&FocusId>, &Element<Msg>, Rect, bool),
    estimate_fn: impl Fn(&Element<Msg>, Rect) -> (u16, u16),
) {
    // Render all layers visually
    for (layer_idx, layer) in layers.iter().enumerate() {
        // Render dim overlay if requested
        if layer.dim_below {
            render_dim_overlay(frame, theme, area);
        }

        // Calculate position based on alignment
        let layer_area = calculate_layer_position(&layer.element, layer.alignment, area, &estimate_fn);

        // Push focus layer context for this stack layer
        focus_registry.push_layer(layer_idx);

        // Render the layer element
        render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, &layer.element, layer_area, inside_panel);

        // Pop focus layer context
        focus_registry.pop_layer();
    }

    // Clear all interactions and focus, then re-render topmost layer to register only its interactions/focus
    registry.clear();
    focus_registry.clear();
    if let Some(last_layer) = layers.last() {
        let layer_idx = layers.len() - 1;
        let layer_area = calculate_layer_position(&last_layer.element, last_layer.alignment, area, &estimate_fn);

        // Re-push the topmost layer context
        focus_registry.push_layer(layer_idx);
        render_fn(frame, theme, registry, focus_registry, dropdown_registry, focused_id, &last_layer.element, layer_area, inside_panel);
        // Keep the layer pushed so focusables remain in active layer

        // Debug: Log focus registry state
        if let Some(layer) = focus_registry.active_layer() {
            log::debug!("Focus registry layer {} has {} focusables",
                       layer.layer_index, layer.focusables.len());
            for focusable in &layer.focusables {
                log::debug!("  Focusable: {:?} at {:?}", focusable.id, focusable.rect);
            }
        }
    }
}
