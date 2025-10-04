//! Resilience configuration with builder pattern
//!
//! Provides a unified configuration for retry policies, rate limiting,
//! and monitoring features with sane defaults.

use super::retry::RetryConfig;
use std::time::Duration;

/// Global resilience configuration for API operations
#[derive(Debug, Clone)]
pub struct ResilienceConfig {
    pub retry: RetryConfig,
    pub rate_limit: RateLimitConfig,
    pub monitoring: MonitoringConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_capacity: u32,
    pub enabled: bool,
}

/// Monitoring and logging configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub correlation_ids: bool,
    pub request_logging: bool,
    pub performance_metrics: bool,
    pub log_level: LogLevel,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            retry: RetryConfig::default(),
            rate_limit: RateLimitConfig::default(),
            monitoring: MonitoringConfig::default(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 90, // Conservative for Dynamics 365 (100/min limit)
            burst_capacity: 10,      // Allow small bursts
            enabled: true,
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            correlation_ids: true,
            request_logging: true,
            performance_metrics: true,
            log_level: LogLevel::Info,
        }
    }
}

impl ResilienceConfig {
    /// Create a new builder for ResilienceConfig
    pub fn builder() -> ResilienceConfigBuilder {
        ResilienceConfigBuilder::new()
    }

    /// Conservative config for production environments
    pub fn conservative() -> Self {
        Self {
            retry: RetryConfig::conservative(),
            rate_limit: RateLimitConfig {
                requests_per_minute: 60, // Very conservative
                burst_capacity: 5,
                enabled: true,
            },
            monitoring: MonitoringConfig {
                correlation_ids: true,
                request_logging: true,
                performance_metrics: true,
                log_level: LogLevel::Warn, // Less verbose in production
            },
        }
    }

    /// Aggressive config for development/testing
    pub fn development() -> Self {
        Self {
            retry: RetryConfig::aggressive(),
            rate_limit: RateLimitConfig {
                requests_per_minute: 200, // Higher limits for dev
                burst_capacity: 20,
                enabled: false, // Often disabled in dev
            },
            monitoring: MonitoringConfig {
                correlation_ids: true,
                request_logging: true,
                performance_metrics: true,
                log_level: LogLevel::Debug, // More verbose for debugging
            },
        }
    }

    /// Disable all resilience features (for testing)
    pub fn disabled() -> Self {
        Self {
            retry: RetryConfig {
                max_attempts: 1, // No retries
                base_delay: Duration::from_millis(0),
                max_delay: Duration::from_millis(0),
                backoff_multiplier: 1.0,
                jitter: false,
            },
            rate_limit: RateLimitConfig {
                requests_per_minute: u32::MAX,
                burst_capacity: u32::MAX,
                enabled: false,
            },
            monitoring: MonitoringConfig {
                correlation_ids: false,
                request_logging: false,
                performance_metrics: false,
                log_level: LogLevel::Error,
            },
        }
    }

    /// Load resilience config from the options system
    pub async fn load_from_options() -> anyhow::Result<Self> {
        let config = crate::global_config();

        // Load retry options
        let retry_enabled = config.options.get_bool("api.retry.enabled").await
            .unwrap_or(true);
        let max_attempts = config.options.get_uint("api.retry.max_attempts").await
            .unwrap_or(3) as u32;
        let base_delay_ms = config.options.get_uint("api.retry.base_delay_ms").await
            .unwrap_or(500);
        let max_delay_ms = config.options.get_uint("api.retry.max_delay_ms").await
            .unwrap_or(30000);
        let backoff_multiplier = config.options.get_float("api.retry.backoff_multiplier").await
            .unwrap_or(2.0);
        let jitter = config.options.get_bool("api.retry.jitter").await
            .unwrap_or(true);

        // Load rate limit options
        let rate_limit_enabled = config.options.get_bool("api.rate_limit.enabled").await
            .unwrap_or(true);
        let requests_per_minute = config.options.get_uint("api.rate_limit.requests_per_minute").await
            .unwrap_or(90) as u32;
        let burst_capacity = config.options.get_uint("api.rate_limit.burst_capacity").await
            .unwrap_or(10) as u32;

        // Load monitoring options
        let correlation_ids = config.options.get_bool("api.monitoring.correlation_ids").await
            .unwrap_or(true);
        let request_logging = config.options.get_bool("api.monitoring.request_logging").await
            .unwrap_or(true);
        let performance_metrics = config.options.get_bool("api.monitoring.performance_metrics").await
            .unwrap_or(true);
        let log_level_str = config.options.get_string("api.monitoring.log_level").await
            .unwrap_or_else(|_| "info".to_string());

        let log_level = match log_level_str.as_str() {
            "error" => LogLevel::Error,
            "warn" => LogLevel::Warn,
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Info,
        };

        Ok(Self {
            retry: RetryConfig {
                max_attempts: if retry_enabled { max_attempts } else { 1 },
                base_delay: Duration::from_millis(base_delay_ms),
                max_delay: Duration::from_millis(max_delay_ms),
                backoff_multiplier,
                jitter,
            },
            rate_limit: RateLimitConfig {
                requests_per_minute,
                burst_capacity,
                enabled: rate_limit_enabled,
            },
            monitoring: MonitoringConfig {
                correlation_ids,
                request_logging,
                performance_metrics,
                log_level,
            },
        })
    }
}

