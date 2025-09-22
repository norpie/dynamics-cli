use anyhow::Result;
use clap::Parser;
use log::{debug, info};

mod auth;
mod cli;
mod commands;
mod config;
mod dynamics;
mod fql;
mod ui;

use cli::Cli;
use cli::app::Commands;
use cli::commands::auth::AuthSubcommands;
use cli::commands::entity::EntitySubcommands;
use cli::commands::migration::MigrationSubCommands;
use cli::commands::query::QuerySubcommands;
use cli::commands::settings::SettingsSubcommands;
use commands::auth::{SetupOptions, remove_command, select_command, setup_command, status_command};
use commands::entity::{
    add_command, list_command, remove_command as entity_remove_command, update_command,
};
use commands::query::{file_command, run_command};
use commands::settings::{
    get_command, list_mappings_command, reset_all_command, reset_command, set_command, show_command,
};
use commands::migration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    dotenv::dotenv().ok();
    debug!("Environment variables loaded");

    let cli = Cli::parse();
    info!("Starting dynamics-cli");

    match cli.command {
        Commands::Auth(auth_commands) => match auth_commands.command {
            AuthSubcommands::Setup {
                name,
                host,
                username,
                password,
                client_id,
                client_secret,
                from_env,
                from_env_file,
            } => {
                setup_command(SetupOptions {
                    name,
                    host,
                    username,
                    password,
                    client_id,
                    client_secret,
                    from_env,
                    from_env_file,
                })
                .await?
            }
            AuthSubcommands::Select { name } => select_command(name).await?,
            AuthSubcommands::Remove { name, force } => remove_command(name, force).await?,
            AuthSubcommands::Status => status_command().await?,
        },
        Commands::Query(query_commands) => match query_commands.command {
            QuerySubcommands::Run {
                query,
                format,
                pretty,
            } => run_command(query, format, pretty).await?,
            QuerySubcommands::File {
                path,
                format,
                pretty,
            } => file_command(path, format, pretty).await?,
        },
        Commands::Entity(entity_commands) => match entity_commands.command {
            EntitySubcommands::List => list_command().await?,
            EntitySubcommands::Add {
                entity_name,
                plural_name,
            } => add_command(entity_name, plural_name).await?,
            EntitySubcommands::Remove { entity_name, force } => {
                entity_remove_command(entity_name, force).await?
            }
            EntitySubcommands::Update {
                entity_name,
                plural_name,
            } => update_command(entity_name, plural_name).await?,
        },
        Commands::Settings(settings_commands) => match settings_commands.command {
            SettingsSubcommands::Show => show_command().await?,
            SettingsSubcommands::Get { name } => get_command(name).await?,
            SettingsSubcommands::Set { name, value } => set_command(name, value).await?,
            SettingsSubcommands::Reset { name } => reset_command(name).await?,
            SettingsSubcommands::ResetAll { force } => reset_all_command(force).await?,
            SettingsSubcommands::ListMappings => list_mappings_command().await?,
        },
        Commands::Migration(migration_commands) => match migration_commands.command {
            MigrationSubCommands::Compare(args) => migration::execute(args).await?,
            MigrationSubCommands::Export(args) => migration::export(args).await?,
            MigrationSubCommands::Start => migration::start().await?,
        },
    }

    Ok(())
}
