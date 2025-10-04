use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::Value;
use std::time::Duration;

/// Represents a keyboard key with optional modifiers (Ctrl, Alt, Shift)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Create a key binding with no modifiers
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::empty(),
        }
    }

    /// Create a key binding with Ctrl modifier
    pub fn ctrl(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::CONTROL,
        }
    }

    /// Create a key binding with Alt modifier
    pub fn alt(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::ALT,
        }
    }

    /// Create a key binding with Shift modifier
    pub fn shift(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: KeyModifiers::SHIFT,
        }
    }

    /// Create a key binding with custom modifiers
    pub fn with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Check if this key binding matches the given key event
    pub fn matches(&self, event: &KeyEvent) -> bool {
        self.code == event.code && self.modifiers == event.modifiers
    }
}

/// Convert KeyCode to KeyBinding for backward compatibility
impl From<KeyCode> for KeyBinding {
    fn from(code: KeyCode) -> Self {
        Self::new(code)
    }
}

/// Subscriptions represent inputs that an app wants to receive.
/// They are registered via the subscriptions() function.
pub enum Subscription<Msg> {
    /// Subscribe to a specific keyboard key (with optional modifiers)
    Keyboard {
        key: KeyBinding,
        msg: Msg,
        description: String,
    },

    /// Subscribe to periodic timer events
    Timer { interval: Duration, msg: Msg },

    /// Subscribe to events on the event bus
    Subscribe {
        topic: String,
        handler: Box<dyn Fn(Value) -> Option<Msg> + Send>,
    },
}

impl<Msg> Subscription<Msg> {
    /// Helper to create a keyboard subscription (accepts KeyCode or KeyBinding)
    pub fn keyboard(key: impl Into<KeyBinding>, description: impl Into<String>, msg: Msg) -> Self {
        Subscription::Keyboard {
            key: key.into(),
            msg,
            description: description.into(),
        }
    }

    /// Helper to create a keyboard subscription with Ctrl modifier
    pub fn ctrl_key(code: KeyCode, description: impl Into<String>, msg: Msg) -> Self {
        Subscription::Keyboard {
            key: KeyBinding::ctrl(code),
            msg,
            description: description.into(),
        }
    }

    /// Helper to create a keyboard subscription with Alt modifier
    pub fn alt_key(code: KeyCode, description: impl Into<String>, msg: Msg) -> Self {
        Subscription::Keyboard {
            key: KeyBinding::alt(code),
            msg,
            description: description.into(),
        }
    }

    /// Helper to create a keyboard subscription with Shift modifier
    pub fn shift_key(code: KeyCode, description: impl Into<String>, msg: Msg) -> Self {
        Subscription::Keyboard {
            key: KeyBinding::shift(code),
            msg,
            description: description.into(),
        }
    }

    /// Helper to create a timer subscription
    pub fn timer(interval: Duration, msg: Msg) -> Self {
        Subscription::Timer { interval, msg }
    }

    /// Helper to create an event bus subscription
    pub fn subscribe<F>(topic: impl Into<String>, handler: F) -> Self
    where
        F: Fn(Value) -> Option<Msg> + Send + 'static,
    {
        Subscription::Subscribe {
            topic: topic.into(),
            handler: Box::new(handler),
        }
    }
}