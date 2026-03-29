// SPDX-License-Identifier: AGPL-3.0-only
use super::super::types::{AgentConfig, ModelConfig};
use super::*;
use std::collections::HashMap;
use std::time::SystemTime;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_agent_backend_deploy() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "test-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };

    let result = backend.deploy_agent(&config).await;
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.name, "test-agent");
    assert_eq!(info.status, AgentStatus::Running);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_agent_backend_lifecycle() {
    let backend = InMemoryAgentBackend::new();
    let config = AgentConfig {
        name: "lifecycle-agent".to_string(),
        model: "test-model".to_string(),
        capabilities: vec!["chat".to_string()],
        resources: None,
        environment: HashMap::new(),
        config: HashMap::new(),
    };

    // Deploy
    backend.deploy_agent(&config).await.unwrap();

    // Scale
    backend.scale_agent("lifecycle-agent", 3).await.unwrap();

    // Check status
    let status = backend.get_agent_status("lifecycle-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Running);

    // Stop
    backend.stop_agent("lifecycle-agent").await.unwrap();
    let status = backend.get_agent_status("lifecycle-agent").await.unwrap();
    assert_eq!(status, AgentStatus::Stopped);

    // Remove
    backend.remove_agent("lifecycle-agent").await.unwrap();

    // Verify removed
    let status_result = backend.get_agent_status("lifecycle-agent").await;
    assert!(status_result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_agent_backend_list() {
    let backend = InMemoryAgentBackend::new();

    // Initially empty
    let list = backend.list_agents().await.unwrap();
    assert_eq!(list.len(), 0);

    // Deploy agents
    for i in 1..=3 {
        let config = AgentConfig {
            name: format!("agent-{i}"),
            model: "test-model".to_string(),
            capabilities: vec!["chat".to_string()],
            resources: None,
            environment: HashMap::new(),
            config: HashMap::new(),
        };
        backend.deploy_agent(&config).await.unwrap();
    }

    // List should have 3 agents
    let list = backend.list_agents().await.unwrap();
    assert_eq!(list.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_scale_agent_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.scale_agent("nope", 2).await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_stop_agent_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.stop_agent("missing").await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_remove_agent_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.remove_agent("ghost").await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_get_agent_status_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.get_agent_status("none").await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_get_agent_resources_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.get_agent_resources("none").await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_unload_model_not_found() {
    let backend = InMemoryAgentBackend::new();
    let err = backend.unload_model("unknown-model").await.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_load_model_list_models_unload() {
    let backend = InMemoryAgentBackend::new();
    let cfg = ModelConfig {
        name: "m1".to_string(),
        model_type: "llm".to_string(),
        parameters: HashMap::new(),
        resources: None,
    };
    backend.load_model(&cfg).await.unwrap();
    let models = backend.list_models().await.unwrap();
    assert_eq!(models.len(), 1);
    assert_eq!(models[0].name, "m1");
    backend.unload_model("m1").await.unwrap();
    assert!(backend.list_models().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_initialize_default_ok() {
    let backend = InMemoryAgentBackend::new();
    backend.initialize().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_health_check_default_ok() {
    let backend = InMemoryAgentBackend::new();
    backend.health_check().await.unwrap();
}

#[test]
fn test_agent_info_serde_roundtrip() {
    let t = SystemTime::UNIX_EPOCH;
    let info = AgentInfo {
        name: "a".to_string(),
        agent_id: "id".to_string(),
        model: "m".to_string(),
        status: AgentStatus::Running,
        replicas: 2,
        capabilities: vec!["x".to_string()],
        resources: AgentResourceUsage {
            cpu_millicores: 500,
            memory_bytes: 1024,
            gpu_percent: Some(12.5),
            network_bytes_per_sec: 99,
        },
        created_at: t,
        last_updated: t,
    };
    let json = serde_json::to_string(&info).unwrap();
    let back: AgentInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, info.name);
    assert_eq!(back.status, info.status);
    assert_eq!(back.resources, info.resources);
}

#[test]
fn test_model_info_serde_roundtrip() {
    let t = SystemTime::UNIX_EPOCH;
    let mi = ModelInfo {
        name: "mod".to_string(),
        model_id: "mid".to_string(),
        model_type: "t".to_string(),
        size_bytes: 42,
        status: ModelStatus::Ready,
        resource_requirements: ModelResourceRequirements {
            min_cpu_cores: 1.0,
            min_memory_gb: 2.0,
            gpu_required: true,
            min_gpu_memory_gb: Some(4.0),
        },
        performance: ModelPerformanceMetrics {
            avg_inference_time_ms: 10,
            throughput_rps: 1.0,
            success_rate: 99.0,
        },
        loaded_at: t,
    };
    let json = serde_json::to_string(&mi).unwrap();
    let back: ModelInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, mi.name);
    assert_eq!(back.status, mi.status);
}

#[test]
fn test_agent_status_failed_variant() {
    let s = AgentStatus::Failed("oom".to_string());
    let j = serde_json::to_string(&s).unwrap();
    let back: AgentStatus = serde_json::from_str(&j).unwrap();
    assert_eq!(back, s);
}
