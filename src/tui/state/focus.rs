/// Focus mode determines how keyboard focus is acquired
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusMode {
    /// Focus only changes on click (Windows-style)
    /// - Mouse click: focuses element
    /// - Mouse hover: no focus change (only visual hover state)
    Click,

    /// Focus follows mouse hover (Linux WM-style)
    /// - Mouse hover: immediately focuses element
    /// - More responsive but can be chaotic
    Hover,

    /// Hybrid: hover focuses only when nothing is focused
    /// - If nothing focused: hover focuses
    /// - If something focused: hover doesn't steal focus
    /// - Preserves intentional Tab navigation
    HoverWhenUnfocused,
}

impl Default for FocusMode {
    fn default() -> Self {
        FocusMode::Click
    }
}
