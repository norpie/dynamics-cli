use std::future::Future;
use std::pin::Pin;
use std::any::Any;
use serde_json::Value;
use crate::tui::element::FocusId;

/// Target for event dispatch - either widget auto-routing or app message
///
/// This enum allows the runtime to distinguish between:
/// - Widget events that should be auto-dispatched to Field types via AppState
/// - App messages that should go directly to the update function
/// - Unhandled keys that should pass through to global subscriptions
pub enum DispatchTarget<Msg> {
    /// Widget event - runtime tries auto-dispatch via AppState::dispatch_widget_event
    WidgetEvent(Box<dyn Any + Send>),

    /// App message - goes directly to update() as before
    AppMsg(Msg),

    /// Pass through to global subscriptions without blurring focus
    PassThrough,
}

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

    /// Perform multiple async operations in parallel with automatic LoadingScreen management
    PerformParallel {
        tasks: Vec<ParallelTask>,
        config: ParallelConfig,
        msg_mapper: Box<dyn Fn(usize, Box<dyn Any + Send>) -> Msg + Send>,
    },

    /// Publish an event to the event bus
    Publish { topic: String, data: Value },

    /// Set focus to a specific element
    SetFocus(FocusId),

    /// Clear focus from all elements
    ClearFocus,

    /// Quit the application
    Quit,
}

/// A single task in a parallel operation
pub struct ParallelTask {
    pub description: String,
    pub future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send>>,
}

/// Configuration for parallel task execution
#[derive(Clone)]
pub struct ParallelConfig {
    pub title: Option<String>,
    pub on_complete: Option<AppId>,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            title: None,
            on_complete: None,
        }
    }
}

/// Unique identifier for each app
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppId {
    AppLauncher,
    LoadingScreen,
    ErrorScreen,
    MigrationEnvironment,
    MigrationComparisonSelect,
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

    /// Start building a parallel task execution command
    pub fn perform_parallel() -> ParallelBuilder<Msg>
    where
        Msg: Send + 'static,
    {
        ParallelBuilder::new()
    }
}

/// Builder for parallel task execution
pub struct ParallelBuilder<Msg> {
    tasks: Vec<ParallelTask>,
    config: ParallelConfig,
    _phantom: std::marker::PhantomData<Msg>,
}

impl<Msg: Send + 'static> ParallelBuilder<Msg> {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            config: ParallelConfig::default(),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Add a task to execute in parallel
    pub fn add_task<F, T>(mut self, description: impl Into<String>, future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = ParallelTask {
            description: description.into(),
            future: Box::pin(async move {
                let result = future.await;
                Box::new(result) as Box<dyn Any + Send>
            }),
        };
        self.tasks.push(task);
        self
    }

    /// Set the title shown on the loading screen
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    /// Set the app to navigate to when all tasks complete
    pub fn on_complete(mut self, app_id: AppId) -> Self {
        self.config.on_complete = Some(app_id);
        self
    }

    /// Build the command with a message mapper that converts task results to messages
    /// The mapper receives (task_index, result) and should downcast the result to the expected type
    pub fn build(self, msg_mapper: impl Fn(usize, Box<dyn Any + Send>) -> Msg + Send + 'static) -> Command<Msg> {
        Command::PerformParallel {
            tasks: self.tasks,
            config: self.config,
            msg_mapper: Box::new(msg_mapper),
        }
    }
}

impl<Msg> Default for Command<Msg> {
    fn default() -> Self {
        Command::None
    }
}