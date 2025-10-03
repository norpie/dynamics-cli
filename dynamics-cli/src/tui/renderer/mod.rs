use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Clear},
    style::{Style, Stylize},
};
use crossterm::event::KeyCode;
use crate::tui::{Element, Theme, LayoutConstraint, Layer, Alignment as LayerAlignment};
use crate::tui::element::FocusId;

// Re-export registries
mod interaction_registry;
mod focus_registry;
mod dropdown_registry;
mod widgets;

pub use interaction_registry::InteractionRegistry;
pub use focus_registry::{FocusRegistry, FocusableInfo, LayerFocusContext};
pub use dropdown_registry::{DropdownRegistry, DropdownInfo, DropdownCallback};

use widgets::*;

/// Renders elements to the terminal
pub struct Renderer;

impl Renderer {
    pub fn render<Msg: Clone + Send + 'static>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        focus_registry: &mut FocusRegistry<Msg>,
        dropdown_registry: &mut DropdownRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
    ) {
        Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, element, area, false);
        // After rendering main UI, render all dropdowns as overlays
        Self::render_dropdowns(frame, theme, registry, dropdown_registry);
    }

    fn render_element<Msg: Clone + Send + 'static>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        focus_registry: &mut FocusRegistry<Msg>,
        dropdown_registry: &mut DropdownRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
        inside_panel: bool,
    ) {
        // Handle primitives (None, Text, StyledText)
        if primitives::is_primitive(element) {
            primitives::render_primitive(frame, theme, element, area);
            return;
        }

        match element {

            Element::Button {
                id,
                label,
                on_press,
                on_hover,
                on_hover_exit,
                on_focus,
                on_blur,
                style,
            } => {
                render_button(frame, theme, registry, focus_registry, focused_id, id, label, on_press, on_hover, on_hover_exit, on_focus, on_blur, style, area, inside_panel);
            }

            Element::Column { items, spacing } => {
                layout::render_column(frame, theme, registry, focus_registry, dropdown_registry, focused_id, items, *spacing, area, inside_panel, Self::render_element);
            }

            Element::Row { items, spacing } => {
                layout::render_row(frame, theme, registry, focus_registry, dropdown_registry, focused_id, items, *spacing, area, inside_panel, Self::render_element);
            }

            Element::Container { child, padding } => {
                layout::render_container(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *padding, area, inside_panel, Self::render_element);
            }

            Element::Panel { child, title, .. } => {
                render_panel(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, title, area, inside_panel, Self::render_element);
            }

            Element::List {
                id,
                items,
                selected,
                scroll_offset,
                on_select,
                on_activate,
                on_navigate,
                on_focus,
                on_blur,
            } => {
                render_list(frame, theme, registry, focus_registry, dropdown_registry, focused_id, id, items, *selected, *scroll_offset, on_select, on_activate, on_navigate, on_focus, on_blur, area, inside_panel, Self::render_element);
            }

            Element::TextInput {
                id,
                value,
                cursor_pos,
                scroll_offset,
                placeholder,
                max_length,
                on_change,
                on_submit,
                on_focus,
                on_blur,
            } => {
                render_text_input(frame, theme, registry, focus_registry, focused_id, id, value, *cursor_pos, *scroll_offset, placeholder, max_length, on_change, on_submit, on_focus, on_blur, area, inside_panel);
            }

            Element::Tree {
                id,
                items,
                node_ids,
                selected,
                scroll_offset,
                on_select,
                on_toggle,
                on_navigate,
                on_event,
                on_focus,
                on_blur,
            } => {
                render_tree(frame, theme, registry, focus_registry, dropdown_registry, focused_id, id, items, node_ids, selected, *scroll_offset, on_select, on_toggle, on_navigate, on_event, on_focus, on_blur, area, inside_panel, Self::render_element);
            }

            Element::Scrollable {
                id,
                child,
                scroll_offset,
                content_height,
                on_scroll,
                on_focus,
                on_blur,
            } => {
                render_scrollable(frame, theme, registry, focus_registry, dropdown_registry, focused_id, id, child, *scroll_offset, content_height, on_scroll, on_focus, on_blur, area, inside_panel, Self::render_element);
            }

            Element::Select {
                id,
                options,
                selected,
                is_open,
                highlight,
                on_select,
                on_toggle,
                on_navigate,
                on_event,
                on_focus,
                on_blur,
            } => {
                render_select(frame, theme, registry, focus_registry, dropdown_registry, focused_id, id, options, *selected, *is_open, *highlight, on_select, on_toggle, on_navigate, on_event, on_focus, on_blur, area, inside_panel);
            }

            Element::Autocomplete {
                id,
                all_options: _,
                current_input,
                placeholder,
                is_open,
                filtered_options,
                highlight,
                on_input,
                on_select,
                on_navigate,
                on_event,
                on_focus,
                on_blur,
            } => {
                render_autocomplete(frame, theme, registry, focus_registry, dropdown_registry, focused_id, id, &[], current_input, placeholder, *is_open, filtered_options, *highlight, on_input, on_select, on_navigate, on_event, on_focus, on_blur, area, inside_panel);
            }

            Element::Stack { layers } => {
                render_stack(frame, theme, registry, focus_registry, dropdown_registry, focused_id, layers, area, inside_panel, Self::render_element, Self::estimate_element_size);
            }

            // Primitives are handled at the top of the function
            Element::None | Element::Text { .. } | Element::StyledText { .. } => {
                unreachable!("Primitives should be handled before the match statement")
            }
        }
    }

    /// Calculate minimum content size needed for an element (recursive)
    fn calculate_content_size<Msg>(element: &Element<Msg>, max_width: u16, max_height: u16) -> (u16, u16) {
        match element {
            Element::None => (0, 0),
            Element::Text { content, .. } => {
                let width = (content.len() as u16).min(max_width);
                (width, 1)
            }
            Element::StyledText { line, .. } => {
                let width = (line.width() as u16).min(max_width);
                (width, 1)
            }
            Element::Button { label, .. } => {
                let width = (label.len() as u16 + 4).min(max_width);
                (width, 3)
            }
            Element::Column { items, spacing } => {
                let mut total_height = 0u16;
                let mut max_item_width = 0u16;

                for (constraint, child) in items {
                    let (child_w, child_h) = Self::calculate_content_size(child, max_width, max_height);
                    max_item_width = max_item_width.max(child_w);

                    match constraint {
                        LayoutConstraint::Length(h) => total_height += h,
                        LayoutConstraint::Min(h) => total_height += (*h).max(child_h),
                        LayoutConstraint::Fill(_) => total_height += child_h,
                    }
                }

                // Add spacing between items
                if items.len() > 1 {
                    total_height += (items.len() as u16 - 1) * spacing;
                }

                (max_item_width.min(max_width), total_height.min(max_height))
            }
            Element::Row { items, spacing } => {
                let mut total_width = 0u16;
                let mut max_item_height = 0u16;

                for (constraint, child) in items {
                    let (child_w, child_h) = Self::calculate_content_size(child, max_width, max_height);
                    max_item_height = max_item_height.max(child_h);

                    match constraint {
                        LayoutConstraint::Length(w) => total_width += w,
                        LayoutConstraint::Min(w) => total_width += (*w).max(child_w),
                        LayoutConstraint::Fill(_) => total_width += child_w,
                    }
                }

                // Add spacing between items
                if items.len() > 1 {
                    total_width += (items.len() as u16 - 1) * spacing;
                }

                (total_width.min(max_width), max_item_height.min(max_height))
            }
            Element::Container { child, padding } => {
                let (child_w, child_h) = Self::calculate_content_size(child, max_width.saturating_sub(padding * 2), max_height.saturating_sub(padding * 2));
                (
                    (child_w + padding * 2).min(max_width),
                    (child_h + padding * 2).min(max_height)
                )
            }
            Element::Panel { child, .. } => {
                // Panel adds 2 for borders (1 top + 1 bottom, 1 left + 1 right)
                let (child_w, child_h) = Self::calculate_content_size(child, max_width.saturating_sub(2), max_height.saturating_sub(2));
                (
                    (child_w + 2).min(max_width),
                    (child_h + 2).min(max_height)
                )
            }
            Element::List { items, .. } => {
                // List height is number of items, width is max item width
                let height = (items.len() as u16).min(max_height);
                // We can't easily calculate width of list items without rendering, so use reasonable default
                (max_width.min(40), height)
            }
            Element::TextInput { .. } => (max_width.min(40), 1),
            Element::Tree { items, .. } => {
                let height = (items.len() as u16).min(max_height);
                (max_width.min(40), height)
            }
            Element::Scrollable { child, .. } => {
                Self::calculate_content_size(child, max_width, max_height)
            }
            Element::Select { .. } => (max_width.min(30), 3),
            Element::Autocomplete { .. } => (max_width.min(40), 3),
            Element::Stack { layers } => {
                // Stack size is the max of all layers
                let mut max_w = 0u16;
                let mut max_h = 0u16;
                for layer in layers {
                    let (w, h) = Self::calculate_content_size(&layer.element, max_width, max_height);
                    max_w = max_w.max(w);
                    max_h = max_h.max(h);
                }
                (max_w.min(max_width), max_h.min(max_height))
            }
        }
    }

    /// Estimate the size an element needs
    fn estimate_element_size<Msg>(element: &Element<Msg>, container: Rect) -> (u16, u16) {
        match element {
            Element::None => (0, 0),
            Element::Text { content, .. } => (content.len() as u16, 1),
            Element::StyledText { line, .. } => (line.width() as u16, 1),
            Element::Button { label, .. } => (label.len() as u16 + 4, 3),
            Element::Panel { child, width, height, .. } => {
                // Use explicit size if provided
                match (width, height) {
                    (Some(w), Some(h)) => (*w, *h),
                    (Some(w), None) => {
                        // Width specified, calculate height from content
                        let (_, content_h) = Self::calculate_content_size(child, container.width, container.height);
                        (*w, content_h.min(container.height))
                    }
                    (None, Some(h)) => {
                        // Height specified, calculate width from content
                        let (content_w, _) = Self::calculate_content_size(child, container.width, container.height);
                        (content_w.min(container.width), *h)
                    }
                    (None, None) => {
                        // Calculate from content with reasonable max bounds
                        let max_width = container.width.min(100);  // Max 100 columns
                        let max_height = container.height.min(40); // Max 40 lines
                        let (content_w, content_h) = Self::calculate_content_size(child, max_width, max_height);
                        (content_w.max(30).min(container.width), content_h.max(10).min(container.height))
                    }
                }
            }
            Element::Container { .. } => {
                // For containers (modals), use a reasonable default
                let width = container.width.min(60);
                let height = container.height.min(15);
                (width, height)
            }
            Element::Column { .. } | Element::Row { .. } | Element::Stack { .. } | Element::List { .. } => {
                // Layout elements should fill the full container
                (container.width, container.height)
            }
            Element::TextInput { .. } => {
                // Text input: fixed height (1 line), full width
                (container.width, 1)
            }
            Element::Autocomplete { .. } => {
                // Autocomplete: fixed height (3 lines including borders), full width
                (container.width, 3)
            }
            _ => {
                // Default: 50% of container
                (container.width / 2, container.height / 2)
            }
        }
    }

    /// Render all registered dropdowns as overlays (called after main UI rendering)
    fn render_dropdowns<Msg: Clone>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        dropdown_registry: &DropdownRegistry<Msg>,
    ) {
        for dropdown in dropdown_registry.dropdowns() {
            // Calculate dropdown position (below the select, or above if no room)
            let dropdown_height = (dropdown.options.len() as u16).min(10) + 2; // +2 for borders
            let dropdown_y = if dropdown.select_area.y + dropdown.select_area.height + dropdown_height <= frame.size().height {
                // Render below
                dropdown.select_area.y + dropdown.select_area.height
            } else {
                // Render above
                dropdown.select_area.y.saturating_sub(dropdown_height)
            };

            let dropdown_area = Rect {
                x: dropdown.select_area.x,
                y: dropdown_y,
                width: dropdown.select_area.width,
                height: dropdown_height,
            };

            // First, clear the area to remove any bleed-through
            frame.render_widget(Clear, dropdown_area);

            // Then render a solid background fill
            let background = Paragraph::new("")
                .style(Style::default().bg(theme.base));
            frame.render_widget(background, dropdown_area);

            // Then render dropdown panel with borders on top
            let dropdown_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.overlay1));

            let dropdown_inner = dropdown_block.inner(dropdown_area);
            frame.render_widget(dropdown_block, dropdown_area);

            // Render options (simple iteration, no virtual scrolling needed for small lists)
            let max_visible = dropdown_inner.height as usize;
            let num_to_render = dropdown.options.len().min(max_visible);

            for idx in 0..num_to_render {
                let line_area = Rect {
                    x: dropdown_inner.x,
                    y: dropdown_inner.y + idx as u16,
                    width: dropdown_inner.width,
                    height: 1,
                };

                let option_text = &dropdown.options[idx];

                // Determine styling for this option
                let (prefix, fg_color, bg_color) = if idx == dropdown.highlight {
                    ("> ", theme.text, theme.surface0)
                } else if Some(idx) == dropdown.selected {
                    ("✓ ", theme.green, theme.base)
                } else {
                    ("  ", theme.text, theme.base)
                };

                // Render the option text with background
                let option_display = format!("{}{}", prefix, option_text);
                let option_widget = Paragraph::new(option_display)
                    .style(Style::default().fg(fg_color).bg(bg_color));
                frame.render_widget(option_widget, line_area);

                // Register click handler for this option
                match &dropdown.on_select {
                    DropdownCallback::Select(Some(select_fn)) => {
                        registry.register_click(line_area, select_fn(idx));
                    }
                    DropdownCallback::SelectEvent(Some(event_fn)) => {
                        use crate::tui::widgets::SelectEvent;
                        registry.register_click(line_area, event_fn(SelectEvent::Select(idx)));
                    }
                    DropdownCallback::Autocomplete(Some(select_fn)) => {
                        registry.register_click(line_area, select_fn(option_text.clone()));
                    }
                    DropdownCallback::AutocompleteEvent(Some(event_fn)) => {
                        use crate::tui::widgets::AutocompleteEvent;
                        registry.register_click(line_area, event_fn(AutocompleteEvent::Select(option_text.clone())));
                    }
                    _ => {}
                }
            }
        }
    }
}