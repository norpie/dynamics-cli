use crossterm::event::KeyCode;
use serde_json::Value;
use std::time::Duration;

/// Subscriptions represent inputs that an app wants to receive.
/// They are registered via the subscriptions() function.
pub enum Subscription<Msg> {
    /// Subscribe to a specific keyboard key
    Keyboard {
        key: KeyCode,
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
    /// Helper to create a keyboard subscription
    pub fn keyboard(key: KeyCode, description: impl Into<String>, msg: Msg) -> Self {
        Subscription::Keyboard {
            key,
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