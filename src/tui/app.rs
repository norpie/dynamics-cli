use crate::tui::{Command, Element, Subscription, Theme};
use ratatui::text::Line;

/// The main trait that all TUI apps must implement.
///
/// This follows the Elm architecture:
/// - State: immutable data that represents the app's current state
/// - Msg: events/actions that can happen
/// - update: pure function that handles messages and returns commands
/// - view: pure function that renders the current state
/// - subscriptions: declares what inputs the app wants to receive
pub trait App: Sized + Send + 'static {
    /// The app's state type
    type State: Default + Send;

    /// The app's message type
    type Msg: Clone + Send + 'static;

    /// Update the state based on a message and return a command
    fn update(state: &mut Self::State, msg: Self::Msg) -> Command<Self::Msg>;

    /// Render the current state to UI elements
    fn view(state: &Self::State, theme: &Theme) -> Element<Self::Msg>;

    /// Declare what inputs this app wants to receive
    fn subscriptions(state: &Self::State) -> Vec<Subscription<Self::Msg>>;

    /// Return the app's title (static string for help menu, etc.)
    fn title() -> &'static str;

    /// Return optional status text (dynamic, styled based on state)
    fn status(state: &Self::State, theme: &Theme) -> Option<Line<'static>> {
        None
    }

    /// Optional: Initialize the app with a command
    fn init() -> (Self::State, Command<Self::Msg>) {
        (Self::State::default(), Command::None)
    }
}