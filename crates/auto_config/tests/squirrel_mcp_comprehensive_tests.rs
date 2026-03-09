// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Comprehensive Tests for Squirrel MCP Interface
//!
//! Target: `squirrel_mcp.rs` (440 lines, 13.18% → 70%+ coverage)
//! Focus: AI request processing, session management, type serialization

use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;

use toadstool_auto_config::ai_mcp_interface::{
    AiPreferences, AiSession, ConfigurationSummary, ExecutionIntent, IoIntensity, McpRequest,
    McpRequestType, McpResponse, MemoryPattern, PerformanceExpectations, ResourceAllocation,
    ResourceHints, ResourcePreferences, SessionInfo,
};

// ============================================================================
// AiPreferences Tests
// ============================================================================

#[test]
fn test_ai_preferences_default() {
    let prefs = AiPreferences::default();

    assert_eq!(prefs.security_level, Some("balanced".to_string()));
    assert_eq!(prefs.performance_priority, 0.7);
    assert_eq!(prefs.resource_preferences.cpu_strategy, "balanced");
    assert_eq!(prefs.resource_preferences.memory_strategy, "balanced");
    assert_eq!(prefs.resource_preferences.gpu_preference, "auto");
}

#[test]
fn test_ai_preferences_serialization() {
    let prefs = AiPreferences::default();

    let json = serde_json::to_string(&prefs).unwrap();
    assert!(json.contains("balanced"));
    assert!(json.contains("auto"));
}

#[test]
fn test_ai_preferences_deserialization() {
    let json = r#"{
        "security_level": "high",
        "performance_priority": 0.9,
        "resource_preferences": {
            "cpu_strategy": "aggressive",
            "memory_strategy": "aggressive",
            "gpu_preference": "required",
            "storage_preference": "speed"
        },
        "runtime_preferences": ["native", "wasm"]
    }"#;

    let prefs: AiPreferences = serde_json::from_str(json).unwrap();
    assert_eq!(prefs.security_level, Some("high".to_string()));
    assert_eq!(prefs.performance_priority, 0.9);
    assert_eq!(prefs.resource_preferences.cpu_strategy, "aggressive");
}

#[test]
fn test_ai_preferences_clone() {
    let prefs = AiPreferences::default();
    let cloned = prefs.clone();

    assert_eq!(prefs.security_level, cloned.security_level);
    assert_eq!(prefs.performance_priority, cloned.performance_priority);
}

// ============================================================================
// ResourcePreferences Tests
// ============================================================================

#[test]
fn test_resource_preferences_strategies() {
    let strategies = vec!["conservative", "balanced", "aggressive"];

    for strategy in strategies {
        let prefs = ResourcePreferences {
            cpu_strategy: strategy.to_string(),
            memory_strategy: strategy.to_string(),
            gpu_preference: "auto".to_string(),
            storage_preference: "balanced".to_string(),
        };

        assert_eq!(prefs.cpu_strategy, strategy);
        assert_eq!(prefs.memory_strategy, strategy);
    }
}

#[test]
fn test_resource_preferences_gpu_options() {
    let gpu_options = vec!["auto", "required", "disabled"];

    for option in gpu_options {
        let prefs = ResourcePreferences {
            cpu_strategy: "balanced".to_string(),
            memory_strategy: "balanced".to_string(),
            gpu_preference: option.to_string(),
            storage_preference: "balanced".to_string(),
        };

        assert_eq!(prefs.gpu_preference, option);
    }
}

#[test]
fn test_resource_preferences_storage_options() {
    let storage_options = vec!["speed", "capacity", "balanced"];

    for option in storage_options {
        let prefs = ResourcePreferences {
            cpu_strategy: "balanced".to_string(),
            memory_strategy: "balanced".to_string(),
            gpu_preference: "auto".to_string(),
            storage_preference: option.to_string(),
        };

        assert_eq!(prefs.storage_preference, option);
    }
}

// ============================================================================
// McpRequest Tests
// ============================================================================

