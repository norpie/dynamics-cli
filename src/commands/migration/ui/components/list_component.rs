use crate::commands::migration::ui::{
    mouse::{MouseAction, MouseHandler, MouseZone},
    styles::STYLES,
};
use crossterm::event::{KeyCode, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use std::cell::RefCell;
use std::fmt::Display;

pub struct ListComponent<T> {
    items: Vec<T>,
    state: RefCell<ListState>,
    mouse_handler: RefCell<MouseHandler>,
    config: ListConfig,
    hover_index: RefCell<Option<usize>>,
}

pub struct ListConfig {
    pub title: Option<String>,
    pub allow_create_new: bool,
    pub create_new_key: char,
    pub create_new_label: String,
    pub enable_mouse: bool,
    pub enable_scroll: bool,
    pub show_indices: bool,
    pub highlight_selected: bool,
}

impl Default for ListConfig {
    fn default() -> Self {
        Self {
            title: None,
            allow_create_new: false,
            create_new_key: 'n',
            create_new_label: "Create New".to_string(),
            enable_mouse: true,
            enable_scroll: true,
            show_indices: false,
            highlight_selected: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListAction {
    Selected(usize),
    CreateNew,
    None,
}

impl<T: Display> ListComponent<T> {
    pub fn new(items: Vec<T>) -> Self {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }

        Self {
            items,
            state: RefCell::new(state),
            mouse_handler: RefCell::new(MouseHandler::new()),
            config: ListConfig::default(),
            hover_index: RefCell::new(None),
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.config.title = Some(title);
        self
    }

    pub fn with_create_new(mut self, key: char, label: String) -> Self {
        self.config.allow_create_new = true;
        self.config.create_new_key = key;
        self.config.create_new_label = label;
        self
    }

    pub fn with_mouse_support(mut self) -> Self {
        self.config.enable_mouse = true;
        self
    }

    pub fn with_indices(mut self) -> Self {
        self.config.show_indices = true;
        self
    }

    pub fn with_config(mut self, config: ListConfig) -> Self {
        self.config = config;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.render_with_border_style(f, area, STYLES.normal);
    }

    pub fn render_with_border_style(
        &self,
        f: &mut Frame,
        area: Rect,
        border_style: ratatui::style::Style,
    ) {
        self.setup_mouse_zones(area);

        let list_items = self.build_list_items();

        let block = if let Some(ref title) = self.config.title {
            Block::default()
                .title(title.as_str())
                .borders(Borders::ALL)
                .border_style(border_style)
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
        };

        let list = List::new(list_items)
            .block(block)
            .highlight_style(STYLES.selected);

        f.render_stateful_widget(list, area, &mut *self.state.borrow_mut());
    }

    pub fn handle_key(&self, key: KeyCode) -> ListAction {
        match key {
            KeyCode::Up => {
                self.previous();
                ListAction::None
            }
            KeyCode::Down => {
                self.next();
                ListAction::None
            }
            KeyCode::PageUp => {
                self.page_up();
                ListAction::None
            }
            KeyCode::PageDown => {
                self.page_down();
                ListAction::None
            }
            KeyCode::Home => {
                self.first();
                ListAction::None
            }
            KeyCode::End => {
                self.last();
                ListAction::None
            }
            KeyCode::Enter => {
                if let Some(selected) = self.state.borrow().selected() {
                    ListAction::Selected(selected)
                } else {
                    ListAction::None
                }
            }
            KeyCode::Char(c) if c == self.config.create_new_key && self.config.allow_create_new => {
                ListAction::CreateNew
            }
            _ => ListAction::None,
        }
    }

    pub fn handle_mouse(&self, event: MouseEvent, area: Rect) -> ListAction {
        if !self.config.enable_mouse {
            return ListAction::None;
        }

        match event.kind {
            MouseEventKind::Down(_) => {
                if let Some(action) = self
                    .mouse_handler
                    .borrow()
                    .handle_click(event.column, event.row)
                {
                    match action {
                        MouseAction::SelectItem(index) => {
                            self.select(Some(index));
                            ListAction::Selected(index)
                        }
                    }
                } else {
                    ListAction::None
                }
            }
            MouseEventKind::Moved => {
                self.mouse_handler
                    .borrow_mut()
                    .handle_hover(event.column, event.row);
                ListAction::None
            }
            MouseEventKind::ScrollUp => {
                self.previous();
                ListAction::None
            }
            MouseEventKind::ScrollDown => {
                self.next();
                ListAction::None
            }
            _ => ListAction::None,
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.borrow().selected()
    }

    pub fn select(&self, index: Option<usize>) {
        self.state.borrow_mut().select(index);
    }

    pub fn next(&self) {
        let total_items = self.total_list_items();
        let mut state = self.state.borrow_mut();
        let i = match state.selected() {
            Some(i) => {
                if i >= total_items - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn previous(&self) {
        let total_items = self.total_list_items();
        let mut state = self.state.borrow_mut();
        let i = match state.selected() {
            Some(i) => {
                if i == 0 {
                    total_items - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn page_down(&self) {
        let total_items = self.total_list_items();
        let mut state = self.state.borrow_mut();
        let page_size = 10; // Could be configurable
        let i = match state.selected() {
            Some(i) => std::cmp::min(i + page_size, total_items - 1),
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn page_up(&self) {
        let mut state = self.state.borrow_mut();
        let page_size = 10; // Could be configurable
        let i = match state.selected() {
            Some(i) => i.saturating_sub(page_size),
            None => 0,
        };
        state.select(Some(i));
    }

    pub fn first(&self) {
        if !self.items.is_empty() {
            self.state.borrow_mut().select(Some(0));
        }
    }

    pub fn last(&self) {
        let total_items = self.total_list_items();
        if total_items > 0 {
            self.state.borrow_mut().select(Some(total_items - 1));
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn update_items(&mut self, items: Vec<T>) {
        self.items = items;
        let mut state = self.state.borrow_mut();
        if self.items.is_empty() {
            state.select(None);
        } else if state.selected().unwrap_or(0) >= self.items.len() {
            state.select(Some(self.items.len() - 1));
        }
    }

    fn build_list_items(&self) -> Vec<ListItem> {
        let mut items = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            let content = if self.config.show_indices {
                format!("{}: {}", i + 1, item)
            } else {
                item.to_string()
            };

            let style = if Some(i) == *self.hover_index.borrow() {
                STYLES.hover
            } else {
                STYLES.normal
            };

            items.push(ListItem::new(Line::from(Span::styled(content, style))));
        }

        items
    }

    fn total_list_items(&self) -> usize {
        self.items.len()
    }

    fn setup_mouse_zones(&self, area: Rect) {
        if !self.config.enable_mouse {
            return;
        }

        let mut mouse_handler = self.mouse_handler.borrow_mut();
        mouse_handler.clear_zones();

        let inner_area = if self.config.title.is_some() {
            Rect {
                x: area.x + 1,
                y: area.y + 2,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(3),
            }
        } else {
            Rect {
                x: area.x + 1,
                y: area.y + 1,
                width: area.width.saturating_sub(2),
                height: area.height.saturating_sub(2),
            }
        };

        for (i, _) in self.items.iter().enumerate() {
            if i as u16 >= inner_area.height {
                break;
            }

            let item_area = Rect {
                x: inner_area.x,
                y: inner_area.y + i as u16,
                width: inner_area.width,
                height: 1,
            };

            mouse_handler.add_zone(MouseZone {
                area: item_area,
                action: MouseAction::SelectItem(i),
                hover_style: Some(STYLES.hover),
            });
        }
    }
}
