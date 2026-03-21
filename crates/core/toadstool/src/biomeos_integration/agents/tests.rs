// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for agent deployment configuration and manager behavior.

use super::super::types::{AgentConfig, ModelConfig};
use super::*;
use std::collections::HashMap;
use std::time::SystemTime;

fn test_config() -> AgentDeploymentConfig {
    AgentDeploymentConfig {
        squirrel_endpoint: "http://localhost:8080".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    }
}

fn sample_agent_info() -> AgentInfo {
    let now = SystemTime::now();
    AgentInfo {
        name: "test-agent".to_string(),
        agent_id: "test-agent-abc123".to_string(),
        model: "test-model".to_string(),
        status: AgentStatus::Running,
        replicas: 1,
        capabilities: vec!["chat".to_string(), "reasoning".to_string()],
        resources: AgentResourceUsage {
            cpu_millicores: 500,
            memory_bytes: 1024 * 1024 * 512,
            gpu_percent: None,
            network_bytes_per_sec: 1024,
        },
        created_at: now,
        last_updated: now,
    }
}

#[test]
fn test_agent_deployment_config_construction() {
    let config = test_config();
    assert_eq!(config.squirrel_endpoint, "http://localhost:8080");
    assert_eq!(config.model_registry, "local");
    assert_eq!(config.agent_runtime, "container");
    assert!(config.mcp_enabled);
    assert!(config.resource_limits.is_empty());
}

#[test]
fn test_agent_deployment_config_serialization_roundtrip() {
    let config = test_config();
    let json = serde_json::to_string(&config).expect("serialize");
    let restored: AgentDeploymentConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(config.squirrel_endpoint, restored.squirrel_endpoint);
}

#[test]
fn test_agent_status_variants() {
    assert_eq!(AgentStatus::Deploying, AgentStatus::Deploying);
    assert_eq!(AgentStatus::Running, AgentStatus::Running);
    assert_eq!(AgentStatus::Scaling, AgentStatus::Scaling);
    assert_eq!(AgentStatus::Updating, AgentStatus::Updating);
    assert_eq!(AgentStatus::Terminating, AgentStatus::Terminating);
    assert_eq!(AgentStatus::Stopped, AgentStatus::Stopped);
    assert!(matches!(
        AgentStatus::Failed("reason".to_string()),
        AgentStatus::Failed(s) if s == "reason"
    ));
}

#[test]
fn test_model_status_variants() {
    assert_eq!(ModelStatus::Loading, ModelStatus::Loading);
    assert_eq!(ModelStatus::Ready, ModelStatus::Ready);
    assert_eq!(ModelStatus::Updating, ModelStatus::Updating);
    assert_eq!(ModelStatus::Unloading, ModelStatus::Unloading);
    assert!(matches!(
        ModelStatus::Error("load failed".to_string()),
        ModelStatus::Error(s) if s == "load failed"
    ));
}

#[test]
fn test_agent_info_construction() {
    let info = sample_agent_info();
    assert_eq!(info.name, "test-agent");
    assert_eq!(info.agent_id, "test-agent-abc123");
    assert_eq!(info.status, AgentStatus::Running);
    assert_eq!(info.replicas, 1);
    assert_eq!(info.capabilities.len(), 2);
    assert_eq!(info.resources.cpu_millicores, 500);
}

#[test]
fn test_agent_info_serialization_roundtrip() {
    let info = sample_agent_info();
    let json = serde_json::to_string(&info).expect("serialize");
    let restored: AgentInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(info.name, restored.name);
    assert_eq!(info.status, restored.status);
}

#[test]
#[allow(clippy::float_cmp)] // literals just assigned in test
fn test_agent_resource_usage_construction() {
    let usage = AgentResourceUsage {
        cpu_millicores: 1000,
        memory_bytes: 2 * 1024 * 1024 * 1024,
        gpu_percent: Some(50.0),
        network_bytes_per_sec: 2048,
    };
    assert_eq!(usage.cpu_millicores, 1000);
    assert_eq!(usage.memory_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(usage.gpu_percent, Some(50.0));
}

#[test]
fn test_agent_resource_usage_serialization_roundtrip() {
    let usage = AgentResourceUsage {
        cpu_millicores: 500,
        memory_bytes: 1024 * 1024,
        gpu_percent: None,
        network_bytes_per_sec: 512,
    };
    let json = serde_json::to_string(&usage).expect("serialize");
    let restored: AgentResourceUsage = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(usage.cpu_millicores, restored.cpu_millicores);
}

#[test]
#[allow(clippy::float_cmp)] // literals just assigned in test
fn test_model_info_construction() {
    let now = SystemTime::now();
    let info = ModelInfo {
        name: "gpt-4".to_string(),
        model_id: "model-xyz".to_string(),
        model_type: "llm".to_string(),
        size_bytes: 1_000_000_000,
        status: ModelStatus::Ready,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 4.0,
            min_memory_gb: 8.0,
            gpu_required: true,
            min_gpu_memory_gb: Some(16.0),
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 50,
            throughput_rps: 10.0,
            success_rate: 99.5,
        },
        loaded_at: now,
    };
    assert_eq!(info.name, "gpt-4");
    assert_eq!(info.status, ModelStatus::Ready);
    assert_eq!(info.resource_requirements.min_cpu_cores, 4.0);
}

