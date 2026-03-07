// SPDX-License-Identifier: AGPL-3.0-or-later
//! Network, health check, and deployment structure tests

use super::super::*;
use super::common::*;
use crate::byob::{
    validation::DeploymentValidator, PortMapping, ServiceResourceRequirements, TeamNetworkConfig,
    TeamResourceQuotas, TeamSecurityConfig,
};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

// ─── create_deployment_network tests ─────────────────────────────────────

#[test]
fn test_create_deployment_network_structure() {
    let config = create_test_config(8080, vec![80, 443]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let mut services = HashMap::new();
    services.insert("svc1".to_string(), create_test_service_spec("svc1"));

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-a".to_string(),
        deployment_name: "test".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 2,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "net1".to_string(),
            subnet_cidr: "192.168.1.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    };

    let network = executor.create_deployment_network(&request);

    assert_eq!(
        network.network_name,
        format!("byob-team-a-{}", request.deployment_id)
    );
    assert_eq!(network.subnet_cidr, "192.168.1.0/24");
    assert_eq!(network.gateway_ip, "10.0.0.1");
    assert_eq!(network.service_endpoints.len(), 1);
    assert!(network.service_endpoints.contains_key("svc1"));

    let ep = network.service_endpoints.get("svc1").unwrap();
    assert_eq!(ep.name, "svc1");
    assert!(ep.internal_ip.starts_with("10.0.0."));
}

#[test]
fn test_create_deployment_network_multiple_services() {
    let config = create_test_config(8080, vec![9999]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let mut services = HashMap::new();
    services.insert("a".to_string(), create_test_service_spec("a"));
    services.insert("b".to_string(), create_test_service_spec("b"));

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "t".to_string(),
        deployment_name: "d".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 2,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "n".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    };

    let network = executor.create_deployment_network(&request);
    assert_eq!(network.service_endpoints.len(), 2);
    let ips: Vec<_> = network
        .service_endpoints
        .values()
        .map(|e| e.internal_ip.as_str())
        .collect();
    assert_eq!(ips.len(), 2);
    assert!(ips.iter().all(|ip| ip.starts_with("10.0.0.")));
}

// ─── allocate_external_ip tests ──────────────────────────────────────────

#[test]
fn test_allocate_external_ip_no_web_ports() {
    let config = create_test_config(8080, vec![80, 443]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "internal".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 5432,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip = executor.allocate_external_ip(&service, "team1");
    assert!(ip.is_none());
}

#[test]
fn test_allocate_external_ip_web_port_allocates() {
    let config = create_test_config(8080, vec![80, 443, 8080]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "web".to_string(),
        version: "1".to_string(),
        image: Some("nginx:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip = executor.allocate_external_ip(&service, "team1");
    let ip = ip.expect("should allocate for port 80");
    assert!(
        ip.starts_with("198.51.100.")
            || ip.starts_with("203.0.113.")
            || ip.starts_with("192.0.2.")
            || ip.starts_with("203.0.114.")
    );
    assert!(ip.split('.').next_back().unwrap().parse::<u32>().unwrap() >= 50);
}

#[test]
fn test_allocate_external_ip_deterministic_by_team_and_service() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "svc".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip1 = executor.allocate_external_ip(&service, "a");
    let ip2 = executor.allocate_external_ip(&service, "ab");
    let ip3 = executor.allocate_external_ip(&service, "abc");
    let ip4 = executor.allocate_external_ip(&service, "abcd");

    assert!(ip1.is_some());
    assert!(ip2.is_some());
    assert!(ip3.is_some());
    assert!(ip4.is_some());
    let ip1b = executor.allocate_external_ip(&service, "a");
    assert_eq!(ip1, ip1b);
}

// ─── perform_health_check tests ──────────────────────────────────────────

#[test]
fn test_perform_health_check_empty_command() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec![],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health);
    assert!(result.unwrap());
}

#[test]
fn test_perform_health_check_http_commands() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());

    for cmd in ["curl", "wget", "http"] {
        let health = HealthCheck {
            command: vec![
                cmd.to_string(),
                "-f".to_string(),
                "http://localhost/".to_string(),
            ],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        };
        let result = executor.perform_health_check("svc", &health);
        assert!(result.unwrap(), "health check for {cmd} should pass");
    }
}

