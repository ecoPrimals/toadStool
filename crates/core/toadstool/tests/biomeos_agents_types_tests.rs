// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for BiomeOS agents integration types

use std::time::SystemTime;
use toadstool::biomeos_integration::{
    AgentDeploymentConfig, AgentInfo, AgentResourceUsage, AgentStatus, ModelInfo,
    ModelPerformanceMetrics, ModelResourceRequirements, ModelStatus,
};

// ============================================================================
// AgentDeploymentConfig Tests
// ============================================================================

#[test]
fn test_agent_deployment_config_creation() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7000".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.squirrel_endpoint, "http://localhost:7000");
    assert_eq!(config.model_registry, "huggingface");
    assert!(config.mcp_enabled);
}

#[test]
fn test_agent_deployment_config_with_local_registry() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7000".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.model_registry, "local");
}

#[test]
fn test_agent_deployment_config_with_lambda_runtime() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7000".to_string(),
        model_registry: "custom".to_string(),
        agent_runtime: "lambda".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    assert_eq!(config.agent_runtime, "lambda");
}

#[test]
fn test_agent_deployment_config_serialization() {
    let config = AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:7000".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("squirrel_endpoint"));
}

// ============================================================================
// AgentStatus Tests
// ============================================================================

#[test]
fn test_agent_status_deploying() {
    let status = AgentStatus::Deploying;
    assert_eq!(status, AgentStatus::Deploying);
}

#[test]
fn test_agent_status_running() {
    let status = AgentStatus::Running;
    assert_eq!(status, AgentStatus::Running);
}

#[test]
fn test_agent_status_scaling() {
    let status = AgentStatus::Scaling;
    assert_eq!(status, AgentStatus::Scaling);
}

#[test]
fn test_agent_status_updating() {
    let status = AgentStatus::Updating;
    assert_eq!(status, AgentStatus::Updating);
}

#[test]
fn test_agent_status_terminating() {
    let status = AgentStatus::Terminating;
    assert_eq!(status, AgentStatus::Terminating);
}

#[test]
fn test_agent_status_failed() {
    let status = AgentStatus::Failed("Connection error".to_string());
    match status {
        AgentStatus::Failed(msg) => assert_eq!(msg, "Connection error"),
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_agent_status_stopped() {
    let status = AgentStatus::Stopped;
    assert_eq!(status, AgentStatus::Stopped);
}

// ============================================================================
// AgentInfo Tests
// ============================================================================

#[test]
fn test_agent_info_creation() {
    let agent = AgentInfo {
        name: "test-agent".to_string(),
        agent_id: "agent-123".to_string(),
        model: "gpt-4".to_string(),
        status: AgentStatus::Running,
        replicas: 3,
        capabilities: vec!["text-generation".to_string(), "code-review".to_string()],
        resources: AgentResourceUsage {
            cpu_millicores: 2000,
            memory_bytes: 4_294_967_296,
            gpu_percent: Some(50.0),
            network_bytes_per_sec: 1_000_000,
        },
        created_at: SystemTime::now(),
        last_updated: SystemTime::now(),
    };

    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.replicas, 3);
    assert_eq!(agent.capabilities.len(), 2);
}

#[test]
fn test_agent_info_with_single_replica() {
    let agent = AgentInfo {
        name: "single-agent".to_string(),
        agent_id: "agent-456".to_string(),
        model: "claude-3".to_string(),
        status: AgentStatus::Running,
        replicas: 1,
        capabilities: vec!["chat".to_string()],
        resources: AgentResourceUsage {
            cpu_millicores: 1000,
            memory_bytes: 2_147_483_648,
            gpu_percent: None,
            network_bytes_per_sec: 500_000,
        },
        created_at: SystemTime::now(),
        last_updated: SystemTime::now(),
    };

    assert_eq!(agent.replicas, 1);
}

#[test]
fn test_agent_info_serialization() {
    let agent = AgentInfo {
        name: "test-agent".to_string(),
        agent_id: "agent-123".to_string(),
        model: "gpt-4".to_string(),
        status: AgentStatus::Running,
        replicas: 2,
        capabilities: vec!["chat".to_string()],
        resources: AgentResourceUsage {
            cpu_millicores: 2000,
            memory_bytes: 4_000_000_000,
            gpu_percent: None,
            network_bytes_per_sec: 1_000_000,
        },
        created_at: SystemTime::now(),
        last_updated: SystemTime::now(),
    };

    let json = serde_json::to_string(&agent).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("test-agent"));
}

