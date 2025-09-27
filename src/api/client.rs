use crate::config::AuthConfig;
use super::constants::{self, headers, methods};
use super::operations::{Operation, OperationResult, BatchRequestBuilder, BatchResponseParser};
use super::query::{Query, QueryResult, QueryResponse};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

/// Modern Dynamics 365 Web API client with connection pooling
#[derive(Clone)]
pub struct DynamicsClient {
    base_url: String,
    http_client: reqwest::Client,
    access_token: String,
}

impl DynamicsClient {
    pub fn new(base_url: String, access_token: String) -> Self {
        let http_client = reqwest::Client::builder()
            .pool_max_idle_per_host(10)           // Max idle connections per host
            .pool_idle_timeout(Duration::from_secs(90))  // Keep connections alive for 90s
            .timeout(Duration::from_secs(30))     // Request timeout
            .connect_timeout(Duration::from_secs(10))    // Connection timeout
            .user_agent("dynamics-cli/1.0")       // Custom user agent
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url,
            http_client,
            access_token,
        }
    }

    /// Create a new client with custom HTTP client configuration
    pub fn with_custom_client(base_url: String, access_token: String, http_client: reqwest::Client) -> Self {
        Self {
            base_url,
            http_client,
            access_token,
        }
    }

    /// Execute a single operation
    pub async fn execute(&self, operation: &Operation) -> anyhow::Result<OperationResult> {
        match operation {
            Operation::Create { entity, data } => self.create_record(entity, data).await,
            Operation::Update { entity, id, data } => self.update_record(entity, id, data).await,
            Operation::Delete { entity, id } => self.delete_record(entity, id).await,
            Operation::Upsert { entity, key_field, key_value, data } => {
                self.upsert_record(entity, key_field, key_value, data).await
            }
        }
    }

    /// Execute multiple operations as a batch
    pub async fn execute_batch(&self, operations: &[Operation]) -> anyhow::Result<Vec<OperationResult>> {
        if operations.is_empty() {
            return Ok(Vec::new());
        }

        if operations.len() == 1 {
            let result = self.execute(&operations[0]).await?;
            return Ok(vec![result]);
        }

        self.execute_batch_request(operations).await
    }

    /// Execute an OData query
    pub async fn execute_query(&self, query: &Query) -> anyhow::Result<QueryResult> {
        let url = constants::entity_endpoint(&self.base_url, &query.entity);
        let params = query.to_query_params();

        let response = self.http_client
            .get(&url)
            .bearer_auth(&self.access_token)
            .header("Accept", headers::CONTENT_TYPE_JSON)
            .header("OData-Version", headers::ODATA_VERSION)
            .query(&params)
            .send()
            .await?;

        self.parse_query_response(response).await
    }

    /// Execute the next page of results using @odata.nextLink
    pub async fn execute_next_page(&self, next_link: &str) -> anyhow::Result<QueryResult> {
        let response = self.http_client
            .get(next_link)
            .bearer_auth(&self.access_token)
            .header("Accept", headers::CONTENT_TYPE_JSON)
            .header("OData-Version", headers::ODATA_VERSION)
            .send()
            .await?;

        self.parse_query_response(response).await
    }

    /// Create a new record
    async fn create_record(&self, entity: &str, data: &Value) -> anyhow::Result<OperationResult> {
        let url = constants::entity_endpoint(&self.base_url, entity);

        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", headers::CONTENT_TYPE_JSON)
            .header("OData-Version", headers::ODATA_VERSION)
            .header("Prefer", headers::PREFER_RETURN_REPRESENTATION)
            .json(data)
            .send()
            .await?;

        self.parse_response(Operation::Create {
            entity: entity.to_string(),
            data: data.clone(),
        }, response).await
    }

    /// Update an existing record
    async fn update_record(&self, entity: &str, id: &str, data: &Value) -> anyhow::Result<OperationResult> {
        let url = constants::entity_record_endpoint(&self.base_url, entity, id);

        let response = self.http_client
            .patch(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", headers::CONTENT_TYPE_JSON)
            .header("OData-Version", headers::ODATA_VERSION)
            .header("If-Match", headers::IF_MATCH_ANY)
            .header("Prefer", headers::PREFER_RETURN_REPRESENTATION)
            .json(data)
            .send()
            .await?;

        self.parse_response(Operation::Update {
            entity: entity.to_string(),
            id: id.to_string(),
            data: data.clone(),
        }, response).await
    }

    /// Delete a record
    async fn delete_record(&self, entity: &str, id: &str) -> anyhow::Result<OperationResult> {
        let url = constants::entity_record_endpoint(&self.base_url, entity, id);

        let response = self.http_client
            .delete(&url)
            .bearer_auth(&self.access_token)
            .header("OData-Version", headers::ODATA_VERSION)
            .send()
            .await?;

        self.parse_response(Operation::Delete {
            entity: entity.to_string(),
            id: id.to_string(),
        }, response).await
    }

    /// Upsert a record using alternate key
    async fn upsert_record(&self, entity: &str, key_field: &str, key_value: &str, data: &Value) -> anyhow::Result<OperationResult> {
        let url = constants::upsert_endpoint(&self.base_url, entity, key_field, key_value);

        let response = self.http_client
            .patch(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", headers::CONTENT_TYPE_JSON)
            .header("OData-Version", headers::ODATA_VERSION)
            .header("Prefer", headers::PREFER_RETURN_REPRESENTATION)
            .json(data)
            .send()
            .await?;

        self.parse_response(Operation::Upsert {
            entity: entity.to_string(),
            key_field: key_field.to_string(),
            key_value: key_value.to_string(),
            data: data.clone(),
        }, response).await
    }

    /// Execute operations using the $batch endpoint
    async fn execute_batch_request(&self, operations: &[Operation]) -> anyhow::Result<Vec<OperationResult>> {
        let url = constants::batch_endpoint(&self.base_url);

        // Build the batch request using the proper builder
        let batch_request = BatchRequestBuilder::new(&self.base_url)
            .add_changeset(operations)
            .build();

        let content_type = batch_request.content_type().to_string();
        let body = batch_request.body().to_string();

        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.access_token)
            .header("Content-Type", content_type)
            .header("OData-Version", headers::ODATA_VERSION)
            .body(body)
            .send()
            .await?;

        if response.status().is_success() {
            let response_text = response.text().await?;
            // Use the proper parser
            BatchResponseParser::parse(&response_text, operations)
        } else {
            let error_text = response.text().await?;
            anyhow::bail!("Batch request failed: {}", error_text)
        }
    }


    /// Parse HTTP response into OperationResult
    async fn parse_response(&self, operation: Operation, response: reqwest::Response) -> anyhow::Result<OperationResult> {
        let status_code = response.status().as_u16();
        let mut headers = HashMap::new();

        // Extract useful headers
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        if response.status().is_success() {
            let data = if response.status() == 204 {
                // No content (delete operations)
                None
            } else {
                // Get response text first, then try to parse as JSON
                let text = response.text().await.unwrap_or_default();
                if text.is_empty() {
                    None
                } else {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(json) => Some(json),
                        Err(_) => Some(Value::String(text)),
                    }
                }
            };

            Ok(OperationResult {
                operation,
                success: true,
                data,
                error: None,
                status_code: Some(status_code),
                headers,
            })
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Ok(OperationResult {
                operation,
                success: false,
                data: None,
                error: Some(error_text),
                status_code: Some(status_code),
                headers,
            })
        }
    }

    /// Parse HTTP response into QueryResult
    async fn parse_query_response(&self, response: reqwest::Response) -> anyhow::Result<QueryResult> {
        let status_code = response.status().as_u16();
        let mut headers = HashMap::new();

        // Extract useful headers
        for (name, value) in response.headers() {
            if let Ok(value_str) = value.to_str() {
                headers.insert(name.to_string(), value_str.to_string());
            }
        }

        if response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            if text.is_empty() {
                return Ok(QueryResult::error(
                    "Empty response from server".to_string(),
                    Some(status_code),
                    headers,
                ));
            }

            match serde_json::from_str::<Value>(&text) {
                Ok(json) => {
                    match QueryResponse::from_json(json) {
                        Ok(query_response) => Ok(QueryResult::success(query_response, status_code, headers)),
                        Err(e) => Ok(QueryResult::error(
                            format!("Failed to parse OData response: {}", e),
                            Some(status_code),
                            headers,
                        )),
                    }
                },
                Err(e) => Ok(QueryResult::error(
                    format!("Invalid JSON response: {}", e),
                    Some(status_code),
                    headers,
                )),
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Ok(QueryResult::error(error_text, Some(status_code), headers))
        }
    }
}