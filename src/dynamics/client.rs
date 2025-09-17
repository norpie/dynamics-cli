use anyhow::Result;
use log::{debug, info};
use reqwest::Client;
use serde_json::Value;
use crate::config::AuthConfig;

pub struct DynamicsClient {
    client: Client,
    auth_config: AuthConfig,
    access_token: Option<String>,
}

impl DynamicsClient {
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            client: Client::new(),
            auth_config,
            access_token: None,
        }
    }

    /// Get or refresh the access token
    async fn get_access_token(&mut self) -> Result<&str> {
        if self.access_token.is_none() {
            self.authenticate().await?;
        }

        Ok(self.access_token.as_ref().unwrap())
    }

    /// Authenticate with Azure AD to get access token
    async fn authenticate(&mut self) -> Result<()> {
        info!("Authenticating with Dynamics 365...");
        debug!("Host: {}, Client ID: {}", self.auth_config.host, self.auth_config.client_id);

        let token_url = "https://login.windows.net/common/oauth2/token";

        let response = self.client
            .post(token_url)
            .form(&[
                ("grant_type", "password"),
                ("client_id", &self.auth_config.client_id),
                ("client_secret", &self.auth_config.client_secret),
                ("username", &self.auth_config.username),
                ("password", &self.auth_config.password),
                ("resource", &self.auth_config.host),
            ])
            .send()
            .await?;

        debug!("Token request status: {}", response.status());

        if response.status().is_success() {
            let token_data: Value = response.json().await?;
            if let Some(access_token) = token_data.get("access_token").and_then(|t| t.as_str()) {
                self.access_token = Some(access_token.to_string());
                debug!("Access token obtained successfully");
                return Ok(());
            }
            anyhow::bail!("Authentication failed: No access token in response");
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Authentication failed: {}", error_text)
        }
    }

    /// Execute a FetchXML query against Dynamics 365
    pub async fn execute_fetchxml(&mut self, fetchxml: &str) -> Result<Value> {
        let token = self.get_access_token().await?.to_string();

        // Construct the Web API URL for FetchXML queries
        let mut base_url = self.auth_config.host.clone();
        if !base_url.ends_with('/') {
            base_url.push('/');
        }
        let query_url = format!("{}api/data/v9.2/", base_url);

        info!("Executing FetchXML query against: {}", query_url);
        debug!("FetchXML: {}", fetchxml);

        // URL encode the FetchXML
        let encoded_fetchxml = urlencoding::encode(fetchxml);
        let full_url = format!("{}?fetchXml={}", query_url, encoded_fetchxml);

        let response = self.client
            .get(&full_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Accept", "application/json")
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .send()
            .await?;

        debug!("Query response status: {}", response.status());

        if response.status().is_success() {
            let result: Value = response.json().await?;
            debug!("Query executed successfully");
            Ok(result)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Query execution failed: {}", error_text)
        }
    }

    /// Execute a FetchXML query and return formatted results
    pub async fn query(&mut self, fetchxml: &str, format: &str, pretty: bool) -> Result<String> {
        let result = self.execute_fetchxml(fetchxml).await?;

        match format {
            "json" => {
                if pretty {
                    Ok(serde_json::to_string_pretty(&result)?)
                } else {
                    Ok(serde_json::to_string(&result)?)
                }
            },
            "xml" => {
                // For XML format, return the original FetchXML along with results
                if pretty {
                    Ok(format!("<!-- FetchXML Query -->\n{}\n\n<!-- Results -->\n{}",
                              fetchxml,
                              serde_json::to_string_pretty(&result)?))
                } else {
                    Ok(format!("{}\n{}", fetchxml, serde_json::to_string(&result)?))
                }
            },
            "table" => {
                // Format as a simple table
                self.format_as_table(&result)
            },
            _ => anyhow::bail!("Unsupported format: {}", format),
        }
    }

    /// Format query results as a simple table
    fn format_as_table(&self, result: &Value) -> Result<String> {
        let mut output = String::new();

        if let Some(values) = result.get("value").and_then(|v| v.as_array()) {
            if values.is_empty() {
                return Ok("No records found.".to_string());
            }

            // Get all unique column names from the first few records
            let mut columns = std::collections::HashSet::new();
            for record in values.iter().take(10) {
                if let Some(obj) = record.as_object() {
                    for key in obj.keys() {
                        if !key.starts_with('@') && !key.starts_with('_') {
                            columns.insert(key.clone());
                        }
                    }
                }
            }

            let mut column_vec: Vec<String> = columns.into_iter().collect();
            column_vec.sort();

            // Header
            output.push_str(&format!("{}\n", column_vec.join(" | ")));
            output.push_str(&format!("{}\n", "-".repeat(column_vec.len() * 15)));

            // Data rows
            for record in values {
                if let Some(obj) = record.as_object() {
                    let row: Vec<String> = column_vec.iter().map(|col| {
                        obj.get(col)
                            .and_then(|v| match v {
                                Value::String(s) => Some(s.clone()),
                                Value::Number(n) => Some(n.to_string()),
                                Value::Bool(b) => Some(b.to_string()),
                                Value::Null => Some("null".to_string()),
                                _ => Some("...".to_string()),
                            })
                            .unwrap_or_else(|| "".to_string())
                    }).collect();
                    output.push_str(&format!("{}\n", row.join(" | ")));
                }
            }

            output.push_str(&format!("\nTotal records: {}\n", values.len()));
        } else {
            output.push_str("Invalid response format\n");
        }

        Ok(output)
    }
}