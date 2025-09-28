pub mod auth;
pub mod deadlines;
pub mod entity;
pub mod migration;
pub mod query;
pub mod settings;

// Re-export new auth command
pub use auth::{AuthCommands, auth_command};
