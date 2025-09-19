use clap::{Args, Subcommand};

#[derive(Args)]
pub struct AuthCommands {
    #[command(subcommand)]
    pub command: AuthSubcommands,
}

#[derive(Subcommand)]
pub enum AuthSubcommands {
    /// Set up authentication for a new environment
    Setup {
        /// Name for this environment (e.g., "production", "test")
        #[arg(short, long)]
        name: Option<String>,
        /// Dynamics 365 Host URL
        #[arg(long)]
        host: Option<String>,
        /// Username
        #[arg(long)]
        username: Option<String>,
        /// Password
        #[arg(long)]
        password: Option<String>,
        /// Azure AD Application Client ID
        #[arg(long)]
        client_id: Option<String>,
        /// Azure AD Application Client Secret
        #[arg(long)]
        client_secret: Option<String>,
        /// Import credentials from environment variables
        #[arg(long)]
        from_env: bool,
        /// Import credentials from specified .env file
        #[arg(long)]
        from_env_file: Option<String>,
    },
    /// Select the current authentication environment
    Select {
        /// Environment name to select
        name: Option<String>,
    },
    /// Remove an authentication environment
    Remove {
        /// Environment name to remove
        name: String,
        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Show current authentication status
    Status,
}
