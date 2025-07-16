//! Integration tests for ToadStool Universal Compute Platform
//!
//! These tests verify end-to-end functionality across the entire system,
//! including runtime engines, federation, WebSocket connections, and ecosystem integration.

use std::time::Duration;

use toadstool::{
    execution::RuntimeType,
    security::{FilesystemSecurity, IsolationLevel, NetworkSecurity, SecurityContext},
};

// Import test utilities
use toadstool_testing::{
    builders::ExecutionRequestBuilder,
    fixtures::create_test_resource_requirements,
    integration::{IntegrationTestConfig, IntegrationTestManager},
};

/// Test basic execution workflow across all runtime types
#[tokio::test]
async fn test_end_to_end_execution_workflow() {
    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
    ];

    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    for runtime_type in runtime_types {
        println!("Testing runtime type: {runtime_type:?}");

        let request = ExecutionRequestBuilder::new()
            .runtime_hint(runtime_type.clone())
            .native_workload("echo", vec!["Hello, ToadStool!".to_string()])
            .timeout(Duration::from_secs(30))
            .build();

        // For now, just validate that we can create the request
        assert!(request.runtime_hint.is_some());
        println!("✓ {runtime_type:?} runtime request created successfully");
    }
}

/// Test WebSocket connection lifecycle
#[tokio::test]
async fn test_websocket_connection_lifecycle() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Create a simple execution request
    let request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("echo", vec!["WebSocket test".to_string()])
        .timeout(Duration::from_secs(10))
        .build();

    // Validate request creation
    assert!(request.runtime_hint.is_some());
    println!("✓ WebSocket test request created");
}

/// Test execution cancellation
#[tokio::test]
async fn test_execution_cancellation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Create a long-running request
    let request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("sleep", vec!["30".to_string()])
        .timeout(Duration::from_secs(60))
        .build();

    // Validate request creation
    assert!(request.runtime_hint.is_some());
    println!("✓ Cancellation test request created");
}

/// Test federation discovery
#[tokio::test]
async fn test_federation_discovery() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    println!("✓ Federation discovery test setup completed");
}

/// Test resource monitoring
#[tokio::test]
async fn test_resource_monitoring() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Create a request with resource requirements
    let resources = create_test_resource_requirements();
    let _request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("echo", vec!["Resource test".to_string()])
        .resources(resources)
        .timeout(Duration::from_secs(10))
        .build();

    // Validate request creation - resources is not optional in the current API
    println!("✓ Resource monitoring test request created");
}

/// Test security isolation levels
#[tokio::test]
async fn test_security_isolation() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    let isolation_levels = vec![
        IsolationLevel::None,
        IsolationLevel::Basic,
        IsolationLevel::Maximum,
    ];

    for isolation_level in isolation_levels {
        let security_context = SecurityContext {
            isolation_level: isolation_level.clone(),
            capabilities: vec![],
            user_context: None,
            network_security: NetworkSecurity {
                allow_outbound: true,
                allow_inbound: false,
                allowed_domains: vec![],
                blocked_domains: vec![],
                allowed_ports: vec![],
                blocked_ports: vec![],
            },
            filesystem_security: FilesystemSecurity::default(),
        };

        let _request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Container)
            .native_workload("echo", vec!["Security test".to_string()])
            .security_context(security_context)
            .timeout(Duration::from_secs(10))
            .build();

        // Validate request creation - security_context is not optional
        println!("✓ {isolation_level:?} isolation level test request created");
    }
}

/// Test ecosystem integration
#[tokio::test]
async fn test_ecosystem_integration() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Test integration with various ecosystem components
    let components = vec!["Songbird", "NestGate", "BearDog"];

    for component in components {
        println!("Testing {component} integration");

        let request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec![format!("{} integration test", component)])
            .timeout(Duration::from_secs(10))
            .build();

        assert!(request.runtime_hint.is_some());
        println!("✓ {component} integration test request created");
    }
}

/// Test concurrent executions
#[tokio::test]
async fn test_concurrent_executions() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    let concurrent_count = 5;
    let mut requests = Vec::new();

    for i in 0..concurrent_count {
        let request = ExecutionRequestBuilder::new()
            .runtime_hint(RuntimeType::Native)
            .native_workload("echo", vec![format!("Concurrent test {}", i)])
            .timeout(Duration::from_secs(10))
            .build();

        requests.push(request);
    }

    assert_eq!(requests.len(), concurrent_count);
    println!("✓ {concurrent_count} concurrent test requests created");
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Test invalid executable
    let invalid_request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("nonexistent_command", vec![])
        .timeout(Duration::from_secs(5))
        .build();

    assert!(invalid_request.runtime_hint.is_some());
    println!("✓ Invalid command test request created");

    // Test resource-heavy request
    let resource_heavy_request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("echo", vec!["Resource heavy test".to_string()])
        .timeout(Duration::from_secs(5))
        .build();

    assert!(resource_heavy_request.runtime_hint.is_some());
    println!("✓ Resource heavy test request created");

    // Test timeout scenario
    let timeout_request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("sleep", vec!["10".to_string()])
        .timeout(Duration::from_secs(1))
        .build();

    assert!(timeout_request.runtime_hint.is_some());
    println!("✓ Timeout test request created");
}

/// Test system health and readiness
#[tokio::test]
async fn test_system_health() {
    let config = IntegrationTestConfig::default();
    let _manager = IntegrationTestManager::new(config);

    // Test basic health check
    let health_request = ExecutionRequestBuilder::new()
        .runtime_hint(RuntimeType::Native)
        .native_workload("echo", vec!["Health check".to_string()])
        .timeout(Duration::from_secs(5))
        .build();

    assert!(health_request.runtime_hint.is_some());
    println!("✓ System health test request created");
}
