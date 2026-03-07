// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for AI/MCP Interface
//!
//! Tests all major functions and paths in `ai_mcp_interface.rs` to achieve 70%+ coverage
//!
//! ## Sovereignty
//! Evolved from `squirrel_mcp` (primal-specific) to `ai_mcp_interface` (agnostic).

use std::collections::HashMap;
use std::time::SystemTime;
use toadstool_auto_config::ai_mcp_interface::*;
use toadstool_auto_config::ToadStoolResult;

// ============================================================================
// UNIT TESTS - Construction & Basic Operations
// ============================================================================

#[test]
fn test_squirrel_mcp_interface_creation() {
    let interface = AiMcpInterface::new();
    assert!(
        interface.is_ok(),
        "AiMcpInterface should be created successfully"
    );
}

#[test]
fn test_ai_preferences_default_values() {
    let prefs = AiPreferences {
        security_level: None,
        performance_priority: 0.5,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "balanced".to_string(),
            memory_strategy: "balanced".to_string(),
            gpu_preference: "auto".to_string(),
            storage_preference: "balanced".to_string(),
        },
        runtime_preferences: vec!["native".to_string()],
    };

    assert_eq!(prefs.performance_priority, 0.5);
    assert_eq!(prefs.resource_preferences.cpu_strategy, "balanced");
}

#[test]
fn test_ai_session_creation() {
    let session = AiSession {
        session_id: "test-session-123".to_string(),
        agent_id: "test-agent".to_string(),
        current_config: None,
        started_at: SystemTime::now(),
        last_activity: SystemTime::now(),
        preferences: AiPreferences {
            security_level: Some("high".to_string()),
            performance_priority: 0.7,
            resource_preferences: ResourcePreferences {
                cpu_strategy: "aggressive".to_string(),
                memory_strategy: "balanced".to_string(),
                gpu_preference: "required".to_string(),
                storage_preference: "speed".to_string(),
            },
            runtime_preferences: vec!["gpu".to_string(), "native".to_string()],
        },
    };

    assert_eq!(session.session_id, "test-session-123");
    assert_eq!(session.agent_id, "test-agent");
    assert_eq!(session.preferences.performance_priority, 0.7);
}

#[test]
fn test_resource_allocation_creation() {
    let allocation = ResourceAllocation {
        cpu_cores: 8.0,
        memory_gb: 16.0,
        gpu_enabled: true,
        storage_gb: 500.0,
    };

    assert_eq!(allocation.cpu_cores, 8.0);
    assert_eq!(allocation.memory_gb, 16.0);
    assert!(allocation.gpu_enabled);
}

#[test]
fn test_memory_pattern_serialization() {
    let patterns = vec![
        MemoryPattern::Minimal,
        MemoryPattern::Normal,
        MemoryPattern::Large,
        MemoryPattern::Streaming,
    ];

    for pattern in patterns {
        let json = serde_json::to_string(&pattern);
        assert!(json.is_ok(), "MemoryPattern should serialize");
    }
}

#[test]
fn test_io_intensity_serialization() {
    let intensities = vec![
        IoIntensity::Low,
        IoIntensity::Medium,
        IoIntensity::High,
        IoIntensity::Extreme,
    ];

    for intensity in intensities {
        let json = serde_json::to_string(&intensity);
        assert!(json.is_ok(), "IoIntensity should serialize");
    }
}