// ============================================================================
// ModelStatus Tests
// ============================================================================

#[test]
fn test_model_status_loading() {
    let status = ModelStatus::Loading;
    assert_eq!(status, ModelStatus::Loading);
}

#[test]
fn test_model_status_ready() {
    let status = ModelStatus::Ready;
    assert_eq!(status, ModelStatus::Ready);
}

#[test]
fn test_model_status_updating() {
    let status = ModelStatus::Updating;
    assert_eq!(status, ModelStatus::Updating);
}

#[test]
fn test_model_status_unloading() {
    let status = ModelStatus::Unloading;
    assert_eq!(status, ModelStatus::Unloading);
}

#[test]
fn test_model_status_error() {
    let status = ModelStatus::Error("Out of memory".to_string());
    match status {
        ModelStatus::Error(msg) => assert_eq!(msg, "Out of memory"),
        _ => panic!("Expected Error status"),
    }
}

// ============================================================================
// ModelInfo Tests
// ============================================================================

#[test]
fn test_model_info_creation() {
    let model = ModelInfo {
        name: "gpt-4".to_string(),
        model_id: "model-123".to_string(),
        model_type: "transformer".to_string(),
        size_bytes: 13_000_000_000,
        status: ModelStatus::Ready,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 4.0,
            min_memory_gb: 16.0,
            gpu_required: true,
            min_gpu_memory_gb: Some(8.0),
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 100,
            throughput_rps: 10.0,
            success_rate: 95.0,
        },
        loaded_at: SystemTime::now(),
    };

    assert_eq!(model.name, "gpt-4");
    assert_eq!(model.model_type, "transformer");
    assert!(model.size_bytes > 0);
}

#[test]
fn test_model_info_with_small_model() {
    let model = ModelInfo {
        name: "tiny-llama".to_string(),
        model_id: "model-456".to_string(),
        model_type: "llm".to_string(),
        size_bytes: 1_000_000_000,
        status: ModelStatus::Ready,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 2.0,
            min_memory_gb: 4.0,
            gpu_required: false,
            min_gpu_memory_gb: None,
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 50,
            throughput_rps: 20.0,
            success_rate: 98.5,
        },
        loaded_at: SystemTime::now(),
    };

    assert!(!model.resource_requirements.gpu_required);
}

#[test]
fn test_model_info_serialization() {
    let model = ModelInfo {
        name: "claude-3".to_string(),
        model_id: "model-789".to_string(),
        model_type: "assistant".to_string(),
        size_bytes: 10_000_000_000,
        status: ModelStatus::Ready,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 4.0,
            min_memory_gb: 16.0,
            gpu_required: true,
            min_gpu_memory_gb: Some(10.0),
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 80,
            throughput_rps: 12.5,
            success_rate: 96.8,
        },
        loaded_at: SystemTime::now(),
    };

    let json = serde_json::to_string(&model).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("claude-3"));
}

// ============================================================================
// AgentResourceUsage Tests
// ============================================================================

#[test]
fn test_agent_resource_usage_creation() {
    let usage = AgentResourceUsage {
        cpu_millicores: 4000,
        memory_bytes: 8_589_934_592,
        gpu_percent: Some(75.0),
        network_bytes_per_sec: 10_000_000,
    };

    assert_eq!(usage.cpu_millicores, 4000);
    assert_eq!(usage.memory_bytes, 8_589_934_592);
    assert_eq!(usage.gpu_percent, Some(75.0));
}

#[test]
fn test_agent_resource_usage_no_gpu() {
    let usage = AgentResourceUsage {
        cpu_millicores: 2000,
        memory_bytes: 4_294_967_296,
        gpu_percent: None,
        network_bytes_per_sec: 1_000_000,
    };

    assert!(usage.gpu_percent.is_none());
}

// ============================================================================
// ModelResourceRequirements Tests
// ============================================================================

#[test]
fn test_model_resource_requirements_with_gpu() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 4.0,
        min_memory_gb: 16.0,
        gpu_required: true,
        min_gpu_memory_gb: Some(11.0),
    };

    assert!(requirements.gpu_required);
    assert!(requirements.min_gpu_memory_gb.is_some());
}

#[test]
fn test_model_resource_requirements_without_gpu() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 2.0,
        min_memory_gb: 4.0,
        gpu_required: false,
        min_gpu_memory_gb: None,
    };

    assert!(!requirements.gpu_required);
    assert!(requirements.min_gpu_memory_gb.is_none());
}
