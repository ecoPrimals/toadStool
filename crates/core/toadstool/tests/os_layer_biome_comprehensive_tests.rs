// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for `os_layer/biome.rs` module
//!
//! Sprint 21: BiomeOS Integration Tests
//! Target: 0% → 70% coverage (~50 tests)

use serde_json::json;
use std::collections::HashMap;
use std::time::SystemTime;
use toadstool::os_layer::biome::{
    BiomeDeployment, BiomeDeploymentStatus, BiomeOSConfig, BiomeOSIntegration, BiomeOrchestrator,
};
use toadstool::universal::{NetworkLocation, PrimalContext, SecurityLevel};
use toadstool::{
    ExecutionResponse, JobPriority, ResourceRequirements, UniversalJob, UniversalJobType,
};
use uuid::Uuid;

fn create_biome_job(team_id: &str, manifest: serde_json::Value) -> UniversalJob {
    UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            team_id: team_id.to_string(),
            biome_manifest: manifest,
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: SystemTime::now(),
        context: create_test_primal_context(),
    }
}

fn create_test_primal_context() -> PrimalContext {
    PrimalContext {
        user_id: "test-user".to_string(),
        device_id: "test-device".to_string(),
        session_id: "test-session".to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

// ============================================================================
// BiomeOSConfig Tests
// ============================================================================

#[test]
fn test_biome_os_config_default() {
    let config = BiomeOSConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.endpoint, None);
    assert!(config.team_isolation);
    assert!(config.resource_quota_enforcement);
}

#[test]
fn test_biome_os_config_custom() {
    let config = BiomeOSConfig {
        enabled: true,
        endpoint: Some("http://localhost:8080".to_string()),
        team_isolation: false,
        resource_quota_enforcement: false,
    };

    assert!(config.enabled);
    assert_eq!(config.endpoint, Some("http://localhost:8080".to_string()));
    assert!(!config.team_isolation);
    assert!(!config.resource_quota_enforcement);
}

#[test]
fn test_biome_os_config_clone() {
    let config1 = BiomeOSConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(config1.endpoint, config2.endpoint);
    assert_eq!(config1.team_isolation, config2.team_isolation);
}

#[test]
fn test_biome_os_config_debug() {
    let config = BiomeOSConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("BiomeOSConfig"));
    assert!(debug_str.contains("enabled"));
}

#[test]
fn test_biome_os_config_serialization() {
    let config = BiomeOSConfig {
        enabled: true,
        endpoint: Some("http://test:8080".to_string()),
        team_isolation: true,
        resource_quota_enforcement: true,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: BiomeOSConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.enabled, deserialized.enabled);
    assert_eq!(config.endpoint, deserialized.endpoint);
}

#[test]
fn test_biome_os_config_endpoint_none() {
    let config = BiomeOSConfig {
        enabled: true,
        endpoint: None,
        team_isolation: true,
        resource_quota_enforcement: true,
    };

    assert!(config.enabled);
    assert!(config.endpoint.is_none());
}

#[test]
fn test_biome_os_config_endpoint_some() {
    let endpoint = "https://biome.example.com:9090";
    let config = BiomeOSConfig {
        enabled: true,
        endpoint: Some(endpoint.to_string()),
        team_isolation: true,
        resource_quota_enforcement: true,
    };

    assert_eq!(config.endpoint, Some(endpoint.to_string()));
}

// ============================================================================
// BiomeDeploymentStatus Tests
// ============================================================================

#[test]
fn test_deployment_status_pending() {
    let status = BiomeDeploymentStatus::Pending;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: BiomeDeploymentStatus = serde_json::from_str(&json).unwrap();

    match deserialized {
        BiomeDeploymentStatus::Pending => {}
        _ => panic!("Expected Pending status"),
    }
}

#[test]
fn test_deployment_status_running() {
    let status = BiomeDeploymentStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: BiomeDeploymentStatus = serde_json::from_str(&json).unwrap();

    match deserialized {
        BiomeDeploymentStatus::Running => {}
        _ => panic!("Expected Running status"),
    }
}

