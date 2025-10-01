use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    style::Style,
};
use crossterm::event::KeyCode;
use std::collections::HashMap;
use crate::tui::{Element, Theme, LayoutConstraint, Layer, Alignment as LayerAlignment};
use crate::tui::element::FocusId;

/// Stores interaction handlers for UI elements
/// Maps (Rect, InteractionType) -> Message
pub struct InteractionRegistry<Msg> {
    click_handlers: Vec<(Rect, Msg)>,
    hover_handlers: Vec<(Rect, Msg)>,
    hover_exit_handlers: Vec<(Rect, Msg)>,
}

impl<Msg: Clone> InteractionRegistry<Msg> {
    pub fn new() -> Self {
        Self {
            click_handlers: Vec::new(),
            hover_handlers: Vec::new(),
            hover_exit_handlers: Vec::new(),
        }
    }

    pub fn register_click(&mut self, rect: Rect, msg: Msg) {
        self.click_handlers.push((rect, msg));
    }

    pub fn register_hover(&mut self, rect: Rect, msg: Msg) {
        self.hover_handlers.push((rect, msg));
    }

    pub fn register_hover_exit(&mut self, rect: Rect, msg: Msg) {
        self.hover_exit_handlers.push((rect, msg));
    }

    pub fn find_click(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.click_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.hover_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover_exit(&self, x: u16, y: u16) -> Option<Msg> {
        // Search in reverse order so topmost layers are checked first
        for (rect, msg) in self.hover_exit_handlers.iter().rev() {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.click_handlers.clear();
        self.hover_handlers.clear();
        self.hover_exit_handlers.clear();
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}

/// Information about a focusable element
pub struct FocusableInfo<Msg> {
    pub id: FocusId,
    pub rect: Rect,
    pub on_key: Box<dyn Fn(KeyCode) -> Option<Msg> + Send>,
    pub on_focus: Option<Msg>,
    pub on_blur: Option<Msg>,
}

/// Focus context for a single layer in the UI
pub struct LayerFocusContext<Msg> {
    pub layer_index: usize,
    pub focusables: Vec<FocusableInfo<Msg>>,
}

/// Stores focus information for UI elements, organized by layer
pub struct FocusRegistry<Msg> {
    layers: Vec<LayerFocusContext<Msg>>,
}

impl<Msg: Clone> FocusRegistry<Msg> {
    pub fn new() -> Self {
        Self {
            layers: vec![LayerFocusContext {
                layer_index: 0,
                focusables: Vec::new(),
            }],
        }
    }

    pub fn clear(&mut self) {
        self.layers.clear();
        self.layers.push(LayerFocusContext {
            layer_index: 0,
            focusables: Vec::new(),
        });
    }

    pub fn push_layer(&mut self, layer_index: usize) {
        self.layers.push(LayerFocusContext {
            layer_index,
            focusables: Vec::new(),
        });
    }

    pub fn pop_layer(&mut self) {
        if self.layers.len() > 1 {
            self.layers.pop();
        }
    }

    fn current_layer_mut(&mut self) -> &mut LayerFocusContext<Msg> {
        self.layers.last_mut().expect("FocusRegistry should always have at least one layer")
    }

    pub fn register_focusable(&mut self, info: FocusableInfo<Msg>) {
        // Check for duplicate IDs and warn/panic
        if self.current_layer_mut().focusables.iter().any(|f| f.id == info.id) {
            #[cfg(debug_assertions)]
            panic!("Duplicate FocusId detected: {:?}. Each focusable element must have a unique ID within its layer.", info.id);

            #[cfg(not(debug_assertions))]
            eprintln!("WARNING: Duplicate FocusId: {:?} - last registration wins", info.id);
        }

        self.current_layer_mut().focusables.push(info);
    }

    pub fn active_layer(&self) -> Option<&LayerFocusContext<Msg>> {
        self.layers.last()
    }

    pub fn find_in_active_layer(&self, id: &FocusId) -> Option<&FocusableInfo<Msg>> {
        self.active_layer()?.focusables.iter().find(|f| &f.id == id)
    }

    pub fn focusable_ids_in_active_layer(&self) -> Vec<FocusId> {
        self.active_layer()
            .map(|layer| layer.focusables.iter().map(|f| f.id.clone()).collect())
            .unwrap_or_default()
    }

    pub fn find_at_position(&self, x: u16, y: u16) -> Option<FocusId> {
        self.active_layer()?
            .focusables
            .iter()
            .rev()
            .find(|f| self.point_in_rect(x, y, f.rect))
            .map(|f| f.id.clone())
    }

    pub fn contains(&self, id: &FocusId) -> bool {
        self.layers.iter().any(|layer| {
            layer.focusables.iter().any(|f| &f.id == id)
        })
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}

/// Renders elements to the terminal
pub struct Renderer;

impl Renderer {
    pub fn render<Msg: Clone + Send + 'static>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        focus_registry: &mut FocusRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
    ) {
        Self::render_element(frame, theme, registry, focus_registry, focused_id, element, area);
    }

    /// Create on_key handler for buttons (Enter or Space activates)
    fn button_on_key<Msg: Clone + Send + 'static>(on_press: Option<Msg>) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
        Box::new(move |key| match key {
            KeyCode::Enter | KeyCode::Char(' ') => on_press.clone(),
            _ => None,
        })
    }

    /// Create on_key handler for lists (Enter activates selected item)
    fn list_on_key<Msg: Clone + Send + 'static>(
        _item_count: usize,
        _on_select: Option<fn(usize) -> Msg>,
        on_activate: Option<fn(usize) -> Msg>,
    ) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
        Box::new(move |key| match key {
            KeyCode::Enter => {
                // Note: We can't access the current selection here because it's in app state
                // The app will handle this via on_activate callback
                // For now, Enter is handled by the app's subscription
                None
            }
            _ => None,
        })
    }

