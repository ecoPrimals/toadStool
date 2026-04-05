// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for BiomeOS agent deployment integration types

use toadstool::biomeos_integration::*;

// ============================================================================
// AgentDeploymentConfig Tests
// ============================================================================

#[test]
fn test_agent_deployment_config_creation() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.ai_processing_endpoint, "http://localhost:7070");
    assert_eq!(config.model_registry, "huggingface");
    assert_eq!(config.agent_runtime, "container");
    assert!(config.mcp_enabled);
}

#[test]
fn test_agent_deployment_config_clone() {
    let config1 = AgentDeploymentConfig {
        ai_processing_endpoint: "http://squirrel:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };
    
    let config2 = config1.clone();
    
    assert_eq!(config1.ai_processing_endpoint, config2.ai_processing_endpoint);
    assert_eq!(config1.model_registry, config2.model_registry);
}

// ============================================================================
// AgentStatus Tests (7 variants)
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
    let status = AgentStatus::Failed("Deployment error".to_string());
    
    match status {
        AgentStatus::Failed(msg) => {
            assert_eq!(msg, "Deployment error");
        }
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_agent_status_stopped() {
    let status = AgentStatus::Stopped;
    assert_eq!(status, AgentStatus::Stopped);
}

#[test]
fn test_agent_status_clone() {
    let status1 = AgentStatus::Running;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// ModelStatus Tests (5 variants)
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
    let status = ModelStatus::Error("Model not found".to_string());
    
    match status {
        ModelStatus::Error(msg) => {
            assert_eq!(msg, "Model not found");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_model_status_clone() {
    let status1 = ModelStatus::Ready;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// AgentResourceUsage Tests
// ============================================================================

#[test]
fn test_agent_resource_usage_creation() {
    let usage = AgentResourceUsage {
        cpu_millicores: 500,
        memory_bytes: 1024 * 1024 * 512, // 512 MB
        gpu_percent: Some(25.5),
        network_bytes_per_sec: 1024 * 100, // 100 KB/s
    };
    
    assert_eq!(usage.cpu_millicores, 500);
    assert_eq!(usage.memory_bytes, 1024 * 1024 * 512);
    assert_eq!(usage.gpu_percent, Some(25.5));
    assert_eq!(usage.network_bytes_per_sec, 1024 * 100);
}

#[test]
fn test_agent_resource_usage_no_gpu() {
    let usage = AgentResourceUsage {
        cpu_millicores: 1000,
        memory_bytes: 1024 * 1024 * 1024, // 1 GB
        gpu_percent: None,
        network_bytes_per_sec: 0,
    };
    
    assert!(usage.gpu_percent.is_none());
}

#[test]
fn test_agent_resource_usage_clone() {
    let usage1 = AgentResourceUsage {
        cpu_millicores: 750,
        memory_bytes: 1024 * 1024 * 256,
        gpu_percent: Some(50.0),
        network_bytes_per_sec: 1024,
    };
    
    let usage2 = usage1.clone();
    
    assert_eq!(usage1.cpu_millicores, usage2.cpu_millicores);
    assert_eq!(usage1.memory_bytes, usage2.memory_bytes);
}

// ============================================================================
// ModelResourceRequirements Tests
// ============================================================================

#[test]
fn test_model_resource_requirements_creation() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 2.0,
        min_memory_gb: 4.0,
        gpu_required: true,
        min_gpu_memory_gb: Some(8.0),
    };
    
    assert_eq!(requirements.min_cpu_cores, 2.0);
    assert_eq!(requirements.min_memory_gb, 4.0);
    assert!(requirements.gpu_required);
    assert_eq!(requirements.min_gpu_memory_gb, Some(8.0));
}

#[test]
fn test_model_resource_requirements_no_gpu() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 1.0,
        min_memory_gb: 2.0,
        gpu_required: false,
        min_gpu_memory_gb: None,
    };
    
    assert!(!requirements.gpu_required);
    assert!(requirements.min_gpu_memory_gb.is_none());
}

#[test]
fn test_model_resource_requirements_minimal() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 0.5,
        min_memory_gb: 0.5,
        gpu_required: false,
        min_gpu_memory_gb: None,
    };
    
    assert_eq!(requirements.min_cpu_cores, 0.5);
    assert_eq!(requirements.min_memory_gb, 0.5);
}

#[test]
fn test_model_resource_requirements_large() {
    let requirements = ModelResourceRequirements {
        min_cpu_cores: 16.0,
        min_memory_gb: 64.0,
        gpu_required: true,
        min_gpu_memory_gb: Some(24.0),
    };
    
    assert_eq!(requirements.min_cpu_cores, 16.0);
    assert_eq!(requirements.min_memory_gb, 64.0);
    assert_eq!(requirements.min_gpu_memory_gb, Some(24.0));
}

// ============================================================================
// ModelPerformanceMetrics Tests
// ============================================================================

#[test]
fn test_model_performance_metrics_creation() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 250,
        throughput_rps: 10.5,
        success_rate: 99.8,
    };
    
    assert_eq!(metrics.avg_inference_time_ms, 250);
    assert_eq!(metrics.throughput_rps, 10.5);
    assert_eq!(metrics.success_rate, 99.8);
}

#[test]
fn test_model_performance_metrics_fast() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 10,
        throughput_rps: 100.0,
        success_rate: 100.0,
    };
    
    assert_eq!(metrics.avg_inference_time_ms, 10);
    assert_eq!(metrics.throughput_rps, 100.0);
}

#[test]
fn test_model_performance_metrics_slow() {
    let metrics = ModelPerformanceMetrics {
        avg_inference_time_ms: 5000,
        throughput_rps: 0.2,
        success_rate: 95.0,
    };
    
    assert_eq!(metrics.avg_inference_time_ms, 5000);
    assert_eq!(metrics.throughput_rps, 0.2);
}

// ============================================================================
// AgentDeploymentManager Tests
// ============================================================================

#[test]
fn test_agent_deployment_manager_creation() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    let _manager = AgentDeploymentManager::new(config);
    // Creation should succeed
}

#[test]
fn test_agent_deployment_manager_with_local_registry() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };
    
    let _manager = AgentDeploymentManager::new(config);
    // Creation should succeed
}

// ============================================================================
// Model Registry Tests
// ============================================================================

#[test]
fn test_model_registry_huggingface() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "huggingface".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.model_registry, "huggingface");
}

#[test]
fn test_model_registry_local() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.model_registry, "local");
}

#[test]
fn test_model_registry_custom() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "custom".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.model_registry, "custom");
}

// ============================================================================
// Agent Runtime Tests
// ============================================================================

#[test]
fn test_agent_runtime_container() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.agent_runtime, "container");
}

#[test]
fn test_agent_runtime_process() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.agent_runtime, "process");
}

#[test]
fn test_agent_runtime_lambda() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "lambda".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert_eq!(config.agent_runtime, "lambda");
}

// ============================================================================
// MCP (Model Control Protocol) Tests
// ============================================================================

#[test]
fn test_mcp_enabled() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "container".to_string(),
        mcp_enabled: true,
        resource_limits: serde_json::Map::new(),
    };
    
    assert!(config.mcp_enabled);
}

#[test]
fn test_mcp_disabled() {
    let config = AgentDeploymentConfig {
        ai_processing_endpoint: "http://localhost:7070".to_string(),
        model_registry: "local".to_string(),
        agent_runtime: "process".to_string(),
        mcp_enabled: false,
        resource_limits: serde_json::Map::new(),
    };
    
    assert!(!config.mcp_enabled);
}

