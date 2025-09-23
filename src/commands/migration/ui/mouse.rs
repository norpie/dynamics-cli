use ratatui::{layout::Rect, style::Style};

#[derive(Debug, Clone)]
pub struct MouseZone {
    pub area: Rect,
    pub action: MouseAction,
    pub hover_style: Option<Style>,
}

#[derive(Debug, Clone)]
pub enum MouseAction {
    SelectItem(usize),
}

pub struct MouseHandler {
    zones: Vec<MouseZone>,
    hover_zone: Option<usize>,
}

impl MouseHandler {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            hover_zone: None,
        }
    }

    pub fn add_zone(&mut self, zone: MouseZone) {
        self.zones.push(zone);
    }

    pub fn clear_zones(&mut self) {
        self.zones.clear();
        self.hover_zone = None;
    }

    pub fn handle_click(&self, x: u16, y: u16) -> Option<MouseAction> {
        for zone in &self.zones {
            if self.point_in_rect(x, y, zone.area) {
                return Some(zone.action.clone());
            }
        }
        None
    }

    pub fn handle_hover(&mut self, x: u16, y: u16) -> bool {
        let new_hover = self
            .zones
            .iter()
            .position(|zone| self.point_in_rect(x, y, zone.area));

        let changed = new_hover != self.hover_zone;
        self.hover_zone = new_hover;
        changed
    }

    pub fn get_hover_style(&self, area: Rect) -> Option<Style> {
        if let Some(hover_idx) = self.hover_zone
            && let Some(zone) = self.zones.get(hover_idx)
            && zone.area == area
        {
            return zone.hover_style;
        }
        None
    }

    pub fn handle_scroll(&self, x: u16, y: u16, delta: i16) -> Option<MouseAction> {
        // Scroll handling is now delegated to specific components
        // that handle their own scroll logic
        let _ = (x, y, delta); // Silence unused parameter warnings
        None
    }

    fn point_in_rect(&self, x: u16, y: u16, rect: Rect) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }
}

impl Default for MouseHandler {
    fn default() -> Self {
        Self::new()
    }
}
