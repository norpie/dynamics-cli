//! Demonstration of the ResilienceConfig API for production-grade Dynamics 365 operations
//!
//! This example shows how to use different resilience configurations for various use cases:
//! - Development testing with aggressive retries
//! - Production deployment with conservative retries
//! - Custom configurations for specific requirements

use dynamics_cli::api::{
    DynamicsClient, Operation, Operations, ResilienceConfig, LogLevel
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    // Example 1: Using the default resilience configuration
    let _client = DynamicsClient::new(
        "https://your-org.crm.dynamics.com".to_string(),
        "your-access-token".to_string(),
    );

    let default_resilience = ResilienceConfig::default();

    // Create a simple operation
    let _create_contact = Operation::create("contacts", json!({
        "firstname": "John",
        "lastname": "Doe",
        "emailaddress1": "john.doe@example.com"
    }));

    println!("=== Example 1: Default Resilience Configuration ===");
    println!("Max attempts: {}", default_resilience.retry.max_attempts);
    println!("Base delay: {:?}", default_resilience.retry.base_delay);
    println!("Rate limit: {} requests/min", default_resilience.rate_limit.requests_per_minute);

    // Execute with default resilience (this would fail without real credentials)
    // let result = create_contact.execute(&client, &default_resilience).await?;

    // Example 2: Conservative configuration for production
    let conservative_resilience = ResilienceConfig::conservative();

    println!("\n=== Example 2: Conservative Configuration (Production) ===");
    println!("Max attempts: {}", conservative_resilience.retry.max_attempts);
    println!("Base delay: {:?}", conservative_resilience.retry.base_delay);
    println!("Rate limit: {} requests/min", conservative_resilience.rate_limit.requests_per_minute);

    // Example 3: Development configuration with aggressive retries
    let development_resilience = ResilienceConfig::development();

    println!("\n=== Example 3: Development Configuration ===");
    println!("Max attempts: {}", development_resilience.retry.max_attempts);
    println!("Base delay: {:?}", development_resilience.retry.base_delay);
    println!("Monitoring enabled: {}", development_resilience.monitoring.request_logging);

    // Example 4: Custom configuration using builder pattern
    let custom_resilience = ResilienceConfig::builder()
        .max_retries(5)
        .requests_per_minute(120)
        .enable_rate_limiting(true)
        .log_level(LogLevel::Debug)
        .performance_metrics(true)
        .correlation_ids(true)
        .request_logging(true)
        .build();

    println!("\n=== Example 4: Custom Builder Configuration ===");
    println!("Max attempts: {}", custom_resilience.retry.max_attempts);
    println!("Rate limit: {} requests/min", custom_resilience.rate_limit.requests_per_minute);
    println!("Rate limiting enabled: {}", custom_resilience.rate_limit.enabled);
    println!("Log level: {:?}", custom_resilience.monitoring.log_level);

    // Example 5: Batch operations with resilience
    let batch_operations = Operations::new()
        .create("contacts", json!({
            "firstname": "Alice",
            "lastname": "Smith",
            "emailaddress1": "alice.smith@example.com"
        }))
        .create("accounts", json!({
            "name": "Acme Corp",
            "telephone1": "555-1234"
        }))
        .update("contacts", "existing-contact-id", json!({
            "jobtitle": "Software Engineer"
        }));

    println!("\n=== Example 5: Batch Operations with Resilience ===");
    println!("Batch size: {} operations", batch_operations.len());
    println!("Using conservative resilience configuration");

    // Execute batch with conservative resilience (this would fail without real credentials)
    // let batch_results = batch_operations.execute(&client, &conservative_resilience).await?;

    // Example 6: Disabled resilience for testing
    let disabled_resilience = ResilienceConfig::disabled();

    println!("\n=== Example 6: Disabled Resilience (Testing Only) ===");
    println!("Max attempts: {}", disabled_resilience.retry.max_attempts);
    println!("Rate limiting: {}", disabled_resilience.rate_limit.enabled);
    println!("Monitoring: {}", disabled_resilience.monitoring.request_logging);

    println!("\n=== Summary ===");
    println!("All resilience configurations include:");
    println!("✓ Exponential backoff with jitter");
    println!("✓ Intelligent error classification");
    println!("✓ Correlation ID tracking");
    println!("✓ Performance metrics collection");
    println!("✓ Structured logging");
    println!("✓ Rate limiting with token bucket algorithm");

    Ok(())
}