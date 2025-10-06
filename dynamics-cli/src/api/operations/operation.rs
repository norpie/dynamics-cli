//! Core Operation types for Dynamics 365 CRUD operations

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Represents a single CRUD operation that can be executed against Dynamics 365
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Create a new record
    Create {
        /// Entity logical name (e.g., "contacts", "accounts")
        entity: String,
        /// Record data as JSON
        data: Value,
    },
    /// Create a new record with references to previous operations in a batch
    /// Uses $<content-id> syntax to reference entities created earlier in the same changeset
    CreateWithRefs {
        /// Entity logical name (e.g., "cgk_cgk_deadline_cgk_support")
        entity: String,
        /// Record data as JSON
        data: Value,
        /// Map of field names to content-ID references
        /// e.g., {"cgk_deadlineid@odata.bind": "$1"} to reference the entity created with Content-ID 1
        content_id_refs: HashMap<String, String>,
    },
    /// Update an existing record
    Update {
        /// Entity logical name
        entity: String,
        /// Record ID (GUID)
        id: String,
        /// Updated field data as JSON
        data: Value,
    },
    /// Delete a record
    Delete {
        /// Entity logical name
        entity: String,
        /// Record ID (GUID)
        id: String,
    },
    /// Upsert operation (create or update based on key)
    Upsert {
        /// Entity logical name
        entity: String,
        /// Alternate key field (e.g., "emailaddress1")
        key_field: String,
        /// Key value to match against
        key_value: String,
        /// Record data as JSON
        data: Value,
    },
}

/// Result of executing an Operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// The operation that was executed
    pub operation: Operation,
    /// Whether the operation succeeded
    pub success: bool,
    /// Response data (record ID for creates, updated record for updates, etc.)
    pub data: Option<Value>,
    /// Error message if operation failed
    pub error: Option<String>,
    /// HTTP status code from the response
    pub status_code: Option<u16>,
    /// Response headers that might be useful (e.g., OData-EntityId)
    pub headers: HashMap<String, String>,
}

impl Operation {
    /// Create a new Create operation
    pub fn create(entity: impl Into<String>, data: Value) -> Self {
        Self::Create {
            entity: entity.into(),
            data,
        }
    }

    /// Create a new Update operation
    pub fn update(entity: impl Into<String>, id: impl Into<String>, data: Value) -> Self {
        Self::Update {
            entity: entity.into(),
            id: id.into(),
            data,
        }
    }

    /// Create a new Delete operation
    pub fn delete(entity: impl Into<String>, id: impl Into<String>) -> Self {
        Self::Delete {
            entity: entity.into(),
            id: id.into(),
        }
    }

    /// Create a new Upsert operation
    pub fn upsert(
        entity: impl Into<String>,
        key_field: impl Into<String>,
        key_value: impl Into<String>,
        data: Value,
    ) -> Self {
        Self::Upsert {
            entity: entity.into(),
            key_field: key_field.into(),
            key_value: key_value.into(),
            data,
        }
    }

    /// Get the entity name for this operation
    pub fn entity(&self) -> &str {
        match self {
            Self::Create { entity, .. } => entity,
            Self::CreateWithRefs { entity, .. } => entity,
            Self::Update { entity, .. } => entity,
            Self::Delete { entity, .. } => entity,
            Self::Upsert { entity, .. } => entity,
        }
    }

    /// Get the HTTP method for this operation
    pub fn http_method(&self) -> &'static str {
        match self {
            Self::Create { .. } => "POST",
            Self::CreateWithRefs { .. } => "POST",
            Self::Update { .. } => "PATCH",
            Self::Delete { .. } => "DELETE",
            Self::Upsert { .. } => "PATCH", // Upsert uses PATCH with specific headers
        }
    }

    /// Get the operation type as a string
    pub fn operation_type(&self) -> &'static str {
        match self {
            Self::Create { .. } => "create",
            Self::CreateWithRefs { .. } => "create_with_refs",
            Self::Update { .. } => "update",
            Self::Delete { .. } => "delete",
            Self::Upsert { .. } => "upsert",
        }
    }

    /// Execute this operation individually against a Dynamics client
    pub async fn execute(&self, client: &crate::api::DynamicsClient, resilience: &crate::api::ResilienceConfig) -> anyhow::Result<OperationResult> {
        client.execute(self, resilience).await
    }
}

impl OperationResult {
    /// Create a new successful result
    pub fn success(operation: Operation, data: Option<Value>) -> Self {
        Self {
            operation,
            success: true,
            data,
            error: None,
            status_code: Some(200),
            headers: HashMap::new(),
        }
    }

    /// Create a new error result
    pub fn error(operation: Operation, error: String, status_code: Option<u16>) -> Self {
        Self {
            operation,
            success: false,
            data: None,
            error: Some(error),
            status_code,
            headers: HashMap::new(),
        }
    }

    /// Check if this result represents a successful operation
    pub fn is_success(&self) -> bool {
        self.success
    }

    /// Check if this result represents a failed operation
    pub fn is_error(&self) -> bool {
        !self.success
    }

    /// Get the result data, returning an error if the operation failed
    pub fn into_result(self) -> Result<Value, String> {
        if self.success {
            Ok(self.data.unwrap_or(Value::Null))
        } else {
            Err(self.error.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }
}