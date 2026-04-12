// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
//! Validation and creation tests for BYOB implementation

use super::super::*;
use super::common::*;
use crate::byob::{
    PortMapping, ServiceResourceRequirements, VolumeMount, validation::DeploymentValidator,
};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn test_validate_deployment_request() {
    let mock_engine = create_test_runtime_engine();
    let _executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let valid_request = create_test_deployment_request();
    assert!(DeploymentValidator::validate_deployment(&valid_request).is_ok());

    let mut invalid_request = valid_request;
    for i in 0..100 {
        invalid_request.services.insert(
            format!("service-{i}"),
            create_test_service_spec(&format!("service-{i}")),
        );
    }
    assert!(DeploymentValidator::validate_deployment(&invalid_request).is_err());

    let mut resource_heavy_request = create_test_deployment_request();
    let mut heavy_service = create_test_service_spec("heavy-service");
    heavy_service.resources.cpu_cores = Some(1000.0);
    heavy_service.resources.memory_bytes = Some(1024 * 1024 * 1024 * 1024);
    resource_heavy_request
        .services
        .insert("heavy-service".to_string(), heavy_service);
    assert!(DeploymentValidator::validate_deployment(&resource_heavy_request).is_err());

    let mut empty_request = create_test_deployment_request();
    empty_request.services.clear();
    assert!(DeploymentValidator::validate_deployment(&empty_request).is_err());

    let mut no_image_no_cmd_request = create_test_deployment_request();
    let mut bad_service = create_test_service_spec("bad-service");
    bad_service.image = None;
    bad_service.command = None;
    no_image_no_cmd_request
        .services
        .insert("bad-service".to_string(), bad_service);
    assert!(DeploymentValidator::validate_deployment(&no_image_no_cmd_request).is_err());
}

#[test]
fn test_byob_executor_creation() {
    let config = ByobExecutorConfig::default();
    assert_eq!(config.max_concurrent_deployments, 50);
    assert_eq!(config.default_network_subnet, "10.0.0.0/24");
    assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
    assert_eq!(config.health_check_interval, Duration::from_secs(10));
    assert_eq!(config.deployment_timeout, Duration::from_secs(600));

    #[allow(deprecated)]
    let env_config = toadstool_config::env_config::EnvironmentConfig::from_env();
    #[allow(deprecated)]
    let expected_port = env_config.network.coordination_port;
    assert_eq!(config.default_host_port, expected_port);
    assert!(config.web_service_ports.contains(&80));
    assert!(config.web_service_ports.contains(&443));
    assert!(config.web_service_ports.contains(&expected_port));
    assert!(config.web_service_ports.contains(&8443));
}

#[test]
fn test_deployment_request_validation() {
    let deployment_request = create_test_deployment_request();
    assert!(!deployment_request.deployment_id.is_nil());
    assert_eq!(deployment_request.team_id, "test-team");
    assert_eq!(deployment_request.services.len(), 2);

    let web_service = deployment_request
        .services
        .get("web-service")
        .expect("web-service should exist in deployment request");
    assert_eq!(web_service.name, "web-service");
    assert!(web_service.image.is_some());
    assert!(!web_service.ports.is_empty());

    let api_service = deployment_request
        .services
        .get("api-service")
        .expect("api-service should exist in deployment request");
    assert_eq!(api_service.name, "api-service");
    assert!(api_service.image.is_some());
    assert!(!api_service.environment.is_empty());
}

#[test]
fn test_deployment_request_serialization_roundtrip() {
    let request = create_test_deployment_request();
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("deployment_id"));
    assert!(json.contains("test-team"));
    let restored: ByobDeploymentRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.team_id, request.team_id);
    assert_eq!(restored.services.len(), request.services.len());
}

// ─── create_service_execution_request tests ──────────────────────────────

