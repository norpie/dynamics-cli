#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedSide {
    Source,
    Target,
}

impl FocusedSide {
    pub fn switch(&self) -> Self {
        match self {
            FocusedSide::Source => FocusedSide::Target,
            FocusedSide::Target => FocusedSide::Source,
        }
    }
}

#[derive(Debug)]
pub struct FocusManager {
    focused_side: FocusedSide,
}

impl FocusManager {
    pub fn new() -> Self {
        Self {
            focused_side: FocusedSide::Source,
        }
    }

    pub fn get_focused_side(&self) -> FocusedSide {
        self.focused_side
    }

    pub fn set_focused_side(&mut self, side: FocusedSide) {
        self.focused_side = side;
    }

    pub fn switch_focus(&mut self) {
        self.focused_side = self.focused_side.switch();
    }

    pub fn is_source_focused(&self) -> bool {
        matches!(self.focused_side, FocusedSide::Source)
    }

    pub fn is_target_focused(&self) -> bool {
        matches!(self.focused_side, FocusedSide::Target)
    }
}
