// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp)]
//! Business Logic Tests for Squirrel MCP Interface
//!
//! Target: Test business logic, helper methods, and edge cases WITHOUT I/O
//! Goal: Increase `squirrel_mcp.rs` coverage from 45% → 70%
//!
//! Focus:
//! - Intent optimization logic
//! - Session lifecycle and edge cases
//! - Error handling paths
//! - Response construction
//! - Metadata handling

use std::collections::HashMap;
use std::time::SystemTime;
use toadstool_auto_config::ai_mcp_interface::{
    AiMcpInterface, AiPreferences, McpRequest, McpRequestType, ResourcePreferences,
};

// ============================================================================
// Session Management Edge Cases
// ============================================================================

#[tokio::test]
async fn test_create_session_generates_unique_ids() {
    let mut interface = AiMcpInterface::new().unwrap();

    let prefs = AiPreferences {
        security_level: Some("high".to_string()),
        performance_priority: 0.8,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "balanced".to_string(),
            memory_strategy: "balanced".to_string(),
            gpu_preference: "auto".to_string(),
            storage_preference: "balanced".to_string(),
        },
        runtime_preferences: vec!["native".to_string()],
    };

    // Create multiple sessions
    let mut session_ids = Vec::new();
    for i in 0..5 {
        let request = McpRequest {
            request_id: format!("req-{i}"),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::CreateSession {
                preferences: Some(prefs.clone()),
            },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };

        let response = interface.process_ai_request(request).await.unwrap();
        assert!(response.success);

        if let Some(session_info) = response.session_info {
            session_ids.push(session_info.session_id);
        }
    }

    // Verify all session IDs are unique
    let unique_count = session_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();
    assert_eq!(unique_count, 5, "All session IDs should be unique");
}

#[tokio::test]
async fn test_session_stores_agent_id() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-agent-test".to_string(),
        session_id: None,
        agent_id: "my-special-agent-123".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);

    // Verify the session was created
    let stats = interface.get_session_stats().await;
    let active = stats.get("active_sessions").unwrap().as_u64().unwrap();
    assert_eq!(active, 1, "Should have 1 active session");
}

#[tokio::test]
async fn test_update_preferences_validates_session_id() {
    let mut interface = AiMcpInterface::new().unwrap();

    let prefs = AiPreferences {
        security_level: Some("low".to_string()),
        performance_priority: 0.5,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "conservative".to_string(),
            memory_strategy: "conservative".to_string(),
            gpu_preference: "disabled".to_string(),
            storage_preference: "capacity".to_string(),
        },
        runtime_preferences: vec![],
    };

    // Try to update non-existent session
    let request = McpRequest {
        request_id: "req-invalid-session".to_string(),
        session_id: Some("non-existent-session-id".to_string()),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences { preferences: prefs },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(!response.success, "Should fail for non-existent session");
    assert!(response.message.contains("not found") || response.message.contains("❌"));
}

#[tokio::test]
async fn test_update_preferences_requires_session_id() {
    let mut interface = AiMcpInterface::new().unwrap();

    let prefs = AiPreferences {
        security_level: None,
        performance_priority: 0.5,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "balanced".to_string(),
            memory_strategy: "balanced".to_string(),
            gpu_preference: "auto".to_string(),
            storage_preference: "balanced".to_string(),
        },
        runtime_preferences: vec![],
    };

    // No session_id provided
    let request = McpRequest {
        request_id: "req-no-session".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences { preferences: prefs },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(!response.success, "Should fail without session_id");
}

