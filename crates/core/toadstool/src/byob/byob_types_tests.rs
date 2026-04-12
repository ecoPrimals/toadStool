// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::collections::HashMap;

fn sample_port_mapping() -> PortMapping {
    PortMapping {
        container_port: 8080,
        host_port: Some(80),
        protocol: "tcp".to_string(),
    }
}

fn sample_volume_mount() -> VolumeMount {
    VolumeMount {
        source: "/data".to_string(),
        target: "/mnt/data".to_string(),
        mount_type: "bind".to_string(),
        read_only: false,
    }
}

fn sample_service_spec() -> ServiceSpec {
    let mut env = HashMap::new();
    env.insert("KEY".to_string(), "value".to_string());
    ServiceSpec {
        name: "test-svc".to_string(),
        version: "1.0.0".to_string(),
        image: Some("nginx:latest".to_string()),
        command: Some(vec!["nginx".to_string(), "-g".to_string()]),
        environment: env,
        resources: ServiceResourceRequirements::default(),
        ports: vec![sample_port_mapping()],
        volumes: vec![sample_volume_mount()],
        dependencies: vec!["redis".to_string()],
        health_check: None,
        replicas: 2,
    }
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // round-trip and literals in tests
fn test_byob_deployment_request_serialization_round_trip() {
    let mut services = HashMap::new();
    services.insert("api".to_string(), sample_service_spec());
    let req = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-1".to_string(),
        deployment_name: "deploy-1".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 16.0,
            max_memory_bytes: 32 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 2,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec!["allow-internal".to_string()],
            volume_policies: vec!["read-write".to_string()],
            resource_policies: vec!["cpu-limit".to_string()],
        },
        network_config: TeamNetworkConfig {
            network_name: "team-net".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let parsed: ByobDeploymentRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(req.deployment_id, parsed.deployment_id);
    assert_eq!(req.team_id, parsed.team_id);
    assert_eq!(req.deployment_name, parsed.deployment_name);
    assert_eq!(
        req.resource_quotas.max_cpu_cores,
        parsed.resource_quotas.max_cpu_cores
    );
}

#[test]
fn test_service_spec_serialization_round_trip() {
    let spec = sample_service_spec();
    let json = serde_json::to_string(&spec).expect("serialize");
    let parsed: ServiceSpec = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(spec.name, parsed.name);
    assert_eq!(spec.version, parsed.version);
    assert_eq!(spec.image, parsed.image);
    assert_eq!(spec.replicas, parsed.replicas);
}

#[test]
fn test_port_mapping_serialization_round_trip() {
    let pm = sample_port_mapping();
    let json = serde_json::to_string(&pm).expect("serialize");
    let parsed: PortMapping = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(pm.container_port, parsed.container_port);
    assert_eq!(pm.host_port, parsed.host_port);
    assert_eq!(pm.protocol, parsed.protocol);
}

#[test]
fn test_volume_mount_serialization_round_trip() {
    let vm = sample_volume_mount();
    let json = serde_json::to_string(&vm).expect("serialize");
    let parsed: VolumeMount = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(vm.source, parsed.source);
    assert_eq!(vm.target, parsed.target);
    assert_eq!(vm.mount_type, parsed.mount_type);
    assert_eq!(vm.read_only, parsed.read_only);
}

#[test]
fn test_service_resource_requirements_default() {
    let req = ServiceResourceRequirements::default();
    assert!(req.cpu_cores.is_none());
    assert!(req.memory_bytes.is_none());
    assert!(req.storage_bytes.is_none());
    assert!(req.gpu_count.is_none());
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // literals just assigned in test
fn test_team_resource_quotas_construction() {
    let quotas = TeamResourceQuotas {
        max_cpu_cores: 8.0,
        max_memory_bytes: 16 * 1024 * 1024 * 1024,
        max_storage_bytes: 50 * 1024 * 1024 * 1024,
        max_gpu_count: 1,
        max_concurrent_services: 5,
    };
    assert_eq!(quotas.max_cpu_cores, 8.0);
    assert_eq!(quotas.max_memory_bytes, 16 * 1024 * 1024 * 1024);
    assert_eq!(quotas.max_gpu_count, 1);
    assert_eq!(quotas.max_concurrent_services, 5);
}

#[test]
fn test_deployment_status_variants() {
    let starting = DeploymentStatus::Starting;
    let running = DeploymentStatus::Running;
    let stopping = DeploymentStatus::Stopping;
    let stopped = DeploymentStatus::Stopped;
    let failed = DeploymentStatus::Failed {
        error: "oops".to_string(),
    };
    assert!(matches!(starting, DeploymentStatus::Starting));
    assert!(matches!(running, DeploymentStatus::Running));
    assert!(matches!(stopping, DeploymentStatus::Stopping));
    assert!(matches!(stopped, DeploymentStatus::Stopped));
    if let DeploymentStatus::Failed { error } = failed {
        assert_eq!(error, "oops");
    } else {
        unreachable!("expected Failed variant");
    }
}

#[test]
fn test_deployment_status_equality() {
    let a = DeploymentStatus::Running;
    let b = DeploymentStatus::Running;
    let c = DeploymentStatus::Stopped;
    assert!(matches!(
        (&a, &b),
        (DeploymentStatus::Running, DeploymentStatus::Running)
    ));
    assert!(!matches!(&a, DeploymentStatus::Stopped));
    assert!(matches!(&c, DeploymentStatus::Stopped));
}

#[test]
fn test_network_info_construction_and_field_access() {
    let mut endpoints = HashMap::new();
    endpoints.insert(
        "api".to_string(),
        ServiceEndpoint {
            name: "api".to_string(),
            internal_ip: "10.0.0.2".to_string(),
            external_ip: Some("203.0.113.1".to_string()),
            ports: vec![sample_port_mapping()],
        },
    );
    let info = NetworkInfo {
        network_name: "prod-net".to_string(),
        subnet_cidr: "10.0.0.0/24".to_string(),
        gateway_ip: "10.0.0.1".to_string(),
        service_endpoints: endpoints.clone(),
    };
    assert_eq!(info.network_name, "prod-net");
    assert_eq!(info.subnet_cidr, "10.0.0.0/24");
    assert_eq!(info.gateway_ip, "10.0.0.1");
    assert_eq!(info.service_endpoints.len(), 1);
    let ep = info.service_endpoints.get("api").unwrap();
    assert_eq!(ep.internal_ip, "10.0.0.2");
    assert_eq!(ep.external_ip, Some("203.0.113.1".to_string()));
}

#[test]
#[expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)] // literal just assigned in test
fn test_resource_usage_construction() {
    let usage = ResourceUsage {
        cpu_usage: 0.5,
        memory_usage: 2 * 1024 * 1024 * 1024,
        storage_usage: 10 * 1024 * 1024 * 1024,
        gpu_usage: 0,
        network_usage: NetworkUsage {
            bytes_sent: 1_000_000,
            bytes_received: 2_000_000,
            packets_sent: 10_000,
            packets_received: 20_000,
        },
    };
    assert_eq!(usage.cpu_usage, 0.5);
    assert_eq!(usage.memory_usage, 2 * 1024 * 1024 * 1024);
    assert_eq!(usage.network_usage.bytes_sent, 1_000_000);
    assert_eq!(usage.network_usage.bytes_received, 2_000_000);
}

#[test]
fn test_network_usage_construction() {
    let usage = NetworkUsage {
        bytes_sent: 100,
        bytes_received: 200,
        packets_sent: 5,
        packets_received: 10,
    };
    assert_eq!(usage.bytes_sent, 100);
    assert_eq!(usage.bytes_received, 200);
    assert_eq!(usage.packets_sent, 5);
    assert_eq!(usage.packets_received, 10);
}

#[test]
fn test_deployment_status_serialization() {
    let status = DeploymentStatus::Running;
    let json = serde_json::to_string(&status).expect("serialize");
    let parsed: DeploymentStatus = serde_json::from_str(&json).expect("deserialize");
    assert!(matches!(parsed, DeploymentStatus::Running));

    let failed = DeploymentStatus::Failed {
        error: "test error".to_string(),
    };
    let json = serde_json::to_string(&failed).expect("serialize");
    let parsed: DeploymentStatus = serde_json::from_str(&json).expect("deserialize");
    if let DeploymentStatus::Failed { error } = parsed {
        assert_eq!(error, "test error");
    } else {
        unreachable!("expected Failed variant");
    }
}

#[test]
fn test_health_check_construction() {
    let hc = HealthCheck {
        command: vec![
            "curl".to_string(),
            "-f".to_string(),
            "http://localhost/health".to_string(),
        ],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 60,
    };
    assert_eq!(hc.interval, 30);
    assert_eq!(hc.timeout, 5);
    assert_eq!(hc.retries, 3);
}

#[test]
fn test_dns_config_and_load_balancer() {
    let dns = DnsConfig {
        servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
        search_domains: vec!["internal".to_string()],
    };
    let mut lb_opts = HashMap::new();
    lb_opts.insert("algorithm".to_string(), "round-robin".to_string());
    let lb = LoadBalancerConfig {
        lb_type: "nginx".to_string(),
        options: lb_opts,
    };
    let net = TeamNetworkConfig {
        network_name: "net".to_string(),
        subnet_cidr: "10.0.0.0/24".to_string(),
        dns_config: Some(dns),
        load_balancer: Some(lb),
    };
    assert!(net.dns_config.is_some());
    assert!(net.load_balancer.is_some());
    let dns = net.dns_config.as_ref().unwrap();
    assert_eq!(dns.servers.len(), 2);
    let lb = net.load_balancer.as_ref().unwrap();
    assert_eq!(lb.lb_type, "nginx");
}
