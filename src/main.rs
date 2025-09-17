use anyhow::Result;
use clap::Parser;
use log::{info, debug};

mod cli;
mod config;
mod auth;
mod ui;
mod commands;

use cli::Cli;
use cli::app::Commands;
use cli::commands::auth::AuthSubcommands;
use commands::auth::{setup_command, select_command, remove_command, status_command};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    dotenv::dotenv().ok();
    debug!("Environment variables loaded");

    let cli = Cli::parse();
    info!("Starting dynamics-cli");

    match cli.command {
        Commands::Auth(auth_commands) => {
            match auth_commands.command {
                AuthSubcommands::Setup {
                    name, host, username, password, client_id, client_secret, from_env, from_env_file
                } => setup_command(name, host, username, password, client_id, client_secret, from_env, from_env_file).await?,
                AuthSubcommands::Select { name } => select_command(name).await?,
                AuthSubcommands::Remove { name, force } => remove_command(name, force).await?,
                AuthSubcommands::Status => status_command().await?,
            }
        }
    }

    Ok(())
}