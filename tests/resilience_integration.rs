//! Comprehensive integration tests for the resilience system
//!
//! Tests the complete resilience stack including retry policies, rate limiting,
//! structured logging, and performance metrics working together.

use dynamics_cli::api::{
    DynamicsClient, Operation, Operations, ResilienceConfig, RateLimitConfig,
    MonitoringConfig, LogLevel, RetryConfig, MetricsCollector, ApiLogger,
    RateLimiter, RetryableError
};
use serde_json::json;
use std::time::Duration;

/// Test that ResilienceConfig can be built with all components
#[tokio::test]
async fn test_resilience_config_integration() {
    let config = ResilienceConfig::builder()
        .max_retries(3)
        .requests_per_minute(120)
        .enable_rate_limiting(true)
        .log_level(LogLevel::Debug)
        .performance_metrics(true)
        .correlation_ids(true)
        .request_logging(true)
        .build();

    // Verify all components are configured correctly
    assert_eq!(config.retry.max_attempts, 3);
    assert_eq!(config.rate_limit.requests_per_minute, 120);
    assert!(config.rate_limit.enabled);
    assert!(config.monitoring.performance_metrics);
    assert!(config.monitoring.correlation_ids);
    assert!(config.monitoring.request_logging);
    assert!(matches!(config.monitoring.log_level, LogLevel::Debug));
}

/// Test different resilience presets work correctly
#[tokio::test]
async fn test_resilience_presets() {
    // Test default configuration
    let default_config = ResilienceConfig::default();
    assert_eq!(default_config.retry.max_attempts, 3);
    assert_eq!(default_config.rate_limit.requests_per_minute, 90);
    assert!(default_config.rate_limit.enabled);

    // Test conservative configuration
    let conservative_config = ResilienceConfig::conservative();
    assert_eq!(conservative_config.retry.max_attempts, 2);
    assert_eq!(conservative_config.rate_limit.requests_per_minute, 60);
    assert!(conservative_config.rate_limit.enabled);
    assert!(matches!(conservative_config.monitoring.log_level, LogLevel::Warn));

    // Test development configuration
    let dev_config = ResilienceConfig::development();
    assert_eq!(dev_config.retry.max_attempts, 5);
    assert_eq!(dev_config.rate_limit.requests_per_minute, 200);
    assert!(!dev_config.rate_limit.enabled); // Rate limiting disabled in dev
    assert!(matches!(dev_config.monitoring.log_level, LogLevel::Debug));

    // Test disabled configuration
    let disabled_config = ResilienceConfig::disabled();
    assert_eq!(disabled_config.retry.max_attempts, 1);
    assert!(!disabled_config.rate_limit.enabled);
    assert!(!disabled_config.monitoring.request_logging);
}

/// Test that rate limiting works as expected
#[tokio::test]
async fn test_rate_limiting_integration() {

    let config = RateLimitConfig {
        requests_per_minute: 120, // 2 requests per second
        burst_capacity: 2,
        enabled: true,
    };

    let rate_limiter = RateLimiter::new(config);

    // Should allow burst capacity immediately
    assert!(rate_limiter.try_acquire());
    assert!(rate_limiter.try_acquire());

    // Next request should be rate limited
    assert!(!rate_limiter.try_acquire());

    // Check statistics
    let stats = rate_limiter.stats();
    assert_eq!(stats.requests_made, 2);
    assert_eq!(stats.requests_rejected, 1);
    assert!(stats.acceptance_rate() > 0.6); // Should be around 2/3
}

/// Test structured logging with correlation tracking
#[tokio::test]
async fn test_logging_integration() {
    use std::collections::HashMap;

    let config = MonitoringConfig {
        correlation_ids: true,
        request_logging: true,
        performance_metrics: true,
        log_level: LogLevel::Debug,
    };

    let logger = ApiLogger::new(config);

    // Start an operation
    let context = logger.start_operation("create", "contacts", "test-correlation-123");
    assert_eq!(context.correlation_id, "test-correlation-123");
    assert_eq!(context.operation_type, "create");
    assert_eq!(context.entity, "contacts");

    // Test request logging
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer secret-token".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    // This should log (but we can't easily test log output in unit tests)
    logger.log_request(&context, "POST", "https://test.api.com/contacts", &headers);

    // Test response logging
    let mut response_headers = HashMap::new();
    response_headers.insert("Location".to_string(), "https://test.api.com/contacts/123".to_string());

    logger.log_response(&context, 201, &response_headers, Duration::from_millis(150));

    // Test completion logging
    let metrics = context.create_metrics(true, Some(201), None);
    logger.complete_operation(&context, &metrics);

    // Verify elapsed time tracking works
    assert!(context.elapsed() >= Duration::ZERO);
}

