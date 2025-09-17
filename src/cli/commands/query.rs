use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct QueryCommands {
    #[command(subcommand)]
    pub command: QuerySubcommands,
}

#[derive(Subcommand)]
pub enum QuerySubcommands {
    /// Execute an FQL query directly from command line
    Run {
        /// FQL query string to execute
        #[arg(help = "FQL query string (e.g., '.account | .name, .revenue | limit(10)')")]
        query: String,
        /// Output format (xml, json, table)
        #[arg(short, long, default_value = "xml")]
        format: String,
        /// Pretty print the output
        #[arg(short, long)]
        pretty: bool,
    },
    /// Execute an FQL query from a file
    File {
        /// Path to file containing FQL query
        #[arg(help = "Path to file containing FQL query")]
        path: PathBuf,
        /// Output format (xml, json, table)
        #[arg(short, long, default_value = "xml")]
        format: String,
        /// Pretty print the output
        #[arg(short, long)]
        pretty: bool,
    },
}