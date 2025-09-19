use clap::{Args, Subcommand};

#[derive(Args)]
pub struct EntityCommands {
    #[command(subcommand)]
    pub command: EntitySubcommands,
}

#[derive(Subcommand)]
pub enum EntitySubcommands {
    /// List all entity name mappings
    List,
    /// Add a new entity name mapping
    Add {
        /// Entity name (singular form used in FetchXML)
        entity_name: String,
        /// Plural name (used in Dynamics Web API)
        plural_name: String,
    },
    /// Remove an entity name mapping
    Remove {
        /// Entity name to remove
        entity_name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Update an existing entity name mapping
    Update {
        /// Entity name to update
        entity_name: String,
        /// New plural name
        plural_name: String,
    },
}