/// Test performance metrics collection
#[tokio::test]
async fn test_metrics_integration() {
    use dynamics_cli::api::OperationMetrics;

    let config = MonitoringConfig {
        correlation_ids: true,
        request_logging: false,
        performance_metrics: true,
        log_level: LogLevel::Info,
    };

    let collector = MetricsCollector::new(config);

    // Record some operations
    let create_metrics = OperationMetrics {
        duration: Duration::from_millis(100),
        retry_attempts: 0,
        success: true,
        status_code: Some(201),
        error_message: None,
        rate_limit_delays: vec![],
    };

    let update_metrics = OperationMetrics {
        duration: Duration::from_millis(150),
        retry_attempts: 1,
        success: false,
        status_code: Some(500),
        error_message: Some("Server error".to_string()),
        rate_limit_delays: vec![Duration::from_millis(50)],
    };

    collector.record_operation("create", "contacts", &create_metrics);
    collector.record_operation("update", "contacts", &update_metrics);

    // Get snapshot and verify metrics
    let snapshot = collector.snapshot();

    assert_eq!(snapshot.global.total_operations, 2);
    assert_eq!(snapshot.global.successful_operations, 1);
    assert_eq!(snapshot.global.failed_operations, 1);
    assert_eq!(snapshot.global.error_rate, 50.0);

    // Verify operation-specific metrics
    assert_eq!(snapshot.operations.len(), 2); // create and update

    let create_op_metrics = collector.operation_metrics("create").unwrap();
    assert_eq!(create_op_metrics.total_operations, 1);
    assert_eq!(create_op_metrics.success_rate(), 100.0);
    assert_eq!(create_op_metrics.average_duration(), Duration::from_millis(100));

    let update_op_metrics = collector.operation_metrics("update").unwrap();
    assert_eq!(update_op_metrics.total_operations, 1);
    assert_eq!(update_op_metrics.success_rate(), 0.0);
    assert_eq!(update_op_metrics.total_retries, 1);

    // Verify entity metrics
    let contact_metrics = collector.entity_metrics("contacts").unwrap();
    assert_eq!(contact_metrics.total_operations, 2);
    assert_eq!(contact_metrics.success_rate(), 50.0);
}

/// Test that retry policies work with different error types
#[tokio::test]
async fn test_retry_policy_integration() {
    use dynamics_cli::api::RetryPolicy;

    let config = RetryConfig {
        max_attempts: 3,
        base_delay: Duration::from_millis(10), // Very short for testing
        max_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
        jitter: false, // Disable jitter for predictable testing
    };

    let _retry_policy = RetryPolicy::new(config);

    // Test error classification
    assert!(RetryableError::from_status_code(500).should_retry());
    assert!(RetryableError::from_status_code(502).should_retry());
    assert!(RetryableError::from_status_code(429).should_retry());
    assert!(!RetryableError::from_status_code(400).should_retry());
    assert!(!RetryableError::from_status_code(401).should_retry());
    assert!(!RetryableError::from_status_code(404).should_retry());

    // Test that timeout errors are retryable
    assert!(RetryableError::from_status_code(408).should_retry());
}

