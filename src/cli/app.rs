use super::commands::AuthCommands;
use super::commands::deadlines::DeadlinesCommands;
use super::commands::entity::EntityCommands;
use super::commands::migration::MigrationCommands;
use super::commands::query::QueryCommands;
use super::commands::settings::SettingsCommands;
use super::commands::tui::TuiCommands;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "dynamics-cli")]
#[command(about = "A CLI tool for interacting with Microsoft Dynamics 365")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authentication management
    Auth(AuthCommands),
    /// Execute FQL queries against Dynamics 365
    Query(QueryCommands),
    /// Entity name mapping management
    Entity(EntityCommands),
    /// Application settings management
    Settings(SettingsCommands),
    /// Migration tools for comparing entities between CRM instances
    Migration(MigrationCommands),
    /// Deadlines management and tracking
    Deadlines(DeadlinesCommands),
    /// Launch interactive TUI interface
    Tui(TuiCommands),
}