#[test]
fn test_create_service_execution_request_container_workload() {
    let config = create_test_config(9999, vec![80, 443, 8080]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "web-svc".to_string(),
        version: "1.0".to_string(),
        image: Some("nginx:latest".to_string()),
        command: Some(vec!["nginx".to_string()]),
        environment: HashMap::from([("FOO".to_string(), "bar".to_string())]),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(2.0),
            memory_bytes: Some(1024 * 1024 * 1024),
            storage_bytes: Some(2 * 1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![VolumeMount {
            source: "/data".to_string(),
            target: "/app/data".to_string(),
            mount_type: "volume".to_string(),
            read_only: true,
        }],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    assert!(!req.execution_id.is_nil());
    match &req.workload {
        crate::WorkloadSpec::Container {
            image,
            command,
            env_vars,
            volumes,
            ports,
            ..
        } => {
            assert_eq!(image, "nginx:latest");
            assert_eq!(command, &Some(vec!["nginx".to_string()]));
            assert_eq!(env_vars.get("FOO"), Some(&"bar".to_string()));
            assert_eq!(volumes.len(), 1);
            assert_eq!(volumes[0].source.to_str(), Some("/data"));
            assert_eq!(volumes[0].target.to_str(), Some("/app/data"));
            assert!(matches!(
                volumes[0].mount_type,
                crate::workload::VolumeMountType::Volume
            ));
            assert!(volumes[0].read_only);
            assert_eq!(ports.len(), 1);
            assert_eq!(ports[0].container_port, 80);
            assert_eq!(ports[0].host_port, 9999);
            assert!(matches!(
                ports[0].protocol,
                crate::workload::PortProtocol::Tcp
            ));
        }
        _ => panic!("expected Container workload"),
    }
    assert_eq!(req.resources.cpu.min_cores, 2.0);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_create_service_execution_request_volume_mount_types() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let volumes = vec![
        VolumeMount {
            source: "/tmp".to_string(),
            target: "/tmp".to_string(),
            mount_type: "bind".to_string(),
            read_only: false,
        },
        VolumeMount {
            source: "myvol".to_string(),
            target: "/data".to_string(),
            mount_type: "volume".to_string(),
            read_only: true,
        },
    ];

    let service = ServiceSpec {
        name: "svc".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(512 * 1024 * 1024),
            storage_bytes: Some(1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![],
        volumes,
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    match &req.workload {
        crate::WorkloadSpec::Container { volumes: v, .. } => {
            assert!(matches!(
                v[0].mount_type,
                crate::workload::VolumeMountType::Bind
            ));
            assert!(matches!(
                v[1].mount_type,
                crate::workload::VolumeMountType::Volume
            ));
        }
        _ => panic!("expected Container"),
    }
}

#[test]
fn test_create_service_execution_request_udp_port() {
    let config = create_test_config(5353, vec![53]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "dns".to_string(),
        version: "1".to_string(),
        image: Some("dns:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(256 * 1024 * 1024),
            storage_bytes: Some(1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![PortMapping {
            container_port: 53,
            host_port: Some(10053),
            protocol: "udp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    match &req.workload {
        crate::WorkloadSpec::Container { ports, .. } => {
            assert!(matches!(
                ports[0].protocol,
                crate::workload::PortProtocol::Udp
            ));
            assert_eq!(ports[0].host_port, 10053);
            assert_eq!(ports[0].container_port, 53);
        }
        _ => panic!("expected Container"),
    }
}

#[test]
fn test_create_service_execution_request_native_workload() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "native-svc".to_string(),
        version: "1".to_string(),
        image: None,
        command: Some(vec!["/usr/bin/myapp".to_string()]),
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: None,
            memory_bytes: None,
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    match &req.workload {
        crate::WorkloadSpec::Native {
            executable: crate::ExecutableSource::File { path },
            ..
        } => {
            assert_eq!(path.to_str().unwrap(), "/bin/sh");
        }
        _ => panic!("expected Native workload"),
    }
    assert_eq!(req.resources.cpu.min_cores, 1.0);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 10 * 1024 * 1024 * 1024);
}

#[test]
fn test_create_service_execution_request_native_with_image_path() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "native".to_string(),
        version: "1".to_string(),
        image: Some("/usr/local/bin/custom".to_string()),
        command: Some(vec!["run".to_string()]),
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    match &req.workload {
        crate::WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "/usr/local/bin/custom");
        }
        _ => panic!("expected Container when image is Some"),
    }
}

#[test]
fn test_create_service_execution_request_resource_defaults() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "minimal".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
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
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    assert_eq!(req.resources.cpu.min_cores, 1.0);
    assert_eq!(req.resources.cpu.max_cores, None);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 10 * 1024 * 1024 * 1024);
    assert!(req.resources.gpu.is_none());
}

#[test]
fn test_create_service_execution_request_with_gpu() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "gpu-svc".to_string(),
        version: "1".to_string(),
        image: Some("cuda:11".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(8.0),
            memory_bytes: Some(32 * 1024 * 1024 * 1024),
            storage_bytes: Some(100 * 1024 * 1024 * 1024),
            gpu_count: Some(2),
        },
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor.create_service_execution_request(&service, Uuid::new_v4());

    let gpu = req.resources.gpu.expect("gpu requirements");
    assert_eq!(gpu.min_units, 2);
    assert_eq!(gpu.max_units, Some(2));
}
