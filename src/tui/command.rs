use std::future::Future;
use std::pin::Pin;
use serde_json::Value;
use crate::tui::element::FocusId;

/// Commands represent side effects that apps want to perform.
/// They are returned from the update() function and executed by the runtime.
pub enum Command<Msg> {
    /// Do nothing
    None,

    /// Execute multiple commands in sequence
    Batch(Vec<Command<Msg>>),

    /// Navigate to a different app
    NavigateTo(AppId),

    /// Perform an async operation and send the result as a message
    Perform(Pin<Box<dyn Future<Output = Msg> + Send>>),

    /// Publish an event to the event bus
    Publish { topic: String, data: Value },

    /// Set focus to a specific element
    SetFocus(FocusId),

    /// Clear focus from all elements
    ClearFocus,

    /// Quit the application
    Quit,
}

/// Unique identifier for each app
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppId {
    AppLauncher,
    Example1,
    Example2,
    Example3,
    Example4,
    LoadingScreen,
    ErrorScreen,
}

impl<Msg> Command<Msg> {
    /// Helper to create a command that performs an async operation
    pub fn perform<F, T>(future: F, to_msg: impl Fn(T) -> Msg + Send + 'static) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        Msg: Send + 'static,
    {
        Command::Perform(Box::pin(async move {
            let result = future.await;
            to_msg(result)
        }))
    }

    /// Helper to navigate to another app
    pub fn navigate_to(app_id: AppId) -> Self {
        Command::NavigateTo(app_id)
    }

    /// Helper to publish an event
    pub fn publish<T: serde::Serialize>(topic: impl Into<String>, data: T) -> Self {
        Command::Publish {
            topic: topic.into(),
            data: serde_json::to_value(data).unwrap_or(Value::Null),
        }
    }

    /// Helper to batch multiple commands
    pub fn batch(commands: Vec<Command<Msg>>) -> Self {
        Command::Batch(commands)
    }

    /// Helper to set focus to an element
    pub fn set_focus(id: FocusId) -> Self {
        Command::SetFocus(id)
    }

    /// Helper to clear focus from all elements
    pub fn clear_focus() -> Self {
        Command::ClearFocus
    }
}

impl<Msg> Default for Command<Msg> {
    fn default() -> Self {
        Command::None
    }
}