/// Test complete operation lifecycle with all resilience features
#[tokio::test]
async fn test_complete_operation_lifecycle() {
    // This test would require a mock HTTP server to be truly comprehensive
    // For now, we'll test the individual components work together

    let _resilience_config = ResilienceConfig::builder()
        .max_retries(2)
        .requests_per_minute(60)
        .enable_rate_limiting(true)
        .log_level(LogLevel::Debug)
        .performance_metrics(true)
        .correlation_ids(true)
        .request_logging(true)
        .build();

    // Create a client (this won't actually connect since we're not using real credentials)
    let client = DynamicsClient::new(
        "https://test.crm.dynamics.com".to_string(),
        "fake-token".to_string(),
    );

    // Test that we can get initial metrics
    let initial_metrics = client.metrics_snapshot();
    assert_eq!(initial_metrics.global.total_operations, 0);

    // Test that we can get rate limiter stats
    let rate_stats = client.rate_limiter_stats();
    assert!(rate_stats.enabled); // Should use default config which has rate limiting enabled

    // Create an operation (this won't execute due to fake credentials, but tests the API)
    let _operation = Operation::create("contacts", json!({
        "firstname": "Test",
        "lastname": "User",
        "emailaddress1": "test@example.com"
    }));

    // Test operations collection
    let operations = Operations::new()
        .create("contacts", json!({"firstname": "Alice"}))
        .update("contacts", "123", json!({"lastname": "Smith"}))
        .delete("accounts", "456");

    assert_eq!(operations.len(), 3);
    assert!(!operations.is_empty());

    // Verify operation metadata
    for op in operations.operations() {
        match op {
            Operation::Create { entity, .. } => assert_eq!(entity, "contacts"),
            Operation::Update { entity, id, .. } => {
                assert_eq!(entity, "contacts");
                assert_eq!(id, "123");
            },
            Operation::Delete { entity, id } => {
                assert_eq!(entity, "accounts");
                assert_eq!(id, "456");
            },
            _ => panic!("Unexpected operation type"),
        }
    }
}

/// Test resilience configuration serialization/deserialization
#[tokio::test]
async fn test_config_serialization() {
    let config = ResilienceConfig::builder()
        .max_retries(5)
        .requests_per_minute(200)
        .enable_rate_limiting(false)
        .performance_metrics(true)
        .build();

    // Test that metrics can be serialized (for monitoring dashboards)
    let collector = MetricsCollector::new(config.monitoring.clone());

    let metrics = dynamics_cli::api::OperationMetrics {
        duration: Duration::from_millis(200),
        retry_attempts: 1,
        success: true,
        status_code: Some(200),
        error_message: None,
        rate_limit_delays: vec![Duration::from_millis(100)],
    };

    collector.record_operation("test", "test_entity", &metrics);
    let snapshot = collector.snapshot();

    // Verify snapshot contains serializable data
    assert!(!snapshot.timestamp.is_empty());
    assert_eq!(snapshot.global.total_operations, 1);
    assert_eq!(snapshot.operations.len(), 1);
    assert_eq!(snapshot.entities.len(), 1);

    // Test that the snapshot can be converted to JSON (via serde)
    let _json_result = serde_json::to_string(&snapshot);
    assert!(_json_result.is_ok());
}

/// Performance benchmark test (basic)
#[tokio::test]
async fn test_performance_overhead() {
    use std::time::Instant;

    let config = ResilienceConfig::development();
    let collector = MetricsCollector::new(config.monitoring);

    let start = Instant::now();

    // Record many operations to test overhead
    for i in 0..1000 {
        let metrics = dynamics_cli::api::OperationMetrics {
            duration: Duration::from_millis((i % 100) + 1), // Ensure non-zero duration
            retry_attempts: (i % 3) as u32,
            success: i % 4 != 0,
            status_code: Some(if i % 4 == 0 { 500 } else { 200 }),
            error_message: None,
            rate_limit_delays: vec![],
        };

        collector.record_operation(
            if i % 2 == 0 { "create" } else { "update" },
            "test_entity",
            &metrics
        );
    }

    let elapsed = start.elapsed();

    // Should be able to record 1000 operations quickly (< 100ms on most systems)
    assert!(elapsed < Duration::from_millis(100), "Performance overhead too high: {:?}", elapsed);

    // Verify all operations were recorded
    let snapshot = collector.snapshot();
    assert_eq!(snapshot.global.total_operations, 1000);

    // Verify metrics are reasonable
    assert!(snapshot.global.error_rate > 0.0 && snapshot.global.error_rate < 100.0);
    assert!(snapshot.global.average_response_time > Duration::ZERO);
}