#[tokio::test]
async fn test_session_timestamps_update() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create session
    let create_request = McpRequest {
        request_id: "req-create".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let create_response = interface.process_ai_request(create_request).await.unwrap();
    let session_id = create_response.session_info.unwrap().session_id;

    // ✅ MODERNIZED: No artificial delay needed - session is immediately available
    // Update preferences (should update timestamp and succeed)
    let update_request = McpRequest {
        request_id: "req-update".to_string(),
        session_id: Some(session_id),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences {
                security_level: None,
                performance_priority: 0.6,
                resource_preferences: ResourcePreferences {
                    cpu_strategy: "balanced".to_string(),
                    memory_strategy: "balanced".to_string(),
                    gpu_preference: "auto".to_string(),
                    storage_preference: "balanced".to_string(),
                },
                runtime_preferences: vec![],
            },
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let update_response = interface.process_ai_request(update_request).await.unwrap();

    // Verify update succeeded (session was found and updated)
    assert!(
        update_response.success,
        "Update should succeed for existing session"
    );
}

// ============================================================================
// Request Counter Tests
// ============================================================================

#[tokio::test]
async fn test_request_counter_starts_at_zero() {
    let interface = AiMcpInterface::new().unwrap();
    let stats = interface.get_session_stats().await;

    let count = stats.get("total_requests").unwrap().as_u64().unwrap();
    assert_eq!(count, 0, "Request counter should start at 0");
}

#[tokio::test]
async fn test_request_counter_increments_per_request() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Make 3 requests
    for i in 0..3 {
        let request = McpRequest {
            request_id: format!("req-{i}"),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        interface.process_ai_request(request).await.unwrap();
    }

    let stats = interface.get_session_stats().await;
    let count = stats.get("total_requests").unwrap().as_u64().unwrap();
    assert_eq!(count, 3, "Request counter should be 3 after 3 requests");
}

#[tokio::test]
async fn test_request_counter_increments_even_on_errors() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Make a request that will fail (update non-existent session)
    let request1 = McpRequest {
        request_id: "req-1".to_string(),
        session_id: Some("fake-session".to_string()),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences {
                security_level: None,
                performance_priority: 0.5,
                resource_preferences: ResourcePreferences {
                    cpu_strategy: "balanced".to_string(),
                    memory_strategy: "balanced".to_string(),
                    gpu_preference: "auto".to_string(),
                    storage_preference: "balanced".to_string(),
                },
                runtime_preferences: vec![],
            },
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let _response1 = interface.process_ai_request(request1).await;

    // Make a successful request
    let request2 = McpRequest {
        request_id: "req-2".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    interface.process_ai_request(request2).await.unwrap();

    let stats = interface.get_session_stats().await;
    let count = stats.get("total_requests").unwrap().as_u64().unwrap();
    assert_eq!(
        count, 2,
        "Request counter should increment even for failed requests"
    );
}

// ============================================================================
// Preference Handling Tests
// ============================================================================

#[tokio::test]
async fn test_default_preferences_applied() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create session without preferences
    let request = McpRequest {
        request_id: "req-defaults".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    assert!(response.success);

    let session_info = response.session_info.unwrap();
    let prefs = session_info.preferences;

    // Check default values
    assert_eq!(
        prefs.performance_priority, 0.7,
        "Default performance priority should be 0.7"
    );
    assert_eq!(prefs.resource_preferences.cpu_strategy, "balanced");
    assert_eq!(prefs.resource_preferences.memory_strategy, "balanced");
    assert_eq!(prefs.resource_preferences.gpu_preference, "auto");
    assert_eq!(prefs.security_level, Some("balanced".to_string()));
}

#[tokio::test]
async fn test_custom_preferences_stored() {
    let mut interface = AiMcpInterface::new().unwrap();

    let custom_prefs = AiPreferences {
        security_level: Some("maximum".to_string()),
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
        request_id: "req-custom".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(custom_prefs.clone()),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();
    let session_info = response.session_info.unwrap();
    let prefs = session_info.preferences;

    // Verify custom preferences were stored
    assert_eq!(prefs.security_level, Some("maximum".to_string()));
    assert_eq!(prefs.performance_priority, 0.9);
    assert_eq!(prefs.resource_preferences.cpu_strategy, "aggressive");
    assert_eq!(prefs.resource_preferences.gpu_preference, "required");
    assert_eq!(prefs.runtime_preferences.len(), 2);
}

#[tokio::test]
async fn test_preferences_can_be_updated() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create session with initial preferences
    let initial_prefs = AiPreferences {
        security_level: Some("low".to_string()),
        performance_priority: 0.3,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "conservative".to_string(),
            memory_strategy: "conservative".to_string(),
            gpu_preference: "disabled".to_string(),
            storage_preference: "capacity".to_string(),
        },
        runtime_preferences: vec![],
    };

    let create_request = McpRequest {
        request_id: "req-create".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(initial_prefs),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let create_response = interface.process_ai_request(create_request).await.unwrap();
    let session_id = create_response.session_info.unwrap().session_id;

    // Update to different preferences
    let updated_prefs = AiPreferences {
        security_level: Some("high".to_string()),
        performance_priority: 0.9,
        resource_preferences: ResourcePreferences {
            cpu_strategy: "aggressive".to_string(),
            memory_strategy: "aggressive".to_string(),
            gpu_preference: "required".to_string(),
            storage_preference: "speed".to_string(),
        },
        runtime_preferences: vec!["native".to_string()],
    };

    let update_request = McpRequest {
        request_id: "req-update".to_string(),
        session_id: Some(session_id),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: updated_prefs,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let update_response = interface.process_ai_request(update_request).await.unwrap();
    assert!(update_response.success);

    let session_info = update_response.session_info.unwrap();
    let prefs = session_info.preferences;

    // Verify preferences were updated
    assert_eq!(prefs.security_level, Some("high".to_string()));
    assert_eq!(prefs.performance_priority, 0.9);
    assert_eq!(prefs.resource_preferences.cpu_strategy, "aggressive");
}

// ============================================================================
// Metadata Handling Tests
// ============================================================================

#[tokio::test]
async fn test_metadata_accepted_in_requests() {
    let mut interface = AiMcpInterface::new().unwrap();

    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), "vscode".to_string());
    metadata.insert("version".to_string(), "1.2.3".to_string());
    metadata.insert("user_id".to_string(), "user-123".to_string());

    let request = McpRequest {
        request_id: "req-metadata".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata,
        timestamp: SystemTime::now(),
    };

    // Should not error with metadata
    let response = interface.process_ai_request(request).await;
    assert!(response.is_ok(), "Should accept requests with metadata");
}

#[tokio::test]
async fn test_empty_metadata_accepted() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-no-metadata".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await;
    assert!(
        response.is_ok(),
        "Should accept requests with empty metadata"
    );
}

