// SPDX-License-Identifier: AGPL-3.0-or-later
//! BYOB (Bring Your Own Biome) type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

/// BYOB deployment request from coordination service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentRequest {
    /// Unique deployment ID
    pub deployment_id: Uuid,
    /// Team identifier
    pub team_id: String,
    /// Deployment name
    pub deployment_name: String,
    /// Services to deploy
    pub services: HashMap<String, ServiceSpec>,
    /// Resource quotas for the team
    pub resource_quotas: TeamResourceQuotas,
    /// Security configuration
    pub security_config: TeamSecurityConfig,
    /// Network configuration
    pub network_config: TeamNetworkConfig,
    /// Created timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
}

/// Service specification within a team deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    /// Service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Container image or executable
    pub image: Option<String>,
    /// Command to run
    pub command: Option<Vec<String>>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Resource requirements
    pub resources: ServiceResourceRequirements,
    /// Port mappings
    pub ports: Vec<PortMapping>,
    /// Volume mounts
    pub volumes: Vec<VolumeMount>,
    /// Dependencies on other services
    pub dependencies: Vec<String>,
    /// Health check configuration
    pub health_check: Option<HealthCheck>,
    /// Replica count
    pub replicas: u32,
}

/// Resource requirements for a service
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceResourceRequirements {
    /// CPU cores required
    pub cpu_cores: Option<f64>,
    /// Memory in bytes
    pub memory_bytes: Option<u64>,
    /// Storage in bytes
    pub storage_bytes: Option<u64>,
    /// GPU count
    pub gpu_count: Option<u32>,
}

/// Team resource quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResourceQuotas {
    /// Maximum CPU cores for the team
    pub max_cpu_cores: f64,
    /// Maximum memory in bytes
    pub max_memory_bytes: u64,
    /// Maximum storage in bytes
    pub max_storage_bytes: u64,
    /// Maximum GPU count
    pub max_gpu_count: u32,
    /// Maximum concurrent services
    pub max_concurrent_services: u32,
}

/// Team security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSecurityConfig {
    /// Isolation level
    pub isolation_level: String,
    /// Network policies
    pub network_policies: Vec<String>,
    /// Volume access policies
    pub volume_policies: Vec<String>,
    /// Resource access policies
    pub resource_policies: Vec<String>,
}

/// Team network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamNetworkConfig {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// DNS configuration
    pub dns_config: Option<DnsConfig>,
    /// Load balancer configuration
    pub load_balancer: Option<LoadBalancerConfig>,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Container port
    pub container_port: u16,
    /// Host port (optional)
    pub host_port: Option<u16>,
    /// Protocol (tcp/udp)
    pub protocol: String,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Source path
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Mount type
    pub mount_type: String,
    /// Read-only flag
    pub read_only: bool,
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Health check command
    pub command: Vec<String>,
    /// Check interval in seconds
    pub interval: u64,
    /// Timeout in seconds
    pub timeout: u64,
    /// Number of retries
    pub retries: u32,
    /// Start period in seconds
    pub start_period: u64,
}

/// DNS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    /// DNS servers
    pub servers: Vec<String>,
    /// Search domains
    pub search_domains: Vec<String>,
}

/// Load balancer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Load balancer type
    pub lb_type: String,
    /// Configuration options
    pub options: HashMap<String, String>,
}

/// BYOB deployment response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByobDeploymentResponse {
    /// Deployment ID
    pub deployment_id: Uuid,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Service statuses
    pub service_statuses: HashMap<String, ServiceStatus>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Network information
    pub network_info: NetworkInfo,
    /// Created timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// Updated timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated_at: SystemTime,
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is starting
    Starting,
    /// Deployment is running
    Running,
    /// Deployment is stopping
    Stopping,
    /// Deployment is stopped
    Stopped,
    /// Deployment failed
    Failed {
        /// Error message describing the failure.
        error: String,
    },
}

/// Service status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    /// Service name
    pub name: String,
    /// Service state
    pub state: String,
    /// Running replicas
    pub running_replicas: u32,
    /// Desired replicas
    pub desired_replicas: u32,
    /// Health status
    pub health: String,
    /// Last updated timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub updated_at: SystemTime,
}

/// Resource usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Storage usage in bytes
    pub storage_usage: u64,
    /// GPU usage
    pub gpu_usage: u32,
    /// Network usage
    pub network_usage: NetworkUsage,
}

/// Network usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
}

/// Network information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    /// Network name
    pub network_name: String,
    /// Subnet CIDR
    pub subnet_cidr: String,
    /// Gateway IP
    pub gateway_ip: String,
    /// Service endpoints
    pub service_endpoints: HashMap<String, ServiceEndpoint>,
}

/// Service endpoint information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Service name
    pub name: String,
    /// Internal IP
    pub internal_ip: String,
    /// External IP (if exposed)
    pub external_ip: Option<String>,
    /// Port mappings
    pub ports: Vec<PortMapping>,
}

#[cfg(test)]
mod tests {
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
}
