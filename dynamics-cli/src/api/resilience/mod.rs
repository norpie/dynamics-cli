//! Production resilience and hardening features
//!
//! Provides retry policies, rate limiting, and monitoring capabilities
//! for production-grade Dynamics 365 API interactions.

pub mod retry;
pub mod config;
pub mod rate_limiter;
pub mod logging;
pub mod metrics;

pub use retry::{RetryPolicy, RetryConfig, RetryableError};
pub use config::{ResilienceConfig, RateLimitConfig, MonitoringConfig, LogLevel};
pub use rate_limiter::{RateLimiter, RateLimiterStats};
pub use logging::{ApiLogger, OperationContext, OperationMetrics};
pub use metrics::{MetricsCollector, MetricsSnapshot, OperationTypeMetrics, EntityMetrics, GlobalMetrics};