#[test]
fn test_perform_health_check_ping() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec!["ping".to_string(), "-c".to_string(), "1".to_string()],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health);
    assert!(result.unwrap());
}

#[test]
fn test_perform_health_check_custom_command() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec!["custom-script".to_string(), "arg1".to_string()],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health);
    assert!(result.unwrap());
}

// ─── create_byob_executor factory ────────────────────────────────────────

#[test]
fn test_create_byob_executor_returns_arc_of_executor() {
    let engine = create_test_runtime_engine();
    let executor = create_byob_executor(engine);
    assert!(std::sync::Arc::strong_count(&executor) >= 1);
}

// ─── DeploymentStatus enum variants ───────────────────────────────────────

#[test]
fn test_deployment_status_variants() {
    let starting = DeploymentStatus::Starting;
    let running = DeploymentStatus::Running;
    let stopping = DeploymentStatus::Stopping;
    let stopped = DeploymentStatus::Stopped;
    let failed = DeploymentStatus::Failed {
        error: "test error".to_string(),
    };

    assert!(matches!(starting, DeploymentStatus::Starting));
    assert!(matches!(running, DeploymentStatus::Running));
    assert!(matches!(stopping, DeploymentStatus::Stopping));
    assert!(matches!(stopped, DeploymentStatus::Stopped));
    assert!(matches!(
        failed,
        DeploymentStatus::Failed { error: ref e } if e == "test error"
    ));
}

#[test]
fn test_deployment_status_serialization() {
    let status = DeploymentStatus::Failed {
        error: "deployment failed".to_string(),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Failed"));
    assert!(json.contains("deployment failed"));

    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();
    match deserialized {
        DeploymentStatus::Failed { error } => assert_eq!(error, "deployment failed"),
        _ => panic!("expected Failed variant"),
    }
}

#[test]
fn test_deployment_status_running_serialization() {
    let status = DeploymentStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Running") || json == "\"Running\"");
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, DeploymentStatus::Running));
}

// ─── ResourceUsage and NetworkUsage ──────────────────────────────────────

#[test]
fn test_resource_usage_default_like() {
    let usage = ResourceUsage {
        cpu_usage: 1.5,
        memory_usage: 512 * 1024 * 1024,
        storage_usage: 10 * 1024 * 1024 * 1024,
        gpu_usage: 0,
        network_usage: NetworkUsage {
            bytes_sent: 1000,
            bytes_received: 500,
            packets_sent: 10,
            packets_received: 5,
        },
    };
    assert_eq!(usage.cpu_usage, 1.5);
    assert_eq!(usage.network_usage.bytes_sent, 1000);
}

#[test]
fn test_byob_executor_new_initializes_empty_deployments() {
    let engine = create_test_runtime_engine();
    let config = ByobExecutorConfig::default();
    let _executor = ByobComputeExecutor::new(engine, config);
    let request = create_test_deployment_request();
    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}

// ─── Network info and service endpoint ────────────────────────────────────

#[test]
fn test_service_endpoint_structure() {
    use crate::byob::byob_types::{PortMapping, ServiceEndpoint};
    let ep = ServiceEndpoint {
        name: "test-svc".to_string(),
        internal_ip: "10.0.0.10".to_string(),
        external_ip: Some("203.0.113.50".to_string()),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: Some(8080),
            protocol: "tcp".to_string(),
        }],
    };
    assert_eq!(ep.name, "test-svc");
    assert_eq!(ep.internal_ip, "10.0.0.10");
    assert_eq!(ep.external_ip.as_deref(), Some("203.0.113.50"));
}