/// Builder for ResilienceConfig
#[derive(Debug)]
pub struct ResilienceConfigBuilder {
    config: ResilienceConfig,
}

impl ResilienceConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ResilienceConfig::default(),
        }
    }

    /// Configure retry behavior
    pub fn retry_config(mut self, retry: RetryConfig) -> Self {
        self.config.retry = retry;
        self
    }

    /// Set max retry attempts
    pub fn max_retries(mut self, attempts: u32) -> Self {
        self.config.retry.max_attempts = attempts;
        self
    }

    /// Configure rate limiting
    pub fn rate_limit_config(mut self, rate_limit: RateLimitConfig) -> Self {
        self.config.rate_limit = rate_limit;
        self
    }

    /// Set requests per minute limit
    pub fn requests_per_minute(mut self, rpm: u32) -> Self {
        self.config.rate_limit.requests_per_minute = rpm;
        self
    }

    /// Enable/disable rate limiting
    pub fn enable_rate_limiting(mut self, enabled: bool) -> Self {
        self.config.rate_limit.enabled = enabled;
        self
    }

    /// Configure monitoring
    pub fn monitoring_config(mut self, monitoring: MonitoringConfig) -> Self {
        self.config.monitoring = monitoring;
        self
    }

    /// Enable/disable correlation IDs
    pub fn correlation_ids(mut self, enabled: bool) -> Self {
        self.config.monitoring.correlation_ids = enabled;
        self
    }

    /// Enable/disable request logging
    pub fn request_logging(mut self, enabled: bool) -> Self {
        self.config.monitoring.request_logging = enabled;
        self
    }

    /// Enable/disable performance metrics
    pub fn performance_metrics(mut self, enabled: bool) -> Self {
        self.config.monitoring.performance_metrics = enabled;
        self
    }

    /// Set logging level
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.config.monitoring.log_level = level;
        self
    }

    /// Build the final configuration
    pub fn build(self) -> ResilienceConfig {
        self.config
    }
}

impl Default for ResilienceConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ResilienceConfig::default();

        assert_eq!(config.retry.max_attempts, 3);
        assert_eq!(config.rate_limit.requests_per_minute, 90);
        assert!(config.rate_limit.enabled);
        assert!(config.monitoring.correlation_ids);
        assert!(config.monitoring.request_logging);
    }

    #[test]
    fn test_conservative_config() {
        let config = ResilienceConfig::conservative();

        assert_eq!(config.retry.max_attempts, 2);
        assert_eq!(config.rate_limit.requests_per_minute, 60);
        assert!(config.rate_limit.enabled);
    }

    #[test]
    fn test_development_config() {
        let config = ResilienceConfig::development();

        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.rate_limit.requests_per_minute, 200);
        assert!(!config.rate_limit.enabled); // Disabled in dev
    }

    #[test]
    fn test_disabled_config() {
        let config = ResilienceConfig::disabled();

        assert_eq!(config.retry.max_attempts, 1);
        assert!(!config.rate_limit.enabled);
        assert!(!config.monitoring.correlation_ids);
        assert!(!config.monitoring.request_logging);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ResilienceConfig::builder()
            .max_retries(5)
            .requests_per_minute(120)
            .enable_rate_limiting(false)
            .correlation_ids(true)
            .log_level(LogLevel::Debug)
            .build();

        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.rate_limit.requests_per_minute, 120);
        assert!(!config.rate_limit.enabled);
        assert!(config.monitoring.correlation_ids);
    }
}