//! Tree node implementations for the operation queue

use crate::api::operations::Operation;
use crate::tui::{Element, Theme};
use crate::tui::widgets::{TreeItem, TableTreeItem};
use ratatui::layout::Constraint;
use super::models::QueueItem;

/// Message type for queue app
pub type Msg = super::app::Msg;

/// Node for table tree display
#[derive(Clone)]
pub enum QueueTreeNode {
    /// Parent node representing a queue item
    Parent(QueueItem),
    /// Child node representing an individual operation
    Child {
        operation: Operation,
        parent_id: String,
        index: usize,
    },
}

impl TreeItem for QueueTreeNode {
    type Msg = Msg;

    fn id(&self) -> String {
        match self {
            Self::Parent(item) => item.id.clone(),
            Self::Child { parent_id, index, .. } => format!("{}_{}", parent_id, index),
        }
    }

    fn has_children(&self) -> bool {
        match self {
            Self::Parent(item) => item.operations.len() > 1,
            Self::Child { .. } => false,
        }
    }

    fn children(&self) -> Vec<Self> {
        match self {
            Self::Parent(item) => {
                // Convert operations to child nodes
                // First operation is shown in the parent, rest are children
                item.operations
                    .operations()
                    .iter()
                    .skip(1) // Skip first operation (shown in parent)
                    .enumerate()
                    .map(|(idx, op)| Self::Child {
                        operation: op.clone(),
                        parent_id: item.id.clone(),
                        index: idx + 1, // +1 because we skipped first
                    })
                    .collect()
            }
            Self::Child { .. } => vec![],
        }
    }

    fn to_element(
        &self,
        _theme: &Theme,
        _depth: usize,
        _is_selected: bool,
        _is_expanded: bool,
    ) -> Element<Self::Msg> {
        // Not used for table trees
        Element::None
    }
}

impl TableTreeItem for QueueTreeNode {
    fn to_table_columns(
        &self,
        _depth: usize,
        _is_selected: bool,
        _is_expanded: bool,
    ) -> Vec<String> {
        match self {
            Self::Parent(item) => {
                let status_symbol = item.status.symbol();

                // Get first operation for display
                let first_op = item.operations.operations().first();
                let op_entity = first_op.map(|op| op.entity()).unwrap_or("unknown");

                // Actions buttons
                let actions = match item.status {
                    super::models::OperationStatus::Running => "[⏸]".to_string(),
                    super::models::OperationStatus::Paused => "[▶]".to_string(),
                    super::models::OperationStatus::Done => "".to_string(),
                    super::models::OperationStatus::Failed => "[Retry]".to_string(),
                    super::models::OperationStatus::Pending => "[⏸][↑][↓]".to_string(),
                };

                vec![
                    item.priority.to_string(),
                    status_symbol.to_string(),
                    format!("{} ({})", item.metadata.description, op_entity),
                    "BATCH".to_string(),
                    actions,
                ]
            }
            Self::Child { operation, .. } => {
                let op_type = operation.operation_type();
                let entity = operation.entity();

                vec![
                    "".to_string(),           // No priority for children
                    "".to_string(),           // No status for children
                    format!("└─ {}", entity), // Indented entity name
                    op_type.to_string(),
                    "".to_string(), // No actions for children
                ]
            }
        }
    }

    fn column_widths() -> Vec<Constraint> {
        vec![
            Constraint::Length(4),  // Priority
            Constraint::Length(3),  // Status symbol
            Constraint::Fill(1),    // Operation description (expandable)
            Constraint::Length(10), // Type
            Constraint::Length(15), // Actions
        ]
    }

    fn column_headers() -> Vec<String> {
        vec![
            "Pri".to_string(),
            "St".to_string(),
            "Operation".to_string(),
            "Type".to_string(),
            "Actions".to_string(),
        ]
    }
}
