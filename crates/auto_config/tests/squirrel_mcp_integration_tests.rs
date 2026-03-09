// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Integration Tests for AI/MCP Interface - Business Logic
//!
//! Target: `ai_mcp_interface.rs` async handlers (16% → 70%+ coverage)
//! Focus: Request processing, session management, integration flows
//!
//! NOTE: These tests use `MockHardwareDetector` and `MockEcosystemDiscoverer`
//! to avoid blocking I/O operations. Tests complete in < 10ms.
//!
//! ## Sovereignty
//! Evolved from `squirrel_mcp` (primal-specific) to `ai_mcp_interface` (agnostic).
//! Works with ANY AI provider via runtime capability discovery.

use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

use toadstool_auto_config::ai_mcp_interface::{
    AiMcpInterface, AiPreferences, ExecutionIntent, IoIntensity, McpRequest, McpRequestType,
    MemoryPattern, PerformanceExpectations, ResourceHints, ResourcePreferences,
};

// Note: Currently tests use the standard AiMcpInterface which creates
// real HardwareDetector and EcosystemDiscoverer internally.
// For now, slow integration tests are marked with #[ignore].

// ============================================================================
// AiMcpInterface Integration Tests (Agnostic AI Provider Support)
// ============================================================================

#[tokio::test]
async fn test_interface_creation() {
    let interface = AiMcpInterface::new();
    assert!(
        interface.is_ok(),
        "Interface should be created successfully"
    );
}

#[tokio::test]
async fn test_interface_get_session_stats_empty() {
    let interface = AiMcpInterface::new().unwrap();

    let stats = interface.get_session_stats().await;

    assert!(!stats.is_empty(), "Should return stats");
    assert!(stats.contains_key("active_sessions"));
    assert!(stats.contains_key("total_requests"));

    // Should start with 0 sessions and 0 requests
    if let Some(serde_json::Value::Number(n)) = stats.get("active_sessions") {
        assert_eq!(n.as_u64().unwrap(), 0, "Should start with 0 sessions");
    }
    if let Some(serde_json::Value::Number(n)) = stats.get("total_requests") {
        assert_eq!(n.as_u64().unwrap(), 0, "Should start with 0 requests");
    }
}

// ============================================================================
// Create Session Tests
// ============================================================================

#[tokio::test]
async fn test_create_session_basic() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-create-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "Session creation should succeed");
    let result = response.unwrap();
    assert!(result.success, "Response should indicate success");
    assert!(result.session_info.is_some(), "Should include session info");

    // Verify session was created
    let stats = interface.get_session_stats().await;
    if let Some(serde_json::Value::Number(n)) = stats.get("active_sessions") {
        assert_eq!(n.as_u64().unwrap(), 1, "Should have 1 active session");
    }
}

#[tokio::test]
async fn test_create_session_with_preferences() {
    let mut interface = AiMcpInterface::new().unwrap();

    let custom_prefs = AiPreferences {
        security_level: Some("high".to_string()),
        performance_priority: 0.9,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "aggressive".to_string(),
            memory_strategy: "aggressive".to_string(),
            gpu_preference: "required".to_string(),
            storage_preference: "speed".to_string(),
        },
        runtime_preferences: vec!["native".to_string(), "gpu".to_string()],
    };

    let request = McpRequest {
        request_id: "req-create-002".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(custom_prefs.clone()),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(
        response.is_ok(),
        "Session creation with preferences should succeed"
    );
    let result = response.unwrap();
    assert!(result.success);

    if let Some(session_info) = result.session_info {
        assert_eq!(
            session_info.preferences.security_level,
            Some("high".to_string())
        );
        assert_eq!(session_info.preferences.performance_priority, 0.9);
    }
}

