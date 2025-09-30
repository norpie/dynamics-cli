pub mod auth;
pub mod deadlines;
pub mod entity;
pub mod migration;
pub mod query;
pub mod settings;
pub mod tui;

// Re-export new auth command
pub use auth::{AuthCommands, auth_command};

// Re-export new query command
pub use query::{QueryCommands, handle_query_command};

// Re-export TUI command
pub use tui::{TuiCommands, tui_command};
