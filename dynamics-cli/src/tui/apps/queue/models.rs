//! Data models for the operation queue

use crate::api::operations::{Operations, OperationResult};

/// Item in the operation queue
#[derive(Clone, Debug)]
pub struct QueueItem {
    /// Unique identifier for this queue item
    pub id: String,
    /// The operations to execute
    pub operations: Operations,
    /// Metadata about where this came from
    pub metadata: QueueMetadata,
    /// Current execution status
    pub status: OperationStatus,
    /// Priority (lower number = higher priority)
    pub priority: u8,
    /// Result after execution (if completed)
    pub result: Option<QueueResult>,
}

impl QueueItem {
    /// Create a new queue item with auto-generated ID and Pending status
    pub fn new(operations: Operations, metadata: QueueMetadata, priority: u8) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operations,
            metadata,
            status: OperationStatus::Pending,
            priority,
            result: None,
        }
    }
}

/// Metadata about where a queue item came from
#[derive(Clone, Debug)]
pub struct QueueMetadata {
    /// Source application/module (e.g., "Deadlines Excel", "Migration")
    pub source: String,
    /// Entity type being operated on (e.g., "cgk_deadline")
    pub entity_type: String,
    /// Human-readable description (e.g., "Row 5: Q1 Report")
    pub description: String,
    /// Source row number (if applicable)
    pub row_number: Option<usize>,
    /// Environment name for client lookup
    pub environment_name: String,
}

/// Status of a queue item
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OperationStatus {
    /// Waiting to execute
    Pending,
    /// Currently executing
    Running,
    /// User paused this item
    Paused,
    /// Completed successfully
    Done,
    /// Execution failed
    Failed,
}

impl OperationStatus {
    /// Get the display symbol for this status
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Pending => "·",
            Self::Running => "▶",
            Self::Paused => "⏸",
            Self::Done => "✓",
            Self::Failed => "⚠",
        }
    }
}

/// Result of executing a queue item
#[derive(Clone, Debug)]
pub struct QueueResult {
    /// Whether all operations succeeded
    pub success: bool,
    /// Results from individual operations
    pub operation_results: Vec<OperationResult>,
    /// Error message if failed
    pub error: Option<String>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
}

/// Filter for displaying queue items
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueFilter {
    /// Show all items
    #[default]
    All,
    /// Show only pending items
    Pending,
    /// Show only running items
    Running,
    /// Show only paused items
    Paused,
    /// Show only failed items
    Failed,
}

impl QueueFilter {
    /// Get display label for this filter
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Pending => "Pending",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Failed => "Failed",
        }
    }

    /// Check if an item matches this filter
    pub fn matches(&self, item: &QueueItem) -> bool {
        match self {
            Self::All => true,
            Self::Pending => item.status == OperationStatus::Pending,
            Self::Running => item.status == OperationStatus::Running,
            Self::Paused => item.status == OperationStatus::Paused,
            Self::Failed => item.status == OperationStatus::Failed,
        }
    }
}

/// Sort mode for queue items
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    /// Sort by priority (lower first)
    #[default]
    Priority,
    /// Sort by status
    Status,
    /// Sort by source
    Source,
}

impl SortMode {
    /// Get display label for this sort mode
    pub fn label(&self) -> &'static str {
        match self {
            Self::Priority => "Priority",
            Self::Status => "Status",
            Self::Source => "Source",
        }
    }
}
