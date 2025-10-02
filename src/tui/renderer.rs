use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph, Clear},
    style::{Style, Stylize},
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
    pub inside_panel: bool,  // True if this element is inside a Panel
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

/// Information about a dropdown that needs to be rendered as an overlay
pub struct DropdownInfo<Msg> {
    pub select_area: Rect,              // The area of the select widget
    pub options: Vec<String>,           // The dropdown options
    pub selected: usize,                // Selected index
    pub highlight: usize,               // Highlighted index
    pub on_select: Option<fn(usize) -> Msg>,  // Callback when option selected
}

/// Stores dropdowns to be rendered as overlays after main UI
pub struct DropdownRegistry<Msg> {
    dropdowns: Vec<DropdownInfo<Msg>>,
}

impl<Msg: Clone> DropdownRegistry<Msg> {
    pub fn new() -> Self {
        Self {
            dropdowns: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.dropdowns.clear();
    }

    pub fn register(&mut self, info: DropdownInfo<Msg>) {
        self.dropdowns.push(info);
    }

    pub fn dropdowns(&self) -> &[DropdownInfo<Msg>] {
        &self.dropdowns
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
        dropdown_registry: &mut DropdownRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
    ) {
        Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, element, area, false);
        // After rendering main UI, render all dropdowns as overlays
        Self::render_dropdowns(frame, theme, registry, dropdown_registry);
    }

    /// Check if an element or its descendants contain a focusable with the given ID
    fn element_contains_focus<Msg>(element: &Element<Msg>, focused_id: &FocusId) -> bool {
        match element {
            Element::Button { id, .. } | Element::List { id, .. } | Element::TextInput { id, .. } | Element::Tree { id, .. } | Element::Scrollable { id, .. } | Element::Select { id, .. } => id == focused_id,
            Element::Column { items, .. } | Element::Row { items, .. } => {
                items.iter().any(|(_, child)| Self::element_contains_focus(child, focused_id))
            }
            Element::Container { child, .. } | Element::Panel { child, .. } => {
                Self::element_contains_focus(child, focused_id)
            }
            Element::Stack { layers } => {
                layers.iter().any(|layer| Self::element_contains_focus(&layer.element, focused_id))
            }
            _ => false,
        }
    }

