// SPDX-License-Identifier: AGPL-3.0-only

use super::DeploymentValidator;
use crate::byob::byob_types::{
    ByobDeploymentRequest, PortMapping, ServiceResourceRequirements, ServiceSpec,
    TeamNetworkConfig, TeamResourceQuotas, TeamSecurityConfig,
};
use uuid::Uuid;

fn create_test_service() -> ServiceSpec {
    ServiceSpec {
        name: "test-service".to_string(),
        version: "1.0.0".to_string(),
        image: Some("test:latest".to_string()),
        command: None,
        environment: std::collections::HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            storage_bytes: Some(10 * 1024 * 1024 * 1024), // 10GB
            gpu_count: None,
        },
        ports: Vec::new(),
        volumes: Vec::new(),
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    }
}

#[test]
fn test_validate_deployment_success() {
    let mut services = std::collections::HashMap::new();
    services.insert("test".to_string(), create_test_service());

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}

#[test]
fn test_validate_deployment_exceeds_cpu_quota() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.resources.cpu_cores = Some(20.0);
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_exceeds_memory_quota() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.resources.memory_bytes = Some(15 * 1024 * 1024 * 1024); // 15GB
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_exceeds_storage_quota() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.resources.storage_bytes = Some(150 * 1024 * 1024 * 1024); // 150GB
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_exceeds_gpu_quota() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.resources.gpu_count = Some(1);
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_empty_services_list() {
    let services = std::collections::HashMap::new();

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_too_many_services() {
    let mut services = std::collections::HashMap::new();
    services.insert("svc1".to_string(), create_test_service());
    services.insert("svc2".to_string(), create_test_service());
    services.insert("svc3".to_string(), create_test_service());

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 2,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_service_no_image_and_no_command() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.image = None;
    service.command = None;
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_duplicate_host_port() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.ports = vec![
        PortMapping {
            container_port: 8080,
            host_port: Some(9000),
            protocol: "tcp".to_string(),
        },
        PortMapping {
            container_port: 9090,
            host_port: Some(9000),
            protocol: "tcp".to_string(),
        },
    ];
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_err());
}

#[test]
fn test_validate_deployment_service_command_but_no_image() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.image = None;
    service.command = Some(vec!["run.sh".to_string(), "start".to_string()]);
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}

#[test]
fn test_validate_deployment_multiple_services_within_quota() {
    let mut services = std::collections::HashMap::new();
    let mut svc1 = create_test_service();
    svc1.resources.cpu_cores = Some(2.0);
    svc1.resources.memory_bytes = Some(2 * 1024 * 1024 * 1024);
    svc1.resources.storage_bytes = Some(20 * 1024 * 1024 * 1024);
    services.insert("svc1".to_string(), svc1);

    let mut svc2 = create_test_service();
    svc2.name = "svc2".to_string();
    svc2.resources.cpu_cores = Some(2.0);
    svc2.resources.memory_bytes = Some(3 * 1024 * 1024 * 1024);
    svc2.resources.storage_bytes = Some(25 * 1024 * 1024 * 1024);
    services.insert("svc2".to_string(), svc2);

    let mut svc3 = create_test_service();
    svc3.name = "svc3".to_string();
    svc3.resources.cpu_cores = Some(1.0);
    svc3.resources.memory_bytes = Some(1024 * 1024 * 1024);
    svc3.resources.storage_bytes = Some(10 * 1024 * 1024 * 1024);
    services.insert("svc3".to_string(), svc3);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}

#[test]
fn test_validate_deployment_zero_resource_services() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.resources = ServiceResourceRequirements {
        cpu_cores: None,
        memory_bytes: None,
        storage_bytes: None,
        gpu_count: None,
    };
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 1.0,
            max_memory_bytes: 1024,
            max_storage_bytes: 1024,
            max_gpu_count: 0,
            max_concurrent_services: 1,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}

#[test]
fn test_validate_deployment_ports_without_duplicates() {
    let mut services = std::collections::HashMap::new();
    let mut service = create_test_service();
    service.ports = vec![
        PortMapping {
            container_port: 8080,
            host_port: Some(9000),
            protocol: "tcp".to_string(),
        },
        PortMapping {
            container_port: 9090,
            host_port: Some(9001),
            protocol: "tcp".to_string(),
        },
    ];
    services.insert("test".to_string(), service);

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 10 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 0,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: std::time::SystemTime::now(),
    };

    assert!(DeploymentValidator::validate_deployment(&request).is_ok());
}