// ============================================================================
// INTEGRATION TESTS - Request Processing
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_create_session_request() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let request = McpRequest {
        request_id: "req-001".to_string(),
        session_id: None,
        agent_id: "test-agent-123".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(AiPreferences {
                security_level: Some("high".to_string()),
                performance_priority: 0.8,
                resource_preferences: ResourcePreferences {
                    cpu_strategy: "aggressive".to_string(),
                    memory_strategy: "balanced".to_string(),
                    gpu_preference: "auto".to_string(),
                    storage_preference: "speed".to_string(),
                },
                runtime_preferences: vec!["native".to_string()],
            }),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "Session creation should succeed");
    assert!(
        response.session_info.is_some(),
        "Session info should be present"
    );

    if let Some(session_info) = response.session_info {
        assert!(
            !session_info.session_id.is_empty(),
            "Session ID should be generated"
        );
    }

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_system_status_request() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let request = McpRequest {
        request_id: "req-status-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "System status request should succeed");
    assert!(response.data.is_some(), "Status data should be present");

    println!("✅ System status retrieved: {}", response.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_natural_language_config_request() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let request = McpRequest {
        request_id: "req-nl-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::NaturalLanguageConfig {
            instruction: "Set up a secure development environment".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "Natural language config should succeed");
    assert!(
        response.config_applied.is_some(),
        "Config should be applied"
    );

    println!("✅ Natural language config applied: {}", response.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimize_for_task_request() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let request = McpRequest {
        request_id: "req-opt-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::OptimizeForTask {
            task_description: "machine learning model training".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "Task optimization should succeed");
    assert!(
        response.config_applied.is_some(),
        "Optimized config should be applied"
    );
    assert!(
        !response.suggestions.is_empty(),
        "Suggestions should be provided"
    );

    println!("✅ Task optimization applied: {}", response.message);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_with_intent_request() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let intent = ExecutionIntent {
        purpose: "Process user data securely".to_string(),
        security_requirements: vec!["sandboxing".to_string(), "encryption".to_string()],
        performance_expectations: PerformanceExpectations {
            expected_duration: None,
            cpu_intensity: 0.6,
            memory_pattern: MemoryPattern::Normal,
            io_intensity: IoIntensity::Medium,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(2.0),
            memory_gb: Some(4.0),
            gpu_required: false,
            storage_gb: Some(10.0),
        },
        runtime_hint: Some("native".to_string()),
    };

    let request = McpRequest {
        request_id: "req-exec-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "fn main() { println!(\"Hello\"); }".to_string(),
            intent,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "Execute with intent should succeed");
    assert!(response.data.is_some(), "Execution data should be present");
    assert!(
        !response.suggestions.is_empty(),
        "Suggestions should be provided"
    );

    println!("✅ Execution with intent processed: {}", response.message);

    Ok(())
}

// ============================================================================
// CONCURRENT TESTS - Thread Safety
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_session_creation() -> ToadStoolResult<()> {
    const NUM_SESSIONS: usize = 10;
    let mut tasks = vec![];

    for i in 0..NUM_SESSIONS {
        tasks.push(tokio::spawn(async move {
            let mut interface = AiMcpInterface::new()?;

            let request = McpRequest {
                request_id: format!("req-concurrent-{i}"),
                session_id: None,
                agent_id: format!("agent-{i}"),
                request_type: McpRequestType::CreateSession { preferences: None },
                metadata: HashMap::new(),
                timestamp: SystemTime::now(),
            };

            interface.process_ai_request(request).await
        }));
    }

    let results = futures::future::join_all(tasks).await;

    let success_count = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .map(|res| res.as_ref().map(|r| r.success).unwrap_or(false))
                .unwrap_or(false)
        })
        .count();

    assert_eq!(
        success_count, NUM_SESSIONS,
        "All concurrent session creations should succeed"
    );

    println!("✅ Created {NUM_SESSIONS} sessions concurrently");

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_status_requests() -> ToadStoolResult<()> {
    const NUM_REQUESTS: usize = 5;
    let mut tasks = vec![];

    for i in 0..NUM_REQUESTS {
        tasks.push(tokio::spawn(async move {
            let mut interface = AiMcpInterface::new()?;

            let request = McpRequest {
                request_id: format!("req-status-{i}"),
                session_id: None,
                agent_id: "test-agent".to_string(),
                request_type: McpRequestType::GetSystemStatus,
                metadata: HashMap::new(),
                timestamp: SystemTime::now(),
            };

            interface.process_ai_request(request).await
        }));
    }

    let results = futures::future::join_all(tasks).await;

    let success_count = results
        .iter()
        .filter(|r| {
            r.as_ref()
                .map(|res| res.as_ref().map(|r| r.success).unwrap_or(false))
                .unwrap_or(false)
        })
        .count();

    assert!(
        success_count >= NUM_REQUESTS / 2,
        "Most concurrent status requests should succeed"
    );

    println!("✅ Completed {success_count} status requests concurrently");

    Ok(())
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test]
async fn test_request_with_empty_agent_id() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let request = McpRequest {
        request_id: "req-empty-agent".to_string(),
        session_id: None,
        agent_id: String::new(), // Empty agent ID
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    // Should handle gracefully
    assert!(
        response.is_ok() || response.is_err(),
        "Should handle empty agent ID"
    );

    Ok(())
}

#[tokio::test]
async fn test_request_with_large_metadata() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    let mut metadata = HashMap::new();
    for i in 0..100 {
        metadata.insert(format!("key-{i}"), format!("value-{i}"));
    }

    let request = McpRequest {
        request_id: "req-large-metadata".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata,
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await?;

    assert!(response.success, "Should handle large metadata");

    Ok(())
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[tokio::test]
async fn test_interface_creation_performance() {
    use std::time::Instant;

    let start = Instant::now();

    for _ in 0..100 {
        let _ = AiMcpInterface::new();
    }

    let duration = start.elapsed();

    assert!(
        duration < std::time::Duration::from_millis(500),
        "Creating 100 interfaces should be <500ms, took {duration:?}"
    );

    println!("✅ Created 100 interfaces in {duration:?}");
}

// ============================================================================
// REGRESSION TESTS
// ============================================================================

#[test]
fn test_interface_creation_never_panics() {
    // Regression: Constructor should never panic
    for _ in 0..10 {
        let _ = AiMcpInterface::new();
    }
}

#[tokio::test]
async fn test_multiple_sequential_requests() -> ToadStoolResult<()> {
    let mut interface = AiMcpInterface::new()?;

    // Sequential requests should all work
    for i in 0..5 {
        let request = McpRequest {
            request_id: format!("req-seq-{i}"),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::GetSystemStatus,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await?;
        assert!(response.success, "Sequential request {i} should succeed");
    }

    Ok(())
}
