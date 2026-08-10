// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::collections::HashMap;
use std::time::SystemTime;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligence_mcp_interface_creation() {
    let interface = AiMcpInterface::new();
    assert!(interface.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg_attr(
    not(feature = "slow-tests"),
    ignore = "slow integration test - runs full NL processing and hardware detection"
)]
async fn test_natural_language_config_request() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "test-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::NaturalLanguageConfig {
            instruction: "Enable high performance mode".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;

    assert!(
        response.is_ok(),
        "AI request should succeed: {:?}",
        response.as_ref().err()
    );
    let result = response.unwrap();
    assert!(result.success, "Should return success response");
    assert!(
        !result.message.is_empty(),
        "Should return non-empty message"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_natural_language_config_request_fast() {
    // Fast test that checks interface state without full NL pipeline
    let interface = AiMcpInterface::new().unwrap();

    // This should be fast as it only checks interface state
    let stats = interface.get_session_stats().await;
    assert!(!stats.is_empty(), "Should return stats");

    // Verify expected keys exist
    assert!(stats.contains_key("active_sessions"));
    assert!(stats.contains_key("total_requests"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligence_mcp_context_handling() {
    let interface = AiMcpInterface::new().unwrap();
    let stats = interface.get_session_stats().await;

    // Check that stats are returned and active sessions is 0
    assert!(!stats.is_empty(), "Should return stats");
    if let Some(serde_json::Value::Number(n)) = stats.get("active_sessions") {
        assert_eq!(
            n.as_u64().unwrap(),
            0,
            "Should start with no active sessions"
        );
    } else {
        unreachable!("active_sessions key not found or wrong type");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_create_session() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "create-session-001".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.message.contains("session created"));
    assert!(response.session_info.is_some());
    let session_info = response.session_info.unwrap();
    assert!(!session_info.session_id.is_empty());
    assert_eq!(session_info.status, "Active");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_create_session_with_preferences() {
    let mut interface = AiMcpInterface::new().unwrap();
    let prefs = AiPreferences::default();

    let request = McpRequest {
        request_id: "create-002".to_string(),
        session_id: None,
        agent_id: "agent-2".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(prefs),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.session_info.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_update_preferences_session_not_found() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "update-001".to_string(),
        session_id: Some("nonexistent-session".to_string()),
        agent_id: "agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences::default(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(!response.success);
    assert!(response.message.contains("Session not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_update_preferences_success() {
    let mut interface = AiMcpInterface::new().unwrap();

    // First create a session
    let create_req = McpRequest {
        request_id: "create".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let create_resp = interface.process_ai_request(create_req).await.unwrap();
    let session_id = create_resp
        .session_info
        .as_ref()
        .unwrap()
        .session_id
        .clone();

    // Now update preferences
    let prefs = AiPreferences::default();
    let request = McpRequest {
        request_id: "update".to_string(),
        session_id: Some(session_id),
        agent_id: "agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: prefs.clone(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.message.contains("Preferences updated"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_get_system_status() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "status-001".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.data.is_some());
    let data = response.data.unwrap();
    assert!(data.get("hardware").is_some());
    assert!(data.get("ecosystem").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_optimize_for_task() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "optimize-001".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::OptimizeForTask {
            task_description: "run machine learning training".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.config_applied.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_execute_with_intent() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "execute-001".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "print('hello')".to_string(),
            intent: ExecutionIntent {
                purpose: "test execution".to_string(),
                security_requirements: vec![],
                performance_expectations: PerformanceExpectations {
                    expected_duration: None,
                    cpu_intensity: 0.5,
                    memory_pattern: MemoryPattern::Normal,
                    io_intensity: IoIntensity::Low,
                },
                resource_hints: ResourceHints {
                    cpu_cores: None,
                    memory_gb: None,
                    gpu_required: false,
                    storage_gb: None,
                },
                runtime_hint: None,
            },
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);
    assert!(response.config_applied.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_increments_counter() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-1".to_string(),
        session_id: None,
        agent_id: "agent".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let _ = interface.process_ai_request(request).await.unwrap();

    let stats = interface.get_session_stats().await;
    let total = stats
        .get("total_requests")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    assert!(total >= 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_process_ai_request_updates_session_activity() {
    let mut interface = AiMcpInterface::new().unwrap();

    let create_req = McpRequest {
        request_id: "c1".to_string(),
        session_id: None,
        agent_id: "a".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let create_resp = interface.process_ai_request(create_req).await.unwrap();
    let session_id = create_resp
        .session_info
        .as_ref()
        .unwrap()
        .session_id
        .clone();

    let status_req = McpRequest {
        request_id: "s1".to_string(),
        session_id: Some(session_id.clone()),
        agent_id: "a".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };
    let _ = interface.process_ai_request(status_req).await.unwrap();

    let stats = interface.get_session_stats().await;
    assert!(
        stats
            .get("active_sessions")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            >= 1
    );
}
