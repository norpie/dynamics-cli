use ratatui::{Frame, layout::Rect};
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::collections::HashMap;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use crossterm::event::{KeyCode, MouseEvent};
use crate::tui::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppId {
    Example1,
    Example2,
    Queue,
    Migration,
    Deadlines,
    Settings,
}

#[derive(Debug, Clone)]
pub enum TuiMessage {
    SwitchFocus(AppId),
    YieldFocus,
    SendMessage { target: AppId, message: AppMessage },
    KillApp(AppId),
    Quit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMessage {
    pub id: MessageId,
    pub from: AppId,
    pub to: AppId,
    pub data: MessageData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub uuid::Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageData {
    ApiRequest { operation: String, data: serde_json::Value },
    ApiResponse { request_id: MessageId, result: serde_json::Value },
    StatusUpdate { status: String },
    Notification { message: String },
}

#[derive(Debug, Clone)]
pub enum Interaction {
    Click,
    RightClick,
    Hover,
    HoverExit,
    Scroll(ScrollDirection),
    Key(KeyCode),
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

pub struct InteractionRegistry {
    elements: HashMap<String, Rect>,
}

impl InteractionRegistry {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: &str, rect: Rect) {
        self.elements.insert(id.to_string(), rect);
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }

    pub fn find_element(&self, mouse_pos: (u16, u16)) -> Option<&str> {
        for (id, rect) in &self.elements {
            if self.point_in_rect(mouse_pos, *rect) {
                return Some(id);
            }
        }
        None
    }

    fn point_in_rect(&self, point: (u16, u16), rect: Rect) -> bool {
        let (x, y) = point;
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}

pub struct HeaderContent {
    pub title: String,
    pub status: Option<String>,
}

pub struct StartupContext {
    // Add fields as needed for startup
}

#[async_trait]
pub trait App: Send {
    fn id(&self) -> AppId;
    fn name(&self) -> &str;

    // Lifecycle
    async fn startup(&mut self, context: StartupContext) -> Result<()>;
    async fn shutdown(&mut self);
    fn can_exit(&self) -> bool { true }

    // Runtime
    async fn handle_key(&mut self, key: KeyCode) -> Option<TuiMessage>;
    async fn handle_interaction(&mut self, element_id: &str, interaction: Interaction) -> Option<TuiMessage>;
    async fn handle_message(&mut self, message: AppMessage) -> Option<TuiMessage>;

    // Rendering
    fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, registry: &mut InteractionRegistry);
    fn header_content(&self) -> HeaderContent;
}