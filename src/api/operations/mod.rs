//! Dynamics 365 Operations Module
//!
//! This module provides a unified interface for Dynamics 365 CRUD operations
//! that can be executed individually or in batches.

pub mod operation;
pub mod operations;
pub mod batch;

pub use operation::{Operation, OperationResult};
pub use operations::Operations;
pub use batch::{BatchRequest, BatchRequestBuilder, BatchResponseParser};