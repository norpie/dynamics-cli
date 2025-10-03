pub mod handler;

use clap::{Args, ValueEnum};
use std::path::PathBuf;

pub use handler::handle_query_command;

#[derive(Args)]
pub struct QueryCommands {
    /// FQL query string to execute (e.g., '.account | .name, .revenue | limit(10)')
    #[arg(help = "FQL query string")]
    pub query: Option<String>,

    /// Execute FQL query from a file instead of command line
    #[arg(short, long, help = "Path to file containing FQL query")]
    pub file: Option<PathBuf>,

    /// Output format
    #[arg(long, default_value = "json", help = "Output format")]
    pub format: OutputFormat,

    /// Pretty print the output
    #[arg(short, long, help = "Pretty print the output")]
    pub pretty: bool,

    /// Show generated FetchXML without executing the query
    #[arg(long, help = "Show FetchXML without executing (dry run)")]
    pub dry: bool,

    /// Save query results to file
    #[arg(short, long, help = "Save results to file")]
    pub output: Option<PathBuf>,

    /// Show query execution time and statistics
    #[arg(long, help = "Show execution statistics")]
    pub stats: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// JSON format (default)
    Json,
    /// XML format
    Xml,
    /// CSV format
    Csv,
    /// Raw FetchXML (for debugging)
    FetchXml,
}