    /// Calculate ratatui Constraints from our LayoutConstraints
    fn calculate_constraints<Msg>(
        items: &[(LayoutConstraint, Element<Msg>)],
        available_space: u16,
    ) -> Vec<Constraint> {
        // Pass 1: Calculate fixed and minimum sizes
        let mut fixed_total = 0u16;
        let mut fill_total_weight = 0u16;

        for (constraint, _) in items {
            match constraint {
                LayoutConstraint::Length(n) => fixed_total += n,
                LayoutConstraint::Min(n) => fixed_total += n,
                LayoutConstraint::Fill(weight) => fill_total_weight += weight,
            }
        }

        // Pass 2: Calculate remaining space for Fill elements
        let remaining = available_space.saturating_sub(fixed_total);

        // Pass 3: Build ratatui constraints
        items
            .iter()
            .map(|(constraint, _)| match constraint {
                LayoutConstraint::Length(n) => Constraint::Length(*n),
                LayoutConstraint::Min(n) => Constraint::Min(*n),
                LayoutConstraint::Fill(weight) => {
                    if fill_total_weight > 0 {
                        // Calculate proportional space
                        let space = (remaining as u32 * *weight as u32 / fill_total_weight as u32) as u16;
                        Constraint::Length(space)
                    } else {
                        Constraint::Length(0)
                    }
                }
            })
            .collect()
    }