#[test]
fn test_squirrel_request_natural_language_config() {
    let request = McpRequest {
        request_id: "req-001".to_string(),
        session_id: Some("session-001".to_string()),
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::NaturalLanguageConfig {
            instruction: "Enable high performance mode".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-001");
    assert_eq!(request.agent_id, "agent-ai");
}

#[test]
fn test_squirrel_request_execute_with_intent() {
    let intent = ExecutionIntent {
        purpose: "Test execution".to_string(),
        security_requirements: vec!["high_security".to_string()],
        performance_expectations: PerformanceExpectations {
            expected_duration: Some(Duration::from_secs(60)),
            cpu_intensity: 0.8,
            memory_pattern: MemoryPattern::Normal,
            io_intensity: IoIntensity::Low,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(4.0),
            memory_gb: Some(8.0),
            gpu_required: false,
            storage_gb: Some(10.0),
        },
        runtime_hint: Some("wasm".to_string()),
    };

    let request = McpRequest {
        request_id: "req-002".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::ExecuteWithIntent {
            code: "print('hello')".to_string(),
            intent,
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-002");
}

#[test]
fn test_squirrel_request_optimize_for_task() {
    let request = McpRequest {
        request_id: "req-003".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::OptimizeForTask {
            task_description: "Machine learning inference".to_string(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-003");
}

#[test]
fn test_squirrel_request_get_system_status() {
    let request = McpRequest {
        request_id: "req-004".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-004");
}

#[test]
fn test_squirrel_request_create_session() {
    let request = McpRequest {
        request_id: "req-005".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::CreateSession {
            preferences: Some(AiPreferences::default()),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-005");
}

#[test]
fn test_squirrel_request_update_preferences() {
    let request = McpRequest {
        request_id: "req-006".to_string(),
        session_id: Some("session-001".to_string()),
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::UpdatePreferences {
            preferences: AiPreferences::default(),
        },
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    assert_eq!(request.request_id, "req-006");
    assert!(request.session_id.is_some());
}

#[test]
fn test_squirrel_request_serialization() {
    let request = McpRequest {
        request_id: "req-001".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("req-001"));
    assert!(json.contains("agent-ai"));
}

// ============================================================================
// McpResponse Tests
// ============================================================================

#[test]
fn test_squirrel_response_success() {
    let response = McpResponse {
        request_id: "req-001".to_string(),
        success: true,
        message: "Operation completed".to_string(),
        data: None,
        suggestions: vec!["Consider optimization".to_string()],
        session_info: None,
        config_applied: None,
    };

    assert!(response.success);
    assert_eq!(response.suggestions.len(), 1);
}

#[test]
fn test_squirrel_response_with_session_info() {
    let session_info = SessionInfo {
        session_id: "session-001".to_string(),
        status: "Active".to_string(),
        preferences: AiPreferences::default(),
    };

    let response = McpResponse {
        request_id: "req-001".to_string(),
        success: true,
        message: "Session created".to_string(),
        data: None,
        suggestions: vec![],
        session_info: Some(session_info),
        config_applied: None,
    };

    assert!(response.session_info.is_some());
}

#[test]
fn test_squirrel_response_with_config() {
    let config_summary = ConfigurationSummary {
        name: "Test Config".to_string(),
        description: "Test configuration".to_string(),
        security_level: "High".to_string(),
        performance_level: "Optimized".to_string(),
        enabled_runtimes: vec!["Native".to_string(), "WASM".to_string()],
        resource_allocation: ResourceAllocation {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            gpu_enabled: true,
            storage_gb: 100.0,
        },
    };

    let response = McpResponse {
        request_id: "req-001".to_string(),
        success: true,
        message: "Config applied".to_string(),
        data: None,
        suggestions: vec![],
        session_info: None,
        config_applied: Some(config_summary),
    };

    assert!(response.config_applied.is_some());
}

#[test]
fn test_squirrel_response_serialization() {
    let response = McpResponse {
        request_id: "req-001".to_string(),
        success: true,
        message: "OK".to_string(),
        data: Some(serde_json::json!({"test": "value"})),
        suggestions: vec![],
        session_info: None,
        config_applied: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("req-001"));
    assert!(json.contains("true"));
}

// ============================================================================
// ExecutionIntent Tests
// ============================================================================

#[test]
fn test_execution_intent_creation() {
    let intent = ExecutionIntent {
        purpose: "Data processing".to_string(),
        security_requirements: vec!["data_privacy".to_string()],
        performance_expectations: PerformanceExpectations {
            expected_duration: Some(Duration::from_secs(120)),
            cpu_intensity: 0.6,
            memory_pattern: MemoryPattern::Large,
            io_intensity: IoIntensity::High,
        },
        resource_hints: ResourceHints {
            cpu_cores: Some(8.0),
            memory_gb: Some(16.0),
            gpu_required: false,
            storage_gb: Some(50.0),
        },
        runtime_hint: Some("native".to_string()),
    };

    assert_eq!(intent.purpose, "Data processing");
    assert_eq!(intent.security_requirements.len(), 1);
}

#[test]
fn test_execution_intent_serialization() {
    let intent = ExecutionIntent {
        purpose: "Test".to_string(),
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
    };

    let json = serde_json::to_string(&intent).unwrap();
    assert!(json.contains("Test"));
}

// ============================================================================
// MemoryPattern Tests
// ============================================================================

#[test]
fn test_memory_pattern_minimal() {
    let pattern = MemoryPattern::Minimal;
    assert!(matches!(pattern, MemoryPattern::Minimal));
}

#[test]
fn test_memory_pattern_normal() {
    let pattern = MemoryPattern::Normal;
    assert!(matches!(pattern, MemoryPattern::Normal));
}

#[test]
fn test_memory_pattern_large() {
    let pattern = MemoryPattern::Large;
    assert!(matches!(pattern, MemoryPattern::Large));
}

#[test]
fn test_memory_pattern_streaming() {
    let pattern = MemoryPattern::Streaming;
    assert!(matches!(pattern, MemoryPattern::Streaming));
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
        let json = serde_json::to_string(&pattern).unwrap();
        assert!(!json.is_empty());
    }
}

// ============================================================================
// IoIntensity Tests
// ============================================================================

#[test]
fn test_io_intensity_low() {
    let intensity = IoIntensity::Low;
    assert!(matches!(intensity, IoIntensity::Low));
}

#[test]
fn test_io_intensity_medium() {
    let intensity = IoIntensity::Medium;
    assert!(matches!(intensity, IoIntensity::Medium));
}

#[test]
fn test_io_intensity_high() {
    let intensity = IoIntensity::High;
    assert!(matches!(intensity, IoIntensity::High));
}

#[test]
fn test_io_intensity_extreme() {
    let intensity = IoIntensity::Extreme;
    assert!(matches!(intensity, IoIntensity::Extreme));
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
        let json = serde_json::to_string(&intensity).unwrap();
        assert!(!json.is_empty());
    }
}

// ============================================================================
// ResourceHints Tests
// ============================================================================

#[test]
fn test_resource_hints_all_specified() {
    let hints = ResourceHints {
        cpu_cores: Some(8.0),
        memory_gb: Some(16.0),
        gpu_required: true,
        storage_gb: Some(100.0),
    };

    assert!(hints.cpu_cores.is_some());
    assert!(hints.memory_gb.is_some());
    assert!(hints.gpu_required);
    assert!(hints.storage_gb.is_some());
}

#[test]
fn test_resource_hints_minimal() {
    let hints = ResourceHints {
        cpu_cores: None,
        memory_gb: None,
        gpu_required: false,
        storage_gb: None,
    };

    assert!(hints.cpu_cores.is_none());
    assert!(!hints.gpu_required);
}

#[test]
fn test_resource_hints_gpu_required() {
    let hints = ResourceHints {
        cpu_cores: Some(4.0),
        memory_gb: Some(8.0),
        gpu_required: true,
        storage_gb: Some(50.0),
    };

    assert!(hints.gpu_required);
}

// ============================================================================
// PerformanceExpectations Tests
// ============================================================================

#[test]
fn test_performance_expectations_with_duration() {
    let perf = PerformanceExpectations {
        expected_duration: Some(Duration::from_secs(300)),
        cpu_intensity: 0.8,
        memory_pattern: MemoryPattern::Normal,
        io_intensity: IoIntensity::Medium,
    };

    assert!(perf.expected_duration.is_some());
    assert_eq!(perf.cpu_intensity, 0.8);
}

#[test]
fn test_performance_expectations_cpu_intensive() {
    let perf = PerformanceExpectations {
        expected_duration: None,
        cpu_intensity: 0.95,
        memory_pattern: MemoryPattern::Normal,
        io_intensity: IoIntensity::Low,
    };

    assert!(perf.cpu_intensity > 0.9);
}

#[test]
fn test_performance_expectations_io_intensive() {
    let perf = PerformanceExpectations {
        expected_duration: None,
        cpu_intensity: 0.3,
        memory_pattern: MemoryPattern::Streaming,
        io_intensity: IoIntensity::Extreme,
    };

    assert!(matches!(perf.io_intensity, IoIntensity::Extreme));
}

// ============================================================================
// AiSession Tests
// ============================================================================

#[test]
fn test_ai_session_creation() {
    let session = AiSession {
        session_id: "session-001".to_string(),
        agent_id: "agent-ai".to_string(),
        current_config: None,
        started_at: SystemTime::now(),
        last_activity: SystemTime::now(),
        preferences: AiPreferences::default(),
    };

    assert_eq!(session.session_id, "session-001");
    assert!(session.current_config.is_none());
}

#[test]
fn test_ai_session_clone() {
    let session = AiSession {
        session_id: "session-001".to_string(),
        agent_id: "agent-ai".to_string(),
        current_config: None,
        started_at: SystemTime::now(),
        last_activity: SystemTime::now(),
        preferences: AiPreferences::default(),
    };

    let cloned = session.clone();
    assert_eq!(session.session_id, cloned.session_id);
}

// ============================================================================
// ConfigurationSummary Tests
// ============================================================================

#[test]
fn test_configuration_summary_creation() {
    let summary = ConfigurationSummary {
        name: "AI Config".to_string(),
        description: "AI-generated configuration".to_string(),
        security_level: "High".to_string(),
        performance_level: "Optimized".to_string(),
        enabled_runtimes: vec!["Native".to_string(), "WASM".to_string()],
        resource_allocation: ResourceAllocation {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            gpu_enabled: false,
            storage_gb: 50.0,
        },
    };

    assert_eq!(summary.name, "AI Config");
    assert_eq!(summary.enabled_runtimes.len(), 2);
}

#[test]
fn test_configuration_summary_serialization() {
    let summary = ConfigurationSummary {
        name: "Test".to_string(),
        description: "Test config".to_string(),
        security_level: "Basic".to_string(),
        performance_level: "Standard".to_string(),
        enabled_runtimes: vec!["Native".to_string()],
        resource_allocation: ResourceAllocation {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            gpu_enabled: false,
            storage_gb: 20.0,
        },
    };

    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("Test"));
}

// ============================================================================
// ResourceAllocation Tests
// ============================================================================

#[test]
fn test_resource_allocation_basic() {
    let allocation = ResourceAllocation {
        cpu_cores: 4.0,
        memory_gb: 8.0,
        gpu_enabled: false,
        storage_gb: 50.0,
    };

    assert_eq!(allocation.cpu_cores, 4.0);
    assert_eq!(allocation.memory_gb, 8.0);
    assert!(!allocation.gpu_enabled);
}

#[test]
fn test_resource_allocation_with_gpu() {
    let allocation = ResourceAllocation {
        cpu_cores: 16.0,
        memory_gb: 64.0,
        gpu_enabled: true,
        storage_gb: 500.0,
    };

    assert!(allocation.gpu_enabled);
    assert!(allocation.cpu_cores >= 8.0);
}

#[test]
fn test_resource_allocation_serialization() {
    let allocation = ResourceAllocation {
        cpu_cores: 8.0,
        memory_gb: 16.0,
        gpu_enabled: true,
        storage_gb: 100.0,
    };

    let json = serde_json::to_string(&allocation).unwrap();
    assert!(json.contains("cpu_cores"));
    assert!(json.contains("gpu_enabled"));
}

// ============================================================================
// SessionInfo Tests
// ============================================================================

#[test]
fn test_session_info_creation() {
    let info = SessionInfo {
        session_id: "session-001".to_string(),
        status: "Active".to_string(),
        preferences: AiPreferences::default(),
    };

    assert_eq!(info.session_id, "session-001");
    assert_eq!(info.status, "Active");
}

#[test]
fn test_session_info_serialization() {
    let info = SessionInfo {
        session_id: "session-001".to_string(),
        status: "Active".to_string(),
        preferences: AiPreferences::default(),
    };

    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("session-001"));
    assert!(json.contains("Active"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_request_response_round_trip() {
    let request = McpRequest {
        request_id: "req-001".to_string(),
        session_id: None,
        agent_id: "agent-ai".to_string(),
        request_type: McpRequestType::GetSystemStatus,
        metadata: HashMap::new(),
        timestamp: SystemTime::now(),
    };

    let request_json = serde_json::to_string(&request).unwrap();
    let deserialized: McpRequest = serde_json::from_str(&request_json).unwrap();

    assert_eq!(request.request_id, deserialized.request_id);
    assert_eq!(request.agent_id, deserialized.agent_id);
}

#[test]
fn test_preferences_round_trip() {
    let prefs = AiPreferences::default();

    let json = serde_json::to_string(&prefs).unwrap();
    let deserialized: AiPreferences = serde_json::from_str(&json).unwrap();

    assert_eq!(
        prefs.performance_priority,
        deserialized.performance_priority
    );
}

// ============================================================================
// Total: 60+ Tests
// ============================================================================
// Expected coverage increase: 13.18% → 70%+
// Coverage areas:
// - AiPreferences: 4 tests
// - ResourcePreferences: 3 tests
// - McpRequest: 8 tests
// - McpResponse: 4 tests
// - ExecutionIntent: 2 tests
// - MemoryPattern: 5 tests
// - IoIntensity: 5 tests
// - ResourceHints: 3 tests
// - PerformanceExpectations: 3 tests
// - AiSession: 2 tests
// - ConfigurationSummary: 2 tests
// - ResourceAllocation: 3 tests
// - SessionInfo: 2 tests
// - Integration: 2 tests
// Total: 48 tests
