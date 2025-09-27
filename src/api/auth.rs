use std::collections::HashMap;
use super::models::{CredentialSet, TokenInfo};

/// Manages authentication tokens and credentials for multiple environments
pub struct AuthManager {
    credentials: HashMap<String, CredentialSet>,
    tokens: HashMap<String, TokenInfo>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            tokens: HashMap::new(),
        }
    }

    pub fn add_credentials(&mut self, name: String, credentials: CredentialSet) {
        self.credentials.insert(name, credentials);
    }

    pub fn try_select_credentials(&self, name: &str) -> anyhow::Result<&CredentialSet> {
        self.credentials.get(name)
            .ok_or_else(|| anyhow::anyhow!("Credentials '{}' not found", name))
    }

    // List methods
    pub fn list_credentials(&self) -> Vec<&str> {
        self.credentials.keys().map(|s| s.as_str()).collect()
    }

    // Delete methods
    pub fn delete_credentials(&mut self, name: &str) -> anyhow::Result<()> {
        self.credentials.remove(name)
            .ok_or_else(|| anyhow::anyhow!("Credentials '{}' not found", name))?;
        Ok(())
    }

    // Rename methods
    pub fn rename_credentials(&mut self, old_name: &str, new_name: String) -> anyhow::Result<()> {
        let credentials = self.credentials.remove(old_name)
            .ok_or_else(|| anyhow::anyhow!("Credentials '{}' not found", old_name))?;
        self.credentials.insert(new_name, credentials);
        Ok(())
    }

    // Authentication methods
    pub async fn authenticate(&mut self, env_name: &str, host: &str, credentials: &CredentialSet) -> anyhow::Result<()> {
        use std::time::{SystemTime, Duration};

        log::info!("Authenticating to {} for environment {}", host, env_name);

        match credentials {
            CredentialSet::UsernamePassword {
                username,
                password,
                client_id,
                client_secret,
            } => {
                let token_url = "https://login.microsoftonline.com/common/oauth2/token";

                let client = reqwest::Client::new();
                let response = client
                    .post(token_url)
                    .form(&[
                        ("grant_type", "password"),
                        ("client_id", client_id),
                        ("client_secret", client_secret),
                        ("username", username),
                        ("password", password),
                        ("resource", host),
                    ])
                    .send()
                    .await?;

                log::debug!("Token request status: {}", response.status());

                if response.status().is_success() {
                    let token_data: serde_json::Value = response.json().await?;

                    if let Some(access_token) = token_data.get("access_token").and_then(|t| t.as_str()) {
                        // Calculate expiration (default to 1 hour if not provided)
                        let expires_in = token_data
                            .get("expires_in")
                            .and_then(|e| e.as_u64())
                            .unwrap_or(3600);

                        let expires_at = SystemTime::now() + Duration::from_secs(expires_in);

                        let refresh_token = token_data
                            .get("refresh_token")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string());

                        let token_info = TokenInfo {
                            access_token: access_token.to_string(),
                            expires_at,
                            refresh_token,
                        };

                        self.tokens.insert(env_name.to_string(), token_info);

                        log::info!("Successfully authenticated for environment {}", env_name);
                        Ok(())
                    } else {
                        anyhow::bail!("No access token in response")
                    }
                } else {
                    let error_text = response.text().await?;
                    anyhow::bail!("Authentication failed: {}", error_text)
                }
            }
            _ => {
                anyhow::bail!("Authentication method not yet implemented: {:?}", credentials)
            }
        }
    }
}