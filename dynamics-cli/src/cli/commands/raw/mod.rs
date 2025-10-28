pub mod handler;

use clap::{Args, ValueEnum};
use std::path::PathBuf;

pub use handler::handle_raw_command;

#[derive(Args)]
pub struct RawCommands {
    /// API endpoint path (e.g., "accounts?$select=name&$top=5")
    #[arg(help = "API endpoint path")]
    pub endpoint: String,

    /// HTTP method
    #[arg(long, default_value = "get", help = "HTTP method")]
    pub method: HttpMethod,

    /// Request body data (JSON)
    #[arg(long, help = "Request body data (JSON string)")]
    pub data: Option<String>,

    /// Output format
    #[arg(long, default_value = "json", help = "Output format")]
    pub format: OutputFormat,

    /// Display style
    #[arg(long, default_value = "minimal", help = "Display style")]
    pub style: DisplayStyle,

    /// Environment name (overrides current environment)
    #[arg(long, help = "Environment name to use")]
    pub env: Option<String>,

    /// Disable colored output
    #[arg(long, help = "Disable colored output")]
    pub no_color: bool,

    /// Save results to file
    #[arg(short, long, help = "Save results to file")]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum HttpMethod {
    /// GET request
    Get,
    /// POST request
    Post,
    /// PATCH request
    Patch,
    /// DELETE request
    Delete,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Pretty-printed JSON (default)
    Json,
    /// Compact JSON (no whitespace, for piping)
    JsonCompact,
    /// XML format
    Xml,
    /// CSV format
    Csv,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DisplayStyle {
    /// Data only, no decorations (default)
    Minimal,
    /// Include timing, HTTP status, record counts, environment name
    Verbose,
}
