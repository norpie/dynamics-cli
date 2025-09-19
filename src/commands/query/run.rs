use crate::config::Config;
use crate::dynamics::DynamicsClient;
use crate::fql::{parse, to_fetchxml, tokenize};
use anyhow::Result;
use log::{debug, info};

/// Execute an FQL query string directly
///
/// # Arguments
/// * `query` - The FQL query string to execute
/// * `format` - Output format (xml, json, table)
/// * `pretty` - Whether to pretty print the output
///
/// # Returns
/// * `Ok(())` - Query executed successfully
/// * `Err(anyhow::Error)` - Query execution error
pub async fn run_command(query: String, format: String, pretty: bool) -> Result<()> {
    info!("Executing FQL query: {}", query);
    debug!("Output format: {}, Pretty: {}", format, pretty);

    // Load config and get current authentication
    let config = Config::load()?;
    let auth_config = config.get_current_auth().ok_or_else(|| {
        anyhow::anyhow!(
            "No authentication environment selected. Run 'dynamics-cli auth setup' first."
        )
    })?;

    info!(
        "Using environment: {}",
        config.get_current_environment_name().unwrap()
    );

    // Parse the FQL query
    let tokens = tokenize(&query)?;
    debug!("Tokenized FQL query into {} tokens", tokens.len());

    let ast = parse(tokens, &query)?;
    debug!("Parsed FQL query into AST");

    // Generate FetchXML
    let fetchxml = to_fetchxml(ast)?;
    debug!("Generated FetchXML from AST");

    // Create Dynamics client and execute query
    let mut client = DynamicsClient::new(auth_config.clone());
    let result = client.query(&fetchxml, &format, pretty).await?;

    // Output the result
    println!("{}", result);

    info!("Query executed successfully");
    Ok(())
}
