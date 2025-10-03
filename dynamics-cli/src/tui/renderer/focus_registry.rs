use crossterm::event::KeyCode;
use ratatui::layout::Rect;
use crate::tui::element::FocusId;
use crate::tui::command::DispatchTarget;

/// Information about a focusable element
pub struct FocusableInfo<Msg> {
    pub id: FocusId,
    pub rect: Rect,
    pub on_key: Box<dyn Fn(KeyCode) -> DispatchTarget<Msg> + Send>,
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

    pub fn iter_active_layer(&self) -> impl Iterator<Item = &FocusableInfo<Msg>> {
        self.active_layer()
            .map(|layer| layer.focusables.iter())
            .into_iter()
            .flatten()
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

    /// Move focus to the next focusable element in the active layer
    /// Returns the new focused ID, or None if there are no focusables
    pub fn next_focus(&self, current: Option<&FocusId>) -> Option<FocusId> {
        let layer = self.active_layer()?;
        if layer.focusables.is_empty() {
            return None;
        }

        match current {
            None => {
                // No current focus, focus the first element
                Some(layer.focusables[0].id.clone())
            }
            Some(id) => {
                // Find current index
                let current_index = layer.focusables.iter().position(|f| &f.id == id)?;
                // Wrap around to next
                let next_index = (current_index + 1) % layer.focusables.len();
                Some(layer.focusables[next_index].id.clone())
            }
        }
    }

    /// Move focus to the previous focusable element in the active layer
    /// Returns the new focused ID, or None if there are no focusables
    pub fn prev_focus(&self, current: Option<&FocusId>) -> Option<FocusId> {
        let layer = self.active_layer()?;
        if layer.focusables.is_empty() {
            return None;
        }

        match current {
            None => {
                // No current focus, focus the last element
                Some(layer.focusables[layer.focusables.len() - 1].id.clone())
            }
            Some(id) => {
                // Find current index
                let current_index = layer.focusables.iter().position(|f| &f.id == id)?;
                // Wrap around to previous
                let prev_index = if current_index == 0 {
                    layer.focusables.len() - 1
                } else {
                    current_index - 1
                };
                Some(layer.focusables[prev_index].id.clone())
            }
        }
    }

    /// Dispatch a key event to the focused element
    /// Returns Some(Msg) if the element handled the key and produced a message
    pub fn dispatch_key(&self, focused_id: &FocusId, key: KeyCode) -> Option<Msg> {
        use crate::tui::command::DispatchTarget;

        let focusable = self.find_in_active_layer(focused_id)?;
        match (focusable.on_key)(key) {
            DispatchTarget::AppMsg(msg) => Some(msg),
            DispatchTarget::WidgetEvent(_) => None, // Widget events not supported in global focus
            DispatchTarget::PassThrough => None,
        }
    }
}
