use anyhow::Result;
use log::debug;
use reqwest::Client;
use serde_json::Value;

pub struct DynamicsAuthClient {
    client: Client,
    token_url: String,
}

impl DynamicsAuthClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            token_url: "https://login.windows.net/common/oauth2/token".to_string(),
        }
    }

    pub async fn test_auth(
        &self,
        host: &str,
        username: &str,
        password: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Result<()> {
        debug!(
            "Attempting authentication to {} with client_id {}",
            host, client_id
        );

        let response = self
            .client
            .post(&self.token_url)
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

        debug!("Token request status: {}", response.status());

        if response.status().is_success() {
            let token_data: Value = response.json().await?;
            if token_data.get("access_token").is_some() {
                debug!("Access token obtained from authentication response");
                println!("  Token obtained successfully");
            }
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Token request failed: {}", error_text);
        }

        Ok(())
    }
}

impl Default for DynamicsAuthClient {
    fn default() -> Self {
        Self::new()
    }
}
