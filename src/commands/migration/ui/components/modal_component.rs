use crate::commands::migration::ui::styles::STYLES;
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub struct ModalComponent<T> {
    content: T,
    config: ModalConfig,
}

pub struct ModalConfig {
    pub title: Option<String>,
    pub closable: bool,
    pub click_outside_to_close: bool,
    pub show_close_button: bool,
    pub min_width: u16,
    pub min_height: u16,
    pub width_percent: u16,
    pub height_percent: u16,
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            title: None,
            closable: true,
            click_outside_to_close: true,
            show_close_button: true,
            min_width: 20,
            min_height: 5,
            width_percent: 60,
            height_percent: 60,
        }
    }
}

pub trait ModalContent {
    fn render_content(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyCode) -> ModalContentAction;
    fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> ModalContentAction;
    fn get_title(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalAction {
    Close,
    ContentAction(ModalContentAction),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalContentAction {
    Close,
    Submit(String),
    Cancel,
    Custom(String),
    None,
}

impl<T: ModalContent> ModalComponent<T> {
    pub fn new(content: T) -> Self {
        Self {
            content,
            config: ModalConfig::default(),
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.config.title = Some(title);
        self
    }

    pub fn with_config(mut self, config: ModalConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_size(mut self, width_percent: u16, height_percent: u16) -> Self {
        self.config.width_percent = width_percent;
        self.config.height_percent = height_percent;
        self
    }

    pub fn non_closable(mut self) -> Self {
        self.config.closable = false;
        self.config.click_outside_to_close = false;
        self.config.show_close_button = false;
        self
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let modal_area = self.calculate_modal_area(area);

        // Clear the modal area
        f.render_widget(Clear, modal_area);

        // Render modal background and border
        let title_string;
        let title = if let Some(ref config_title) = self.config.title {
            config_title.as_str()
        } else if let Some(content_title) = self.content.get_title() {
            title_string = content_title;
            title_string.as_str()
        } else {
            ""
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(STYLES.modal_border)
            .title_style(STYLES.modal_title);

        f.render_widget(block, modal_area);

        // Calculate content area (inside the border)
        let content_area = modal_area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        });

        // Render content
        self.content.render_content(f, content_area);

        // Render close button if enabled
        if self.config.show_close_button && self.config.closable {
            self.render_close_button(f, modal_area);
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> ModalAction {
        match key {
            KeyCode::Esc if self.config.closable => ModalAction::Close,
            _ => match self.content.handle_key(key) {
                ModalContentAction::Close => ModalAction::Close,
                other => ModalAction::ContentAction(other),
            },
        }
    }

    pub fn handle_mouse(&mut self, event: MouseEvent, area: Rect) -> ModalAction {
        let modal_area = self.calculate_modal_area(area);

        match event.kind {
            MouseEventKind::Down(_) => {
                // Check if click is outside modal
                if self.config.click_outside_to_close
                    && !self.point_in_rect(event.column, event.row, modal_area)
                {
                    return ModalAction::Close;
                }

                // Check if click is on close button
                if self.config.show_close_button && self.config.closable {
                    let close_button_area = self.get_close_button_area(modal_area);
                    if self.point_in_rect(event.column, event.row, close_button_area) {
                        return ModalAction::Close;
                    }
                }

                // Forward to content if click is inside modal
                if self.point_in_rect(event.column, event.row, modal_area) {
                    let content_area = modal_area.inner(Margin {
                        vertical: 1,
                        horizontal: 1,
                    });

                    match self.content.handle_mouse(event, content_area) {
                        ModalContentAction::Close => ModalAction::Close,
                        other => ModalAction::ContentAction(other),
                    }
                } else {
                    ModalAction::None
                }
            }
            _ => {
                // Forward other mouse events to content
                let content_area = modal_area.inner(Margin {
                    vertical: 1,
                    horizontal: 1,
                });

                match self.content.handle_mouse(event, content_area) {
                    ModalContentAction::Close => ModalAction::Close,
                    other => ModalAction::ContentAction(other),
                }
            }
        }
    }

    pub fn content(&self) -> &T {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut T {
        &mut self.content
    }

    fn calculate_modal_area(&self, area: Rect) -> Rect {
        let width = std::cmp::max(
            area.width * self.config.width_percent / 100,
            self.config.min_width,
        );
        let height = std::cmp::max(
            area.height * self.config.height_percent / 100,
            self.config.min_height,
        );

        let x = (area.width.saturating_sub(width)) / 2 + area.x;
        let y = (area.height.saturating_sub(height)) / 2 + area.y;

        Rect {
            x,
            y,
            width,
            height,
        }
    }

    fn render_close_button(&self, f: &mut Frame, modal_area: Rect) {
        if modal_area.width < 4 {
            return;
        }

        let close_area = Rect {
            x: modal_area.x + modal_area.width - 3,
            y: modal_area.y,
            width: 3,
            height: 1,
        };

        let close_button = Paragraph::new("[×]")
            .style(STYLES.error)
            .alignment(Alignment::Center);

        f.render_widget(close_button, close_area);
    }

    fn get_close_button_area(&self, modal_area: Rect) -> Rect {
        Rect {
            x: modal_area.x + modal_area.width - 3,
            y: modal_area.y,
            width: 3,
            height: 1,
        }
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}

// Simple text modal content implementation
pub struct TextModalContent {
    text: String,
}

impl TextModalContent {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

impl ModalContent for TextModalContent {
    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(self.text.clone()).style(STYLES.normal);
        f.render_widget(paragraph, area);
    }

    fn handle_key(&mut self, _key: KeyCode) -> ModalContentAction {
        ModalContentAction::None
    }

    fn handle_mouse(&mut self, _event: MouseEvent, _area: Rect) -> ModalContentAction {
        ModalContentAction::None
    }
}