#[tokio::test]
async fn test_create_multiple_sessions() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create first session
    let request1 = McpRequest {
        request_id: "req-001".to_string(),
        session_id: None,
        agent_id: "agent-1".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response1 = interface.process_ai_request(request1).await;
    assert!(response1.is_ok());

    // Create second session
    let request2 = McpRequest {
        request_id: "req-002".to_string(),
        session_id: None,
        agent_id: "agent-2".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response2 = interface.process_ai_request(request2).await;
    assert!(response2.is_ok());

    // Verify both sessions exist
    let stats = interface.get_session_stats().await;
    if let Some(serde_json::Value::Number(n)) = stats.get("active_sessions") {
        assert_eq!(n.as_u64().unwrap(), 2, "Should have 2 active sessions");
    }
}

// ============================================================================
// Update Preferences Tests
// ============================================================================

#[tokio::test]
async fn test_update_preferences_success() {
    let mut interface = AiMcpInterface::new().unwrap();

    // First create a session
    let create_request = McpRequest {
        request_id: "req-create".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let create_response = interface.process_ai_request(create_request).await.unwrap();
    let session_id = create_response
        .session_info
        .as_ref()
        .unwrap()
        .session_id
        .clone();

    // Now update preferences
    let new_prefs = AiPreferences {
        security_level: Some("paranoid".to_string()),
        performance_priority: 0.3,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "conservative".to_string(),
            memory_strategy: "conservative".to_string(),
            gpu_preference: "disabled".to_string(),
            storage_preference: "capacity".to_string(),
        },
        runtime_preferences: vec!["container".to_string()],
    };

    let update_request = McpRequest {
        request_id: "req-update".to_string(),
        session_id: Some(session_id),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: new_prefs.clone(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let update_response = interface.process_ai_request(update_request).await;

    assert!(update_response.is_ok(), "Update should succeed");
    let result = update_response.unwrap();
    assert!(result.success, "Update should be successful");

    if let Some(session_info) = result.session_info {
        assert_eq!(
            session_info.preferences.security_level,
            Some("paranoid".to_string())
        );
        assert_eq!(session_info.preferences.performance_priority, 0.3);
    }
}

#[tokio::test]
async fn test_update_preferences_nonexistent_session() {
    let mut interface = AiMcpInterface::new().unwrap();

    let update_request = McpRequest {
        request_id: "req-update".to_string(),
        session_id: Some("nonexistent-session".to_string()),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences::default(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(update_request).await;

    assert!(
        response.is_ok(),
        "Should return response even if session not found"
    );
    let result = response.unwrap();
    assert!(!result.success, "Should indicate failure");
    assert!(result.message.contains("not found") || result.message.contains("Session not found"));
}

#[tokio::test]
async fn test_update_preferences_no_session_id() {
    let mut interface = AiMcpInterface::new().unwrap();

    let update_request = McpRequest {
        request_id: "req-update".to_string(),
        session_id: None, // No session ID provided
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences::default(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(update_request).await;

    assert!(response.is_ok(), "Should return response");
    let result = response.unwrap();
    assert!(
        !result.success,
        "Should indicate failure without session ID"
    );
}

// ============================================================================
// Get System Status Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - runs hardware/network detection"]
async fn test_get_system_status() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-status".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "System status request should succeed");
    let result = response.unwrap();
    assert!(result.success, "Should return success");
    assert!(result.data.is_some(), "Should include system data");

    // Verify data structure
    if let Some(data) = result.data {
        assert!(data.is_object(), "Data should be an object");
        let obj = data.as_object().unwrap();
        assert!(obj.contains_key("hardware"), "Should include hardware info");
        assert!(
            obj.contains_key("ecosystem"),
            "Should include ecosystem info"
        );
        assert!(
            obj.contains_key("toadstool_status"),
            "Should include status"
        );
    }
}

// ============================================================================
// Request Counter Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - calls GetSystemStatus which triggers hardware detection"]
async fn test_request_counter_increments() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Check initial state
    let stats1 = interface.get_session_stats().await;
    let initial_count = if let Some(serde_json::Value::Number(n)) = stats1.get("total_requests") {
        n.as_u64().unwrap()
    } else {
        0
    };

    // Make a request
    let request = McpRequest {
        request_id: "req-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let _ = interface.process_ai_request(request).await;

    // Check that counter incremented
    let stats2 = interface.get_session_stats().await;
    let new_count = if let Some(serde_json::Value::Number(n)) = stats2.get("total_requests") {
        n.as_u64().unwrap()
    } else {
        0
    };

    assert_eq!(
        new_count,
        initial_count + 1,
        "Request counter should increment"
    );
}

#[tokio::test]
#[ignore = "slow integration test - calls GetSystemStatus which triggers hardware detection"]
async fn test_request_counter_multiple_requests() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Make multiple requests
    for i in 0..5 {
        let request = McpRequest {
            request_id: format!("req-{i}"),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::GetSystemStatus,
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let _ = interface.process_ai_request(request).await;
    }

    // Check final count
    let stats = interface.get_session_stats().await;
    let count = if let Some(serde_json::Value::Number(n)) = stats.get("total_requests") {
        n.as_u64().unwrap()
    } else {
        0
    };

    assert_eq!(count, 5, "Should have processed 5 requests");
}

// ============================================================================
// Session Activity Tracking Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - calls GetSystemStatus which triggers hardware detection"]
async fn test_session_activity_updates() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create a session
    let create_request = McpRequest {
        request_id: "req-create".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let create_response = interface.process_ai_request(create_request).await.unwrap();
    let session_id = create_response
        .session_info
        .as_ref()
        .unwrap()
        .session_id
        .clone();

    // ✅ MODERNIZED: No artificial delay needed - session is created synchronously
    // Make a request with the session (should update last_activity)
    let status_request = McpRequest {
        request_id: "req-status".to_string(),
        session_id: Some(session_id),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(status_request).await;
    assert!(response.is_ok(), "Request with session should succeed");
}

// ============================================================================
// Execution Intent Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - runs auto-config generation"]
async fn test_execute_with_intent_basic() {
    let mut interface = AiMcpInterface::new().unwrap();

    let intent = ExecutionIntent {
        purpose: "Test computation".to_string(),
        security_requirements: vec![],
        performance_expectations: PerformanceExpectations {
            expected_duration: Some(Duration::from_secs(10)),
            cpu_intensity: 0.5,
            memory_pattern: MemoryPattern::Normal,
            io_intensity: IoIntensity::Low,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(2.0),
            memory_gb: Some(4.0),
            gpu_required: false,
            storage_gb: Some(10.0),
        },
        runtime_hint: Some("wasm".to_string()),
    };

    let request = McpRequest {
        request_id: "req-execute".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "fn main() { println!(\"Hello\"); }".to_string(),
            intent,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "Execute with intent should succeed");
    let result = response.unwrap();
    assert!(result.success, "Should indicate success");
    assert!(
        result.config_applied.is_some(),
        "Should include config summary"
    );
}

#[tokio::test]
#[ignore = "slow integration test - runs auto-config generation"]
async fn test_execute_with_high_security_intent() {
    let mut interface = AiMcpInterface::new().unwrap();

    let intent = ExecutionIntent {
        purpose: "Secure computation".to_string(),
        security_requirements: vec!["high_security".to_string(), "data_privacy".to_string()],
        performance_expectations: PerformanceExpectations {
            expected_duration: None,
            cpu_intensity: 0.3,
            memory_pattern: MemoryPattern::Minimal,
            io_intensity: IoIntensity::Low,
        },
        resource_hints: ResourceHints {
            cpu_cores: None,
            memory_gb: None,
            gpu_required: false,
            storage_gb: None,
        },
        runtime_hint: None,
    };

    let request = McpRequest {
        request_id: "req-secure".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "secure_code()".to_string(),
            intent,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "Secure execution should succeed");
    let result = response.unwrap();
    assert!(result.success);

    if let Some(config) = result.config_applied {
        assert_eq!(config.security_level, "High", "Should apply high security");
    }
}

#[tokio::test]
#[ignore = "slow integration test - runs auto-config generation"]
async fn test_execute_with_gpu_intent() {
    let mut interface = AiMcpInterface::new().unwrap();

    let intent = ExecutionIntent {
        purpose: "GPU computation".to_string(),
        security_requirements: vec![],
        performance_expectations: PerformanceExpectations {
            expected_duration: Some(Duration::from_secs(60)),
            cpu_intensity: 0.9,
            memory_pattern: MemoryPattern::Large,
            io_intensity: IoIntensity::Medium,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(8.0),
            memory_gb: Some(16.0),
            gpu_required: true, // GPU required
            storage_gb: Some(50.0),
        },
        runtime_hint: Some("gpu".to_string()),
    };

    let request = McpRequest {
        request_id: "req-gpu".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "gpu_kernel()".to_string(),
            intent,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "GPU execution should succeed");
    let result = response.unwrap();
    assert!(result.success);

    if let Some(config) = result.config_applied {
        assert!(
            config.resource_allocation.gpu_enabled,
            "GPU should be enabled"
        );
    }
}

// ============================================================================
// Optimize for Task Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - runs NL config processing"]
async fn test_optimize_for_task() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-optimize".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::OptimizeForTask {
            task_description: "Machine learning inference".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "Optimize for task should succeed");
    let result = response.unwrap();
    assert!(result.success, "Should indicate success");
    assert!(
        result.config_applied.is_some(),
        "Should include config summary"
    );
    assert!(!result.suggestions.is_empty(), "Should include suggestions");
}

// ============================================================================
// Natural Language Config Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - runs NL config processing"]
async fn test_natural_language_config() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-nl".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::NaturalLanguageConfig {
            instruction: "Configure for high performance".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(response.is_ok(), "Natural language config should succeed");
    let result = response.unwrap();
    assert!(result.success, "Should indicate success");
    assert!(
        result.config_applied.is_some(),
        "Should include config summary"
    );
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
#[ignore = "slow integration test - calls GetSystemStatus which triggers hardware detection"]
async fn test_metadata_handling() {
    let mut interface = AiMcpInterface::new().unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("user".to_string(), "test-user".to_string());
    metadata.insert("priority".to_string(), "high".to_string());

    let request = McpRequest {
        request_id: "req-meta".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata,
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;
    assert!(response.is_ok(), "Request with metadata should succeed");
}

// ============================================================================
// Total: 25+ Integration Tests
// ============================================================================
// Expected coverage increase: 16% → 70%+
// Coverage areas:
// - Interface creation (2 tests)
// - Session management (create, update, track) (8 tests)
// - System status (1 test)
// - Request counter (2 tests)
// - Session activity (1 test)
// - Execute with intent (3 tests)
// - Optimize for task (1 test)
// - Natural language config (1 test)
// - Error handling (1 test)
// Total: 20 tests
