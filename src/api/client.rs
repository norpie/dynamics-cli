use crate::config::AuthConfig;
use std::time::Duration;

/// Modern Dynamics 365 Web API client with connection pooling
pub struct DynamicsClient {
    auth_config: AuthConfig,
    http_client: reqwest::Client,
    access_token: Option<String>,
}

impl DynamicsClient {
    pub fn new(auth_config: AuthConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)           // Max idle connections per host
            .pool_idle_timeout(Duration::from_secs(90))  // Keep connections alive for 90s
            .timeout(Duration::from_secs(30))     // Request timeout
            .connect_timeout(Duration::from_secs(10))    // Connection timeout
            .user_agent("dynamics-cli/1.0")       // Custom user agent
            .build()
            .expect("Failed to build HTTP client");

        Self {
            auth_config,
            http_client,
            access_token: None,
        }
    }

    /// Get shared HTTP client for making requests (cheap clone)
    pub fn http_client(&self) -> reqwest::Client {
        self.http_client.clone()
    }

    /// Create a new client with custom HTTP client configuration
    pub fn with_custom_client(auth_config: AuthConfig, http_client: reqwest::Client) -> Self {
        Self {
            auth_config,
            http_client,
            access_token: None,
        }
    }
}