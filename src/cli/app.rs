use clap::{Parser, Subcommand};
use super::commands::auth::AuthCommands;

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
}