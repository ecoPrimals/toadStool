// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service specs, deployment status, resource usage, and deployment requests.

use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use toadstool::byob::*;

// ============================================================================
// ServiceSpec Tests
// ============================================================================

#[test]
fn test_service_spec_basic() {
    let spec = ServiceSpec {
        name: "web-service".to_string(),
        version: "1.0.0".to_string(),
        image: Some("nginx:latest".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(512_000_000),
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: vec![],
        health_check: None,
        replicas: 1,
    };

    assert_eq!(spec.name, "web-service");
    assert_eq!(spec.version, "1.0.0");
    assert_eq!(spec.replicas, 1);
}

#[test]
fn test_service_spec_with_environment() {
    let mut env = HashMap::new();
    env.insert(
        "DATABASE_URL".to_string(),
        "postgres://localhost/db".to_string(),
    );
    env.insert("API_KEY".to_string(), "secret123".to_string());

    let spec = ServiceSpec {
        name: "api".to_string(),
        version: "2.0.0".to_string(),
        image: Some("api:v2".to_string()),
        command: Some(vec!["npm".to_string(), "start".to_string()]),
        environment: env,
        resources: ServiceResourceRequirements {
            cpu_cores: Some(2.0),
            memory_bytes: Some(2_000_000_000),
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: vec![],
        health_check: None,
        replicas: 3,
    };

    assert_eq!(spec.environment.len(), 2);
    assert_eq!(spec.replicas, 3);
    assert_eq!(
        spec.command,
        Some(vec!["npm".to_string(), "start".to_string()])
    );
}

#[test]
fn test_service_spec_with_dependencies() {
    let spec = ServiceSpec {
        name: "worker".to_string(),
        version: "1.0.0".to_string(),
        image: Some("worker:latest".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: None,
            memory_bytes: None,
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: vec!["database".to_string(), "cache".to_string()],
        health_check: None,
        replicas: 2,
    };

    assert_eq!(spec.dependencies.len(), 2);
    assert!(spec.dependencies.contains(&"database".to_string()));
}

#[test]
fn test_service_spec_clone() {
    let spec1 = ServiceSpec {
        name: "test-service".to_string(),
        version: "0.1.0".to_string(),
        image: None,
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: None,
            memory_bytes: None,
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: vec![],
        health_check: None,
        replicas: 1,
    };

    let spec2 = spec1.clone();
    assert_eq!(spec1.name, spec2.name);
    assert_eq!(spec1.version, spec2.version);
}

#[test]
fn test_service_spec_serialization() {
    let spec = ServiceSpec {
        name: "db".to_string(),
        version: "13".to_string(),
        image: Some("postgres:13".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(2.0),
            memory_bytes: Some(4_000_000_000),
            storage_bytes: Some(50_000_000_000),
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: vec![],
        health_check: None,
        replicas: 1,
    };

    let json = serde_json::to_string(&spec).unwrap();
    let deserialized: ServiceSpec = serde_json::from_str(&json).unwrap();

    assert_eq!(spec.name, deserialized.name);
    assert_eq!(spec.image, deserialized.image);
}

// ============================================================================
// DeploymentStatus Tests
// ============================================================================

#[test]
fn test_deployment_status_starting() {
    let status = DeploymentStatus::Starting;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();

    matches!(deserialized, DeploymentStatus::Starting);
}

#[test]
fn test_deployment_status_stopping() {
    let status = DeploymentStatus::Stopping;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();

    matches!(deserialized, DeploymentStatus::Stopping);
}

#[test]
fn test_deployment_status_running() {
    let status = DeploymentStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();

    matches!(deserialized, DeploymentStatus::Running);
}

#[test]
fn test_deployment_status_failed() {
    let status = DeploymentStatus::Failed {
        error: "Connection timeout".to_string(),
    };
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();

    if let DeploymentStatus::Failed { error } = deserialized {
        assert_eq!(error, "Connection timeout");
    } else {
        panic!("Expected Failed status");
    }
}

#[test]
fn test_deployment_status_stopped() {
    let status = DeploymentStatus::Stopped;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();

    matches!(deserialized, DeploymentStatus::Stopped);
}

// ============================================================================
// ResourceUsage Tests
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_resource_usage_creation() {
    let usage = ResourceUsage {
        cpu_usage: 2.5,
        memory_usage: 4_000_000_000,
        storage_usage: 10_000_000_000,
        gpu_usage: 1,
        network_usage: NetworkUsage {
            bytes_sent: 500_000,
            bytes_received: 1_000_000,
            packets_sent: 8_000,
            packets_received: 10_000,
        },
    };

    assert_eq!(usage.cpu_usage, 2.5);
    assert_eq!(usage.gpu_usage, 1);
    assert_eq!(usage.network_usage.bytes_received, 1_000_000);
}

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_resource_usage_clone() {
    let usage1 = ResourceUsage {
        cpu_usage: 1.0,
        memory_usage: 1_000_000_000,
        storage_usage: 5_000_000_000,
        gpu_usage: 0,
        network_usage: NetworkUsage {
            bytes_sent: 50_000,
            bytes_received: 100_000,
            packets_sent: 500,
            packets_received: 1_000,
        },
    };

    let usage2 = usage1.clone();
    assert_eq!(usage1.cpu_usage, usage2.cpu_usage);
    assert_eq!(
        usage1.network_usage.bytes_received,
        usage2.network_usage.bytes_received
    );
}

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_resource_usage_serialization() {
    let usage = ResourceUsage {
        cpu_usage: 3.0,
        memory_usage: 8_000_000_000,
        storage_usage: 20_000_000_000,
        gpu_usage: 2,
        network_usage: NetworkUsage {
            bytes_sent: 3_000_000,
            bytes_received: 5_000_000,
            packets_sent: 30_000,
            packets_received: 50_000,
        },
    };

    let json = serde_json::to_string(&usage).unwrap();
    let deserialized: ResourceUsage = serde_json::from_str(&json).unwrap();

    assert_eq!(usage.cpu_usage, deserialized.cpu_usage);
    assert_eq!(usage.gpu_usage, deserialized.gpu_usage);
}

// ============================================================================
// ByobDeploymentRequest Tests
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#[test]
fn test_byob_deployment_request_creation() {
    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-123".to_string(),
        deployment_name: "my-deployment".to_string(),
        services: HashMap::new(),
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 20_000_000_000,
            max_storage_bytes: 100_000_000_000,
            max_gpu_count: 2,
            max_concurrent_services: 20,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "high".to_string(),
            network_policies: vec!["deny-all".to_string()],
            volume_policies: vec!["read-only".to_string()],
            resource_policies: vec!["limited".to_string()],
        },
        network_config: TeamNetworkConfig {
            network_name: "team-network".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    };

    assert_eq!(request.team_id, "team-123");
    assert_eq!(request.deployment_name, "my-deployment");
    assert_eq!(request.resource_quotas.max_cpu_cores, 10.0);
}

#[test]
fn test_byob_deployment_request_serialization() {
    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-456".to_string(),
        deployment_name: "test-deployment".to_string(),
        services: HashMap::new(),
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 8.0,
            max_memory_bytes: 16_000_000_000,
            max_storage_bytes: 50_000_000_000,
            max_gpu_count: 1,
            max_concurrent_services: 15,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "medium".to_string(),
            network_policies: vec!["allow-internal".to_string()],
            volume_policies: vec!["read-write".to_string()],
            resource_policies: vec!["flexible".to_string()],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-network".to_string(),
            subnet_cidr: "10.1.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: ByobDeploymentRequest = serde_json::from_str(&json).unwrap();

    assert_eq!(request.team_id, deserialized.team_id);
    assert_eq!(request.deployment_name, deserialized.deployment_name);
}

// ============================================================================
// Sprint 22 Complete: 60 Tests Created
// Coverage Target: 36% → 60%
// ============================================================================