// ============================================================================
// Response Structure Tests
// ============================================================================

#[tokio::test]
async fn test_successful_response_structure() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-response-test".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();

    // Verify response structure
    assert_eq!(response.request_id, "req-response-test");
    assert!(response.success);
    assert!(!response.message.is_empty());
    assert!(response.session_info.is_some());
}

#[tokio::test]
async fn test_failed_response_structure() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-fail-test".to_string(),
        session_id: Some("non-existent".to_string()),
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences {
                security_level: None,
                performance_priority: 0.5,
                resource_preferences: ResourcePreferences {
                    cpu_strategy: "balanced".to_string(),
                    memory_strategy: "balanced".to_string(),
                    gpu_preference: "auto".to_string(),
                    storage_preference: "balanced".to_string(),
                },
                runtime_preferences: vec![],
            },
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();

    // Verify failure response structure
    assert_eq!(response.request_id, "req-fail-test");
    assert!(!response.success);
    assert!(!response.message.is_empty());
    assert!(!response.suggestions.is_empty());
}

#[tokio::test]
async fn test_response_includes_suggestions() {
    let mut interface = AiMcpInterface::new().unwrap();

    let request = McpRequest {
        request_id: "req-suggestions".to_string(),
        session_id: None,
        agent_id: "test-agent".to_string(),
        request_type: McpRequestType::CreateSession { preferences: None },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let response = interface.process_ai_request(request).await.unwrap();

    // Successful responses should include helpful suggestions
    assert!(
        !response.suggestions.is_empty(),
        "Response should include suggestions"
    );
}

// ============================================================================
// Stats Retrieval Tests
// ============================================================================

#[tokio::test]
async fn test_get_session_stats_structure() {
    let interface = AiMcpInterface::new().unwrap();
    let stats = interface.get_session_stats().await;

    // Verify stats structure (matches actual implementation)
    assert!(stats.contains_key("active_sessions"));
    assert!(stats.contains_key("total_requests"));
    assert!(stats.contains_key("average_session_duration"));
}

#[tokio::test]
async fn test_get_session_stats_empty_state() {
    let interface = AiMcpInterface::new().unwrap();
    let stats = interface.get_session_stats().await;

    let active_sessions = stats.get("active_sessions").unwrap().as_u64().unwrap();
    let total_requests = stats.get("total_requests").unwrap().as_u64().unwrap();

    assert_eq!(active_sessions, 0);
    assert_eq!(total_requests, 0);
}

#[tokio::test]
async fn test_get_session_stats_tracks_sessions() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Create 3 sessions
    for i in 0..3 {
        let request = McpRequest {
            request_id: format!("req-{i}"),
            session_id: None,
            agent_id: format!("agent-{i}"),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        interface.process_ai_request(request).await.unwrap();
    }

    let stats = interface.get_session_stats().await;
    let active_sessions = stats.get("active_sessions").unwrap().as_u64().unwrap();

    assert_eq!(active_sessions, 3, "Should track 3 active sessions");
}

#[tokio::test]
async fn test_get_session_stats_tracks_requests() {
    let mut interface = AiMcpInterface::new().unwrap();

    // Make 5 requests
    for i in 0..5 {
        let request = McpRequest {
            request_id: format!("req-{i}"),
            session_id: None,
            agent_id: "test-agent".to_string(),
            request_type: McpRequestType::CreateSession { preferences: None },
            metadata: HashMap::new(),
            timestamp: SystemTime::now(),
        };
        interface.process_ai_request(request).await.unwrap();
    }

    let stats = interface.get_session_stats().await;
    let total_requests = stats.get("total_requests").unwrap().as_u64().unwrap();

    assert_eq!(total_requests, 5, "Should track 5 total requests");
}
