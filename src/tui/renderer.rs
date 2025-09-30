use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    style::Style,
};
use std::collections::HashMap;
use crate::tui::{Element, Theme};

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
        for (rect, msg) in &self.click_handlers {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover(&self, x: u16, y: u16) -> Option<Msg> {
        for (rect, msg) in &self.hover_handlers {
            if self.point_in_rect(x, y, *rect) {
                return Some(msg.clone());
            }
        }
        None
    }

    pub fn find_hover_exit(&self, x: u16, y: u16) -> Option<Msg> {
        for (rect, msg) in &self.hover_exit_handlers {
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

/// Renders elements to the terminal
pub struct Renderer;

impl Renderer {
    pub fn render<Msg: Clone>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
        element: &Element<Msg>,
        area: Rect,
    ) {
        Self::render_element(frame, theme, registry, element, area);
    }

    fn render_element<Msg: Clone>(
        frame: &mut Frame,
        theme: &Theme,
        registry: &mut InteractionRegistry<Msg>,
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

            Element::Button {
                label,
                on_press,
                on_hover,
                on_hover_exit,
                style,
            } => {
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

                // Render button widget
                let default_style = Style::default().fg(theme.text);
                let block = Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.overlay0));
                let widget = Paragraph::new(label.as_str())
                    .block(block)
                    .alignment(Alignment::Center)
                    .style(style.unwrap_or(default_style));
                frame.render_widget(widget, area);
            }

            Element::Column { children, spacing } => {
                if children.is_empty() {
                    return;
                }

                // Calculate constraints for children
                let constraints: Vec<Constraint> = children
                    .iter()
                    .map(|_| Constraint::Length(3)) // TODO: Calculate actual size
                    .collect();

                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(constraints)
                    .split(area);

                // Render each child
                for (child, chunk) in children.iter().zip(chunks.iter()) {
                    Self::render_element(frame, theme, registry, child, *chunk);
                }
            }

            Element::Row { children, spacing } => {
                if children.is_empty() {
                    return;
                }

                // Calculate constraints for children
                let constraints: Vec<Constraint> = children
                    .iter()
                    .map(|_| Constraint::Percentage(100 / children.len() as u16))
                    .collect();

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(constraints)
                    .split(area);

                // Render each child
                for (child, chunk) in children.iter().zip(chunks.iter()) {
                    Self::render_element(frame, theme, registry, child, *chunk);
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
                Self::render_element(frame, theme, registry, child, padded_area);
            }
        }
    }
}