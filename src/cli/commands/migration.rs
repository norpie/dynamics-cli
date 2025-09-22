use clap::{Args, Subcommand};

#[derive(Args)]
pub struct MigrationCommands {
    #[command(subcommand)]
    pub command: MigrationSubCommands,
}

#[derive(Subcommand)]
pub enum MigrationSubCommands {
    /// Compare entity fields between two Dynamics instances
    Compare(CompareArgs),
    /// Export migration analysis to Excel file
    Export(ExportArgs),
    /// Launch migration management interface
    Start,
}

#[derive(Args)]
pub struct CompareArgs {
    /// Source entity name (e.g., 'account', 'old_account', 'contractor1_contact')
    pub source_entity: Option<String>,

    /// Target entity name (e.g., 'account', 'new_account', 'contractor2_contact')
    /// If not provided, uses the same name as source entity
    #[arg(long)]
    pub target_entity: Option<String>,

    /// Source (old) environment name
    #[arg(long)]
    pub source: Option<String>,

    /// Target (new) environment name
    #[arg(long)]
    pub target: Option<String>,
}

#[derive(Args)]
pub struct ExportArgs {
    /// Source entity name (e.g., 'account', 'old_account', 'contractor1_contact')
    pub source_entity: String,

    /// Target entity name (e.g., 'account', 'new_account', 'contractor2_contact')
    /// If not provided, uses the same name as source entity
    #[arg(long)]
    pub target_entity: Option<String>,

    /// Source (old) environment name
    #[arg(long)]
    pub source: String,

    /// Target (new) environment name
    #[arg(long)]
    pub target: String,

    /// Output Excel file path
    #[arg(short, long, default_value = "migration_analysis.xlsx")]
    pub output: String,
}