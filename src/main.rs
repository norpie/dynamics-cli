#![allow(warnings)]

use anyhow::Result;
use clap::Parser;
use log::{debug, info};

mod api;
mod auth;
mod cli;
// mod commands; // Disabled during config rewrite
mod config;
// mod dynamics; // Disabled during config rewrite
mod fql;
mod ui;

use cli::Cli;
// use cli::app::Commands;
// use cli::commands::auth::AuthSubcommands;
// use cli::commands::entity::EntitySubcommands;
// use cli::commands::query::QuerySubcommands;
// use cli::commands::settings::SettingsSubcommands;
// use commands::auth::{SetupOptions, remove_command, select_command, setup_command, status_command};
// #[cfg(feature = "deadlines")]
// use commands::deadlines;
// use commands::entity::{
//     add_command, list_command, remove_command as entity_remove_command, update_command,
// };
// #[cfg(feature = "migration")]
// use commands::migration;
// use commands::query::{file_command, run_command};
// use commands::settings::{
//     get_command, list_mappings_command, reset_all_command, reset_command, set_command, show_command,
// };

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger to file (truncate on each run)
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("dynamics-cli.log")?;
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    let cli = Cli::parse();
    info!("Starting dynamics-cli");

    // Initialize ClientManager once (contains config internally)
    let client_manager = api::ClientManager::new().await?;

    // Handle commands
    use cli::app::Commands;
    match cli.command {
        Commands::Auth(auth_args) => {
            cli::commands::auth_command(auth_args, &client_manager).await?;
        }
        Commands::Query(query_args) => {
            cli::commands::handle_query_command(query_args, &client_manager).await?;
        }
        _ => {
            println!("Some commands are temporarily disabled during the config system rewrite.");
            println!("Available commands: auth, query");
            println!("Use --help with any command for more information.");
        }
    }

    Ok(())
}