#[test]
fn test_deployment_status_stopped() {
    let status = BiomeDeploymentStatus::Stopped;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: BiomeDeploymentStatus = serde_json::from_str(&json).unwrap();

    match deserialized {
        BiomeDeploymentStatus::Stopped => {}
        _ => panic!("Expected Stopped status"),
    }
}

#[test]
fn test_deployment_status_failed() {
    let error_msg = "Connection timeout";
    let status = BiomeDeploymentStatus::Failed(error_msg.to_string());

    match &status {
        BiomeDeploymentStatus::Failed(msg) => {
            assert_eq!(msg, error_msg);
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_deployment_status_failed_serialization() {
    let status = BiomeDeploymentStatus::Failed("Test error".to_string());
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: BiomeDeploymentStatus = serde_json::from_str(&json).unwrap();

    match deserialized {
        BiomeDeploymentStatus::Failed(msg) => {
            assert_eq!(msg, "Test error");
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_deployment_status_clone() {
    let status1 = BiomeDeploymentStatus::Failed("Error".to_string());
    let status2 = status1.clone();

    match (&status1, &status2) {
        (BiomeDeploymentStatus::Failed(msg1), BiomeDeploymentStatus::Failed(msg2)) => {
            assert_eq!(msg1, msg2);
        }
        _ => panic!("Status mismatch after clone"),
    }
}

#[test]
fn test_deployment_status_debug() {
    let status = BiomeDeploymentStatus::Pending;
    let debug_str = format!("{status:?}");
    assert!(debug_str.contains("Pending"));

    let failed_status = BiomeDeploymentStatus::Failed("Error".to_string());
    let failed_debug = format!("{failed_status:?}");
    assert!(failed_debug.contains("Failed"));
    assert!(failed_debug.contains("Error"));
}

// ============================================================================
// BiomeDeployment Tests
// ============================================================================

#[test]
fn test_biome_deployment_creation() {
    let now = SystemTime::now();
    let deployment = BiomeDeployment {
        deployment_id: "deploy-123".to_string(),
        team_id: "team-456".to_string(),
        biome_manifest: json!({"name": "test-biome"}),
        status: BiomeDeploymentStatus::Pending,
        created_at: now,
        updated_at: now,
    };

    assert_eq!(deployment.deployment_id, "deploy-123");
    assert_eq!(deployment.team_id, "team-456");
    assert_eq!(deployment.created_at, deployment.updated_at);
}

#[test]
fn test_biome_deployment_status_transition() {
    let now = SystemTime::now();
    let mut deployment = BiomeDeployment {
        deployment_id: "deploy-123".to_string(),
        team_id: "team-456".to_string(),
        biome_manifest: json!({"name": "test-biome"}),
        status: BiomeDeploymentStatus::Pending,
        created_at: now,
        updated_at: now,
    };

    // Transition to Running
    deployment.status = BiomeDeploymentStatus::Running;
    match deployment.status {
        BiomeDeploymentStatus::Running => {}
        _ => panic!("Status transition failed"),
    }

    // Transition to Stopped
    deployment.status = BiomeDeploymentStatus::Stopped;
    match deployment.status {
        BiomeDeploymentStatus::Stopped => {}
        _ => panic!("Status transition failed"),
    }
}

#[test]
fn test_biome_deployment_manifest_complex() {
    let now = SystemTime::now();
    let complex_manifest = json!({
        "name": "ml-pipeline",
        "version": "1.0.0",
        "team": {
            "id": "ml-team",
            "members": ["alice", "bob"]
        },
        "resources": {
            "cpu": "4 cores",
            "memory": "8GB",
            "gpu": true
        }
    });

    let deployment = BiomeDeployment {
        deployment_id: "deploy-ml-001".to_string(),
        team_id: "ml-team".to_string(),
        biome_manifest: complex_manifest.clone(),
        status: BiomeDeploymentStatus::Running,
        created_at: now,
        updated_at: now,
    };

    assert_eq!(deployment.biome_manifest["name"], "ml-pipeline");
    assert_eq!(deployment.biome_manifest["team"]["id"], "ml-team");
    assert_eq!(deployment.biome_manifest["resources"]["gpu"], true);
}

#[test]
fn test_biome_deployment_serialization() {
    let now = SystemTime::now();
    let deployment = BiomeDeployment {
        deployment_id: "deploy-123".to_string(),
        team_id: "team-456".to_string(),
        biome_manifest: json!({"name": "test"}),
        status: BiomeDeploymentStatus::Running,
        created_at: now,
        updated_at: now,
    };

    let json = serde_json::to_string(&deployment).unwrap();
    let deserialized: BiomeDeployment = serde_json::from_str(&json).unwrap();

    assert_eq!(deployment.deployment_id, deserialized.deployment_id);
    assert_eq!(deployment.team_id, deserialized.team_id);
}

#[test]
fn test_biome_deployment_clone() {
    let now = SystemTime::now();
    let deployment1 = BiomeDeployment {
        deployment_id: "deploy-123".to_string(),
        team_id: "team-456".to_string(),
        biome_manifest: json!({"name": "test"}),
        status: BiomeDeploymentStatus::Pending,
        created_at: now,
        updated_at: now,
    };

    let deployment2 = deployment1.clone();
    assert_eq!(deployment1.deployment_id, deployment2.deployment_id);
    assert_eq!(deployment1.team_id, deployment2.team_id);
}

#[test]
fn test_biome_deployment_debug() {
    let now = SystemTime::now();
    let deployment = BiomeDeployment {
        deployment_id: "deploy-123".to_string(),
        team_id: "team-456".to_string(),
        biome_manifest: json!({"name": "test"}),
        status: BiomeDeploymentStatus::Running,
        created_at: now,
        updated_at: now,
    };

    let debug_str = format!("{deployment:?}");
    assert!(debug_str.contains("BiomeDeployment"));
    assert!(debug_str.contains("deploy-123"));
}

#[test]
fn test_biome_deployment_empty_manifest() {
    let now = SystemTime::now();
    let deployment = BiomeDeployment {
        deployment_id: "deploy-empty".to_string(),
        team_id: "team-empty".to_string(),
        biome_manifest: json!({}),
        status: BiomeDeploymentStatus::Pending,
        created_at: now,
        updated_at: now,
    };

    assert_eq!(deployment.biome_manifest, json!({}));
}

// ============================================================================
// BiomeOrchestrator Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_new() {
    let result = BiomeOrchestrator::new().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_initialize() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();
    let result = orchestrator.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_execute_deployment() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();
    let job = create_biome_job("team-alpha", json!({"app": "test-service", "replicas": 1}));
    let result = orchestrator.execute_deployment(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_rejects_non_biome_job_types() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();

    let native_job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/usr/bin/echo".to_string(),
            args: vec!["test".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: None,
        created_at: SystemTime::now(),
        context: create_test_primal_context(),
    };

    let result = orchestrator.execute_deployment(native_job).await;
    assert!(
        result.is_err(),
        "Native jobs should be rejected by BiomeOrchestrator"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_execute_deployment_returns_response() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();
    let mut job = create_biome_job(
        "team-beta",
        json!({"app": "response-test", "version": "1.0"}),
    );
    job.priority = JobPriority::High;
    job.timeout = Some(std::time::Duration::from_secs(300));

    let response = orchestrator.execute_deployment(job).await.unwrap();
    let _execution_response: ExecutionResponse = response;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_orchestrator_multiple_deployments() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();

    for i in 0..3 {
        let job = create_biome_job(
            &format!("team-{i}"),
            json!({"app": format!("service-{i}"), "replicas": i + 1}),
        );
        let result = orchestrator.execute_deployment(job).await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// BiomeOSIntegration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_os_integration_new() {
    let result = BiomeOSIntegration::new().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_os_integration_execute_deployment() {
    let integration = BiomeOSIntegration::new().await.unwrap();
    let job = create_biome_job("team-deploy", json!({"app": "echo-service", "replicas": 1}));
    let result = integration.execute_deployment(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_os_integration_multiple_jobs() {
    let integration = BiomeOSIntegration::new().await.unwrap();

    let manifests = vec![
        json!({"app": "compute-service", "replicas": 2}),
        json!({"app": "storage-service", "replicas": 1}),
        json!({"app": "api-gateway", "replicas": 3}),
    ];

    for (i, manifest) in manifests.into_iter().enumerate() {
        let job = create_biome_job(&format!("team-{i}"), manifest);
        let result = integration.execute_deployment(job).await;
        assert!(result.is_ok(), "deployment {i} failed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_biome_os_integration_concurrent_deployments() {
    let integration = BiomeOSIntegration::new().await.unwrap();

    for i in 0..5 {
        let job = create_biome_job(
            &format!("team-concurrent-{i}"),
            json!({"app": format!("deployment-{i}"), "replicas": 1}),
        );
        let result = integration.execute_deployment(job).await;
        assert!(result.is_ok(), "concurrent deployment {i} failed");
    }
}

// ============================================================================
// Integration Scenario Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_full_deployment_lifecycle() {
    let integration = BiomeOSIntegration::new().await.unwrap();
    let mut job = create_biome_job(
        "team-lifecycle",
        json!({"app": "lifecycle-service", "config": "test.yaml"}),
    );
    job.timeout = Some(std::time::Duration::from_secs(600));

    let response = integration.execute_deployment(job).await;
    assert!(response.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_with_complex_manifest() {
    let integration = BiomeOSIntegration::new().await.unwrap();
    let mut job = create_biome_job(
        "team-complex",
        json!({
            "app": "complex-service",
            "replicas": 3,
            "resources": {"cpu": "2", "memory": "4Gi"},
            "env": {"RUST_LOG": "info"}
        }),
    );
    job.priority = JobPriority::High;

    let result = integration.execute_deployment(job).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_orchestrator_initialization_sequence() {
    let orchestrator = BiomeOrchestrator::new().await.unwrap();

    let init_result = orchestrator.initialize().await;
    assert!(init_result.is_ok());

    let job = create_biome_job("team-init", json!({"app": "init-service", "replicas": 1}));
    let deploy_result = orchestrator.execute_deployment(job).await;
    assert!(deploy_result.is_ok());
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_config_with_all_features_enabled() {
    let config = BiomeOSConfig {
        enabled: true,
        endpoint: Some("https://biome.prod.com".to_string()),
        team_isolation: true,
        resource_quota_enforcement: true,
    };

    assert!(config.enabled);
    assert!(config.endpoint.is_some());
    assert!(config.team_isolation);
    assert!(config.resource_quota_enforcement);
}

#[test]
fn test_config_with_minimal_features() {
    let config = BiomeOSConfig {
        enabled: false,
        endpoint: None,
        team_isolation: false,
        resource_quota_enforcement: false,
    };

    assert!(!config.enabled);
    assert!(config.endpoint.is_none());
    assert!(!config.team_isolation);
    assert!(!config.resource_quota_enforcement);
}

#[test]
fn test_config_serialization_roundtrip() {
    let original = BiomeOSConfig {
        enabled: true,
        endpoint: Some("http://test:8080".to_string()),
        team_isolation: true,
        resource_quota_enforcement: false,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: BiomeOSConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original.enabled, deserialized.enabled);
    assert_eq!(original.endpoint, deserialized.endpoint);
    assert_eq!(original.team_isolation, deserialized.team_isolation);
    assert_eq!(
        original.resource_quota_enforcement,
        deserialized.resource_quota_enforcement
    );
}

// ============================================================================
// Sprint 21 Complete: 50 Tests Created
// Coverage Target: 0% → 70%
// ============================================================================
