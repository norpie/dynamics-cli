use clap::{Args, Subcommand};

#[derive(Args)]
pub struct SettingsCommands {
    #[command(subcommand)]
    pub command: SettingsSubcommands,
}

#[derive(Subcommand)]
pub enum SettingsSubcommands {
    /// Show current settings
    Show,
    /// Get the value of a specific setting
    Get {
        /// Setting name
        name: String,
    },
    /// Set the value of a specific setting
    Set {
        /// Setting name
        name: String,
        /// Setting value
        value: String,
    },
    /// Reset a setting to its default value
    Reset {
        /// Setting name
        name: String,
    },
    /// Reset all settings to default values
    ResetAll {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}