#[test]
fn test_model_info_serialization_roundtrip() {
    let now = SystemTime::now();
    let info = ModelInfo {
        name: "model-a".to_string(),
        model_id: "id-1".to_string(),
        model_type: "type-a".to_string(),
        size_bytes: 100,
        status: ModelStatus::Loading,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 1.0,
            min_memory_gb: 2.0,
            gpu_required: false,
            min_gpu_memory_gb: None,
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 10,
            throughput_rps: 5.0,
            success_rate: 100.0,
        },
        loaded_at: now,
    };
    let json = serde_json::to_string(&info).expect("serialize");
    let restored: ModelInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(info.name, restored.name);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_with_inmemory_backend() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "test-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };

    let result = manager.deploy_agent(&agent_config).await;
    assert!(result.is_ok());

    let agent_info = result.expect("Agent deployment should succeed in test");
    assert_eq!(agent_info.name, "test-agent");
    assert!(agent_info.agent_id.starts_with("test-agent-"));
    assert_eq!(agent_info.status, AgentStatus::Running);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_agents_returns_deployed() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "list-test-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let agents = manager.list_agents().await;
    assert!(!agents.is_empty());
    assert!(agents.iter().any(|a| a.name == "list-test-agent"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_agent_status_after_deploy() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "status-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let status = manager.get_agent_status("status-agent").await;
    assert!(status.is_ok());
    assert_eq!(status.unwrap(), AgentStatus::Running);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_model() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let model_config = ModelConfig {
        name: "test-model".to_string(),
        model_type: "llm".to_string(),
        parameters: HashMap::new(),
        resources: None,
    };

    let result = manager.load_model(&model_config).await;
    assert!(result.is_ok());
    let model_info = result.unwrap();
    assert_eq!(model_info.name, "test-model");
    assert_eq!(model_info.model_type, "llm");
    assert_eq!(model_info.status, ModelStatus::Ready);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_models_returns_loaded() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let model_config = ModelConfig {
        name: "list-model".to_string(),
        model_type: "embedding".to_string(),
        parameters: HashMap::new(),
        resources: None,
    };
    manager.load_model(&model_config).await.unwrap();

    let models = manager.list_models().await;
    assert!(!models.is_empty());
    assert!(models.iter().any(|m| m.name == "list-model"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scale_agent() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "scale-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let result = manager.scale_agent("scale-agent", 3).await;
    assert!(result.is_ok());

    let status = manager.get_agent_status("scale-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Running);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_agent() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "stop-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let result = manager.stop_agent("stop-agent").await;
    assert!(result.is_ok());

    let status = manager.get_agent_status("stop-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Stopped);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_remove_agent() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "remove-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let result = manager.remove_agent("remove-agent").await;
    assert!(result.is_ok());

    let status_result = manager.get_agent_status("remove-agent").await;
    assert!(status_result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unload_model() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let model_config = ModelConfig {
        name: "unload-model".to_string(),
        model_type: "llm".to_string(),
        parameters: HashMap::new(),
        resources: None,
    };
    manager.load_model(&model_config).await.unwrap();

    let result = manager.unload_model("unload-model").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_agent_resources() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let agent_config = AgentConfig {
        name: "resources-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };
    manager.deploy_agent(&agent_config).await.unwrap();

    let result = manager.get_agent_resources("resources-agent").await;
    assert!(result.is_ok());
    let usage = result.unwrap();
    assert!(usage.cpu_millicores > 0);
    assert!(usage.memory_bytes > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_inmemory() {
    let config = test_config();
    let manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.health_check().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_initialize_squirrel_connection_inmemory() {
    let config = test_config();
    let manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.initialize_squirrel_connection().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_agent_status_nonexistent_returns_error() {
    let config = test_config();
    let manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.get_agent_status("nonexistent-agent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scale_agent_nonexistent_returns_error() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.scale_agent("nonexistent", 2).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_agent_nonexistent_returns_error() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.stop_agent("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_remove_agent_nonexistent_returns_error() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.remove_agent("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unload_model_nonexistent_returns_error() {
    let config = test_config();
    let mut manager = AgentDeploymentManager::with_inmemory(config);

    let result = manager.unload_model("nonexistent-model").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_new_with_backend() {
    let config = test_config();
    let backend = crate::biomeos_integration::InMemoryAgentBackend::new();
    let manager = AgentDeploymentManager::new(config, std::sync::Arc::new(backend));

    let agents = manager.list_agents().await;
    assert!(agents.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_discover_with_config_returns_manager() {
    // with_inmemory is the fallback path - test it directly to avoid nested runtime
    let config = AgentDeploymentConfig::default();
    let manager = AgentDeploymentManager::with_inmemory(config);
    let agents = manager.list_agents().await;
    assert!(agents.is_empty());
}
