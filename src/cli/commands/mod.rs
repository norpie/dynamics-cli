pub mod auth;
pub mod deadlines;
pub mod entity;
pub mod migration;
pub mod query;
pub mod query_handler;
pub mod settings;

// Re-export new auth command
pub use auth::{AuthCommands, auth_command};

// Re-export new query command
pub use query::QueryCommands;
pub use query_handler::handle_query_command;
