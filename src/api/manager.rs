use std::collections::HashMap;
use super::client::DynamicsClient;
use super::auth::AuthManager;
use super::models::{Environment, CredentialSet};


/// Manages multiple Dynamics client instances for different environments
pub struct ClientManager {
    clients: HashMap<String, DynamicsClient>,
    auth_manager: AuthManager,
    config: crate::config::Config,
    environments: HashMap<String, Environment>,
    current_env: Option<String>,
}

impl ClientManager {
    pub async fn from_env() -> anyhow::Result<Self> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let host = std::env::var("DYNAMICS_HOST")?;
        let username = std::env::var("DYNAMICS_USERNAME")?;
        let password = std::env::var("DYNAMICS_PASSWORD")?;
        let client_id = std::env::var("DYNAMICS_CLIENT_ID")?;
        let client_secret = std::env::var("DYNAMICS_CLIENT_SECRET")?;

        // Create test config (in-memory)
        let config = crate::config::Config::new_test().await?;
        let mut auth_manager = AuthManager::new();

        // Add test credentials to both config and auth manager
        let credentials = CredentialSet::UsernamePassword {
            username,
            password,
            client_id,
            client_secret,
        };

        // Save to test config
        config.add_credentials(".env".to_string(), credentials.clone()).await?;
        auth_manager.add_credentials(".env".to_string(), credentials);

        // Add test environment
        let environment = Environment {
            name: ".env".to_string(),
            host,
            credentials_ref: ".env".to_string(),
        };

        // Save to test config
        config.add_environment(environment.clone()).await?;
        config.set_current_environment(".env".to_string()).await?;

        let mut environments = HashMap::new();
        environments.insert(".env".to_string(), environment);

        Ok(Self {
            clients: HashMap::new(),
            auth_manager,
            config,
            environments,
            current_env: Some(".env".to_string()),
        })
    }

    /// Create a new ClientManager from the config database
    pub async fn new() -> anyhow::Result<Self> {
        let config = crate::config::Config::load().await?;
        let mut auth_manager = AuthManager::new();
        let mut environments = HashMap::new();

        // Load environments from config
        let env_names = config.list_environments().await?;
        for env_name in env_names {
            if let Some(environment) = config.get_environment(&env_name).await? {
                // Load credentials for this environment
                if let Some(credentials) = config.get_credentials(&environment.credentials_ref).await? {
                    auth_manager.add_credentials(environment.credentials_ref.clone(), credentials);
                }
                environments.insert(env_name, environment);
            }
        }

        // Get current environment
        let current_env = config.get_current_environment().await?;

        Ok(Self {
            clients: HashMap::new(),
            auth_manager,
            config,
            environments,
            current_env,
        })
    }

    pub async fn authenticate(&mut self) -> anyhow::Result<()> {
        let current_env = self.current_env.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No environment selected"))?
            .clone();

        let environment = self.try_select_env(&current_env)?;
        let host = environment.host.clone();
        let credentials_ref = environment.credentials_ref.clone();
        let credentials = self.auth_manager.try_select_credentials(&credentials_ref)?.clone();

        self.auth_manager
            .authenticate(&current_env, &host, &credentials)
            .await
    }

    // Environment management
    pub async fn add_environment(&mut self, name: String, environment: Environment) -> anyhow::Result<()> {
        // Save to config database
        self.config.add_environment(environment.clone()).await?;

        // Update local cache
        self.environments.insert(name, environment);
        Ok(())
    }

    pub async fn add_credentials(&mut self, name: String, credentials: CredentialSet) -> anyhow::Result<()> {
        // Save to config database
        self.config.add_credentials(name.clone(), credentials.clone()).await?;

        // Update auth manager
        self.auth_manager.add_credentials(name, credentials);
        Ok(())
    }

    /// Test authentication with specific credentials and environment
    pub async fn test_auth_with_credentials(&self, environment: &Environment, credentials: &CredentialSet) -> anyhow::Result<()> {
        // Create a temporary auth manager for testing
        let mut temp_auth_manager = AuthManager::new();
        temp_auth_manager.add_credentials("test".to_string(), credentials.clone());

        // Test authentication
        temp_auth_manager.authenticate("test", &environment.host, credentials).await
    }

    pub fn try_select_env(&self, name: &str) -> anyhow::Result<&Environment> {
        self.environments.get(name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", name))
    }

    pub fn list_environments(&self) -> Vec<&str> {
        self.environments.keys().map(|s| s.as_str()).collect()
    }

    pub fn delete_environment(&mut self, name: &str) -> anyhow::Result<()> {
        self.environments.remove(name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", name))?;
        Ok(())
    }

    pub fn rename_environment(&mut self, old_name: &str, new_name: String) -> anyhow::Result<()> {
        let mut environment = self.environments.remove(old_name)
            .ok_or_else(|| anyhow::anyhow!("Environment '{}' not found", old_name))?;
        environment.name = new_name.clone();
        self.environments.insert(new_name, environment);
        Ok(())
    }

    // Selection methods
    pub fn select_environment(&mut self, name: &str) -> anyhow::Result<()> {
        // Verify environment exists
        self.try_select_env(name)?;
        self.current_env = Some(name.to_string());
        Ok(())
    }

    pub fn get_current_environment(&self) -> Option<&str> {
        self.current_env.as_deref()
    }

    // Expose auth_manager for testing
    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }

    /// Get a configured DynamicsClient for the specified environment
    pub fn get_client(&self, env_name: &str) -> anyhow::Result<DynamicsClient> {
        let environment = self.try_select_env(env_name)?;

        // Get token for this environment
        let token_info = self.auth_manager.get_token(env_name)?;

        // Check if token is still valid (basic check)
        if let Ok(elapsed) = token_info.expires_at.elapsed() {
            if elapsed.as_secs() > 0 {
                anyhow::bail!("Token for environment '{}' has expired. Please re-authenticate.", env_name);
            }
        }

        Ok(DynamicsClient::new(
            environment.host.clone(),
            token_info.access_token.clone(),
        ))
    }

    /// Get a configured DynamicsClient for the current environment
    pub fn get_current_client(&self) -> anyhow::Result<DynamicsClient> {
        let current_env = self.current_env.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No environment selected"))?;
        self.get_client(current_env)
    }
}