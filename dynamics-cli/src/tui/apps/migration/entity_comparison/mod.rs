mod app;
mod models;
mod tree_items;

pub use app::{EntityComparisonApp, EntityComparisonParams, State as EntityComparisonState};
pub use models::*;

// Internal message type for the app
#[derive(Clone)]
pub enum Msg {
    Back,
    ConfirmBack,
    CancelBack,
    SwitchTab(usize), // 1-indexed tab number
}
