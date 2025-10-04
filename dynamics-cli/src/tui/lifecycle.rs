use std::time::Duration;

/// Lifecycle state of an app instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecycle {
    /// App hasn't been instantiated yet
    NotCreated,

    /// App is foreground, receiving input
    Running,

    /// App is suspended, state preserved, optionally processing async
    Background,

    /// Cleanup in progress
    QuittingRequested,

    /// Ready for removal
    Dead,
}

/// Policy for what happens when navigating away from an app
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitPolicy {
    /// Keep in background when navigating away (default)
    Sleep,

    /// Destroy immediately when navigating away
    QuitOnExit,

    /// Destroy after N seconds in background
    QuitOnIdle(Duration),
}

impl Default for QuitPolicy {
    fn default() -> Self {
        QuitPolicy::Sleep
    }
}

/// Reason why an app is being forcibly killed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillReason {
    /// Too many background apps
    MemoryPressure,

    /// QuitOnIdle policy expired
    PolicyTimeout,

    /// User explicitly closed app
    UserRequested,

    /// App entered error state
    Crashed,

    /// Navigating away with QuitOnExit policy
    NavigatedAway,
}
