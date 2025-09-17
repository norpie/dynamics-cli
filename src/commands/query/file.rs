use anyhow::Result;
use log::{info, debug};
use std::path::PathBuf;
use std::fs;
use crate::fql::{tokenize, parse, to_fetchxml};
use crate::config::Config;
use crate::dynamics::DynamicsClient;

/// Execute an FQL query from a file
///
/// # Arguments
/// * `path` - Path to file containing the FQL query
/// * `format` - Output format (xml, json, table)
/// * `pretty` - Whether to pretty print the output
///
/// # Returns
/// * `Ok(())` - Query executed successfully
/// * `Err(anyhow::Error)` - Query execution error
pub async fn file_command(path: PathBuf, format: String, pretty: bool) -> Result<()> {
    info!("Executing FQL query from file: {}", path.display());
    debug!("Output format: {}, Pretty: {}", format, pretty);

    // Check if file exists
    if !path.exists() {
        return Err(anyhow::anyhow!("File does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(anyhow::anyhow!("Path is not a file: {}", path.display()));
    }

    // Read the FQL query from file
    let query = fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read file {}: {}", path.display(), e))?;

    debug!("Read {} characters from file", query.len());

    // Trim whitespace and check if file is empty
    let query = query.trim();
    if query.is_empty() {
        return Err(anyhow::anyhow!("File contains no FQL query: {}", path.display()));
    }

    info!("Executing FQL query: {}", query);

    // Load config and get current authentication
    let config = Config::load()?;
    let auth_config = config.get_current_auth()
        .ok_or_else(|| anyhow::anyhow!("No authentication environment selected. Run 'dynamics-cli auth setup' first."))?;

    info!("Using environment: {}", config.get_current_environment_name().unwrap());

    // Parse the FQL query
    let tokens = tokenize(query)?;
    debug!("Tokenized FQL query into {} tokens", tokens.len());

    let ast = parse(tokens)?;
    debug!("Parsed FQL query into AST");

    // Generate FetchXML
    let fetchxml = to_fetchxml(ast)?;
    debug!("Generated FetchXML from AST");

    // Create Dynamics client and execute query
    let mut client = DynamicsClient::new(auth_config.clone());
    let result = client.query(&fetchxml, &format, pretty).await?;

    // Output the result
    println!("{}", result);

    info!("Query from file executed successfully");
    Ok(())
}