    fn render_element<Msg: Clone + Send + 'static>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        focus_registry: &mut FocusRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
    ) {
        match element {
            Element::None => {}

            Element::Text { content, style } => {
                let default_style = Style::default().fg(theme.text);
                let widget = Paragraph::new(content.as_str())
                    .style(style.unwrap_or(default_style));
                frame.render_widget(widget, area);
            }

            Element::StyledText { line } => {
                let widget = Paragraph::new(line.clone());
                frame.render_widget(widget, area);
            }

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
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::button_on_key(on_press.clone()),
                    on_focus: on_focus.clone(),
                    on_blur: on_blur.clone(),
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

                // Render button widget
                let default_style = Style::default().fg(theme.text);
                let border_style = if is_focused {
                    Style::default().fg(theme.blue)
                } else {
                    Style::default().fg(theme.overlay0)
                };
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style);
                let widget = Paragraph::new(label.as_str())
                    .block(block)
                    .alignment(Alignment::Center)
                    .style(style.unwrap_or(default_style));
                frame.render_widget(widget, area);
            }

            Element::Column { items, spacing } => {
                if items.is_empty() {
                    return;
                }

                // Calculate ratatui constraints from layout constraints
                let constraints = Self::calculate_constraints(items, area.height);

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(area);

                // Render each child
                for ((_, child), chunk) in items.iter().zip(chunks.iter()) {
                    Self::render_element(frame, theme, registry, focus_registry, focused_id, child, *chunk);
                }
            }

            Element::Row { items, spacing } => {
                if items.is_empty() {
                    return;
                }

                // Calculate ratatui constraints from layout constraints
                let constraints = Self::calculate_constraints(items, area.width);

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(area);

                // Render each child
                for ((_, child), chunk) in items.iter().zip(chunks.iter()) {
                    Self::render_element(frame, theme, registry, focus_registry, focused_id, child, *chunk);
                }
            }

            Element::Container { child, padding } => {
                // Apply padding by shrinking the area
                let padded_area = Rect {
                    x: area.x + padding,
                    y: area.y + padding,
                    width: area.width.saturating_sub(padding * 2),
                    height: area.height.saturating_sub(padding * 2),
                };
                Self::render_element(frame, theme, registry, focus_registry, focused_id, child, padded_area);
            }

            Element::Panel { child, title } => {
                // Render border block
                let block = if let Some(title_text) = title {
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.overlay0))
                        .title(title_text.as_str())
                } else {
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.overlay0))
                };

                let inner_area = block.inner(area);
                frame.render_widget(block, area);

                // Render child in the inner area
                Self::render_element(frame, theme, registry, focus_registry, focused_id, child, inner_area);
            }

            Element::List {
                id,
                items,
                selected,
                scroll_offset,
                on_select,
                on_activate,
                on_focus,
                on_blur,
            } => {
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::list_on_key(items.len(), on_select.clone(), on_activate.clone()),
                    on_focus: on_focus.clone(),
                    on_blur: on_blur.clone(),
                });

                // Check if this list is focused
                let is_focused = focused_id == Some(id);

                // Calculate visible height
                let visible_height = area.height as usize;

                // Virtual scrolling: only render visible items
                let start_idx = *scroll_offset;
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
                        Self::render_element(frame, theme, registry, focus_registry, focused_id, child, *chunk);
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
                        (*scroll_offset as f32 / (items.len() - visible_height) as f32 * (area.height - 1) as f32) as u16
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

                // Render border if focused
                if is_focused {
                    let border = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.blue));
                    frame.render_widget(border, area);
                }
            }

            Element::Stack { layers } => {
                // Render all layers visually
                for (layer_idx, layer) in layers.iter().enumerate() {
                    // Render dim overlay if requested
                    if layer.dim_below {
                        Self::render_dim_overlay(frame, theme, area);
                    }

                    // Calculate position based on alignment
                    let layer_area = Self::calculate_layer_position(&layer.element, layer.alignment, area);

                    // Push focus layer context for this stack layer
                    focus_registry.push_layer(layer_idx);

                    // Render the layer element
                    Self::render_element(frame, theme, registry, focus_registry, focused_id, &layer.element, layer_area);

                    // Pop focus layer context
                    focus_registry.pop_layer();
                }

                // Clear all interactions and focus, then re-render topmost layer to register only its interactions/focus
                registry.clear();
                if let Some(last_layer) = layers.last() {
                    let layer_idx = layers.len() - 1;
                    let layer_area = Self::calculate_layer_position(&last_layer.element, last_layer.alignment, area);

                    // Re-push the topmost layer context
                    focus_registry.push_layer(layer_idx);
                    Self::render_element(frame, theme, registry, focus_registry, focused_id, &last_layer.element, layer_area);
                    focus_registry.pop_layer();
                }
            }
        }
    }

    /// Render a semi-transparent dim overlay
    fn render_dim_overlay(frame: &mut Frame, theme: &Theme, area: Rect) {
        let dim_block = Block::default()
            .style(Style::default().bg(theme.surface0));
        frame.render_widget(dim_block, area);
    }

    /// Calculate the position of a layer based on its alignment
    fn calculate_layer_position<Msg>(element: &Element<Msg>, alignment: LayerAlignment, container: Rect) -> Rect {
        // Estimate element size (for centered modal, use reasonable defaults)
        let (width, height) = Self::estimate_element_size(element, container);

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

    /// Estimate the size an element needs
    fn estimate_element_size<Msg>(element: &Element<Msg>, container: Rect) -> (u16, u16) {
        match element {
            Element::None => (0, 0),
            Element::Text { content, .. } => (content.len() as u16, 1),
            Element::StyledText { line } => (line.width() as u16, 1),
            Element::Button { label, .. } => (label.len() as u16 + 4, 3),
            Element::Panel { .. } => {
                // For panels (modals), use a reasonable default
                let width = container.width.min(60);
                let height = container.height.min(15);
                (width, height)
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
            _ => {
                // Default: 50% of container
                (container.width / 2, container.height / 2)
            }
        }
    }
}