    /// Create on_key handler for buttons (Enter or Space activates)
    fn button_on_key<Msg: Clone + Send + 'static>(on_press: Option<Msg>) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
        Box::new(move |key| match key {
            KeyCode::Enter | KeyCode::Char(' ') => on_press.clone(),
            _ => None,
        })
    }

    /// Create on_key handler for lists (navigation and activation)
    fn list_on_key<Msg: Clone + Send + 'static>(
        selected: Option<usize>,
        on_navigate: Option<fn(KeyCode) -> Msg>,
        on_activate: Option<fn(usize) -> Msg>,
    ) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
        Box::new(move |key| match key {
            // Navigation keys - handled by on_navigate callback
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown
            | KeyCode::Home | KeyCode::End => {
                on_navigate.map(|f| f(key))
            }
            // Enter activates selected item
            KeyCode::Enter => {
                if let (Some(idx), Some(activate)) = (selected, on_activate) {
                    Some(activate(idx))
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    /// Create on_key handler for text inputs (all keys pass to on_change, Enter also fires on_submit)
    fn text_input_on_key<Msg: Clone + Send + 'static>(
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

    /// Create on_key handler for trees (navigation and toggle)
    fn tree_on_key<Msg: Clone + Send + 'static>(
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

    /// Create on_key handler for scrollable elements (scroll navigation)
    fn scrollable_on_key<Msg: Clone + Send + 'static>(
        on_scroll: Option<fn(usize) -> Msg>,
    ) -> Box<dyn Fn(KeyCode) -> Option<Msg> + Send> {
        Box::new(move |_key| {
            // Scrollable doesn't emit scroll messages directly via keyboard
            // The app should handle Up/Down/PageUp/PageDown in subscriptions
            // and call ScrollableState methods directly
            // This handler is here for focus management
            on_scroll.map(|_f| None)?
        })
    }

    /// Create on_key handler for select elements (dropdown navigation)
    fn select_on_key<Msg: Clone + Send + 'static>(
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
        dropdown_registry: &mut DropdownRegistry<Msg>,
        focused_id: Option<&FocusId>,
        element: &Element<Msg>,
        area: Rect,
        inside_panel: bool,
    ) {
        match element {
            Element::None => {}

            Element::Text { content, style } => {
                let default_style = Style::default().fg(theme.text);
                let widget = Paragraph::new(content.as_str())
                    .style(style.unwrap_or(default_style));
                frame.render_widget(widget, area);
            }

            Element::StyledText { line, background } => {
                let mut widget = Paragraph::new(line.clone());
                if let Some(bg_style) = background {
                    widget = widget.style(*bg_style);
                }
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

                // Render button widget
                let default_style = Style::default().fg(theme.text);
                // Only show focus border on button itself if NOT inside a panel
                // (panels will show focus on their border instead)
                let border_style = if is_focused && !inside_panel {
                    Style::default().fg(theme.lavender)
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
                    Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *chunk, inside_panel);
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
                    Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *chunk, inside_panel);
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
                Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, padded_area, inside_panel);
            }

            Element::Panel { child, title, .. } => {
                // Check if the child (or any descendant) contains the focused element
                let child_has_focus = focused_id
                    .map(|fid| Self::element_contains_focus(child, fid))
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
                Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, inner_area, true);
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
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::list_on_key(*selected, on_navigate.clone(), on_activate.clone()),
                    on_focus: on_focus.clone(),
                    on_blur: on_blur.clone(),
                    inside_panel,
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
                        Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *chunk, inside_panel);
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

                // Only render focus border if NOT inside a panel
                // (panels will show focus on their border instead)
                if is_focused && !inside_panel {
                    let border = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.lavender));
                    frame.render_widget(border, area);
                }
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
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::text_input_on_key(on_change.clone(), on_submit.clone()),
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
                let start_idx = *scroll_offset;
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

            Element::Tree {
                id,
                items,
                node_ids,
                selected,
                scroll_offset,
                on_select,
                on_toggle,
                on_navigate,
                on_focus,
                on_blur,
            } => {
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::tree_on_key(selected.clone(), on_navigate.clone(), on_toggle.clone()),
                    on_focus: on_focus.clone(),
                    on_blur: on_blur.clone(),
                    inside_panel,
                });

                // Check if this tree is focused
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
                        Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, *chunk, inside_panel);
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

                // Only render focus border if NOT inside a panel
                if is_focused && !inside_panel {
                    let border = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.lavender));
                    frame.render_widget(border, area);
                }
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
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::scrollable_on_key(on_scroll.clone()),
                    on_focus: on_focus.clone(),
                    on_blur: on_blur.clone(),
                    inside_panel,
                });

                // Check if this scrollable is focused
                let is_focused = focused_id == Some(id);

                // Calculate dimensions
                let viewport_height = area.height as usize;

                // Determine content height
                let detected_content_height = match child.as_ref() {
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
                let clamped_scroll = (*scroll_offset).min(max_scroll);

                // Render content based on type
                match child.as_ref() {
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
                                Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, item_child, *chunk, inside_panel);
                            }
                        }
                    }
                    _ => {
                        // For other element types, render normally (can't virtual scroll)
                        Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, child, content_area, inside_panel);
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

            Element::Select {
                id,
                options,
                selected,
                is_open,
                highlight,
                on_select,
                on_toggle,
                on_navigate,
                on_focus,
                on_blur,
            } => {
                // Register in focus registry
                focus_registry.register_focusable(FocusableInfo {
                    id: id.clone(),
                    rect: area,
                    on_key: Self::select_on_key(*is_open, on_toggle.clone(), on_navigate.clone()),
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
                let selected_text = if *selected < options.len() {
                    &options[*selected]
                } else {
                    ""
                };

                // Render closed state: Panel with selected value + arrow
                let arrow = if *is_open { " ▲" } else { " ▼" };
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
                if *is_open && !options.is_empty() {
                    dropdown_registry.register(DropdownInfo {
                        select_area,
                        options: options.clone(),
                        selected: *selected,
                        highlight: *highlight,
                        on_select: *on_select,
                    });
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
                    Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, &layer.element, layer_area, inside_panel);

                    // Pop focus layer context
                    focus_registry.pop_layer();
                }

                // Clear all interactions and focus, then re-render topmost layer to register only its interactions/focus
                registry.clear();
                focus_registry.clear();
                if let Some(last_layer) = layers.last() {
                    let layer_idx = layers.len() - 1;
                    let layer_area = Self::calculate_layer_position(&last_layer.element, last_layer.alignment, area);

                    // Re-push the topmost layer context
                    focus_registry.push_layer(layer_idx);
                    Self::render_element(frame, theme, registry, focus_registry, dropdown_registry, focused_id, &last_layer.element, layer_area, inside_panel);
                    // Keep the layer pushed so focusables remain in active layer
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
                } else if idx == dropdown.selected {
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
                if let Some(select_fn) = dropdown.on_select {
                    registry.register_click(line_area, select_fn(idx));
                }
            }
        }
    }
}