//! Network management for BYOB deployments

use super::byob_types::{ByobDeploymentRequest, NetworkInfo, PortMapping, ServiceSpec};
use super::config::ByobExecutorConfig;
use std::net::Ipv4Addr;
use tracing::{debug, warn};

/// Manages network configuration and IP allocation for deployments
pub(super) struct NetworkManager<'a> {
    config: &'a ByobExecutorConfig,
}

impl<'a> NetworkManager<'a> {
    /// Create a new network manager with configuration
    pub fn new(config: &'a ByobExecutorConfig) -> Self {
        Self { config }
    }

    /// Create network configuration for a deployment
    pub fn create_network(&self, request: &ByobDeploymentRequest) -> NetworkInfo {
        let subnet = self.config.default_network_subnet.clone();
        let gateway = self.calculate_gateway(&subnet);

        debug!(
            "Creating network for deployment {} with subnet {}",
            request.deployment_id, subnet
        );

        NetworkInfo {
            network_id: format!("byob-net-{}", request.deployment_id),
            subnet: subnet.clone(),
            gateway,
            dns_servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
            isolation_enabled: request
                .network_config
                .as_ref()
                .map(|cfg| cfg.isolation_level == "strict")
                .unwrap_or(true),
        }
    }

    /// Allocate external IP for a service if needed
    pub fn allocate_external_ip(&self, service_spec: &ServiceSpec, team_id: &str) -> Option<String> {
        // Check if service exposes ports that need external access
        let needs_external_ip = service_spec.ports.iter().any(|port_mapping| {
            self.is_public_port(port_mapping)
        });

        if needs_external_ip {
            let ip = self.generate_external_ip(team_id, &service_spec.image);
            debug!(
                "Allocated external IP {} for service {} (team: {})",
                ip, service_spec.image, team_id
            );
            Some(ip)
        } else {
            None
        }
    }

    /// Check if port mapping requires external IP
    fn is_public_port(&self, port_mapping: &PortMapping) -> bool {
        // Check if this is a common web service port
        self.config
            .web_service_ports
            .contains(&port_mapping.container_port)
            || port_mapping.host_port.map_or(false, |hp| {
                self.config.web_service_ports.contains(&hp)
            })
    }

    /// Calculate gateway IP from subnet
    fn calculate_gateway(&self, subnet: &str) -> String {
        // Parse subnet (e.g., "10.0.0.0/24")
        if let Some((network, _mask)) = subnet.split_once('/') {
            if let Ok(ip) = network.parse::<Ipv4Addr>() {
                let octets = ip.octets();
                // Gateway is typically .1 in the subnet
                return format!("{}.{}.{}.1", octets[0], octets[1], octets[2]);
            }
        }

        warn!("Could not parse subnet {}, using default gateway", subnet);
        "10.0.0.1".to_string()
    }

    /// Generate external IP for service (simplified allocation)
    fn generate_external_ip(&self, team_id: &str, service_name: &str) -> String {
        // In production, this would integrate with cloud provider APIs
        // or IPAM systems. For now, generate a deterministic IP based on hash
        let hash = Self::simple_hash(team_id, service_name);
        let octet3 = ((hash >> 8) & 0xFF) as u8;
        let octet4 = (hash & 0xFF) as u8;

        format!("203.0.{}.{}", octet3, octet4)
    }

    /// Simple hash function for deterministic IP generation
    fn simple_hash(team_id: &str, service_name: &str) -> u16 {
        let combined = format!("{}{}", team_id, service_name);
        let mut hash: u16 = 0;
        for byte in combined.bytes() {
            hash = hash.wrapping_add(byte as u16);
            hash = hash.wrapping_mul(31);
        }
        hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> ByobExecutorConfig {
        ByobExecutorConfig {
            max_concurrent_deployments: 50,
            default_network_subnet: "10.0.0.0/24".to_string(),
            resource_monitoring_interval: std::time::Duration::from_secs(30),
            health_check_interval: std::time::Duration::from_secs(10),
            deployment_timeout: std::time::Duration::from_secs(600),
            default_host_port: 8080,
            web_service_ports: vec![80, 443, 8080],
            graceful_shutdown_timeout_secs: 30,
        }
    }

    #[test]
    fn test_create_network() {
        let config = create_test_config();
        let manager = NetworkManager::new(&config);

        let request = ByobDeploymentRequest {
            deployment_id: uuid::Uuid::new_v4(),
            team_id: "test-team".to_string(),
            services: HashMap::new(),
            network_config: None,
            resource_quotas: Default::default(),
        };

        let network = manager.create_network(&request);
        assert_eq!(network.subnet, "10.0.0.0/24");
        assert_eq!(network.gateway, "10.0.0.1");
        assert!(network.isolation_enabled);
    }

    #[test]
    fn test_allocate_external_ip_for_web_service() {
        let config = create_test_config();
        let manager = NetworkManager::new(&config);

        let service_spec = ServiceSpec {
            image: "nginx:latest".to_string(),
            environment: HashMap::new(),
            ports: vec![PortMapping {
                container_port: 80,
                host_port: Some(8080),
                protocol: "tcp".to_string(),
            }],
            volumes: vec![],
            resources: Default::default(),
            depends_on: vec![],
            health_check: None,
        };

        let ip = manager.allocate_external_ip(&service_spec, "team-123");
        assert!(ip.is_some());
    }

    #[test]
    fn test_no_external_ip_for_internal_service() {
        let config = create_test_config();
        let manager = NetworkManager::new(&config);

        let service_spec = ServiceSpec {
            image: "database:latest".to_string(),
            environment: HashMap::new(),
            ports: vec![PortMapping {
                container_port: 5432,
                host_port: None,
                protocol: "tcp".to_string(),
            }],
            volumes: vec![],
            resources: Default::default(),
            depends_on: vec![],
            health_check: None,
        };

        let ip = manager.allocate_external_ip(&service_spec, "team-123");
        assert!(ip.is_none());
    }

    #[test]
    fn test_calculate_gateway() {
        let config = create_test_config();
        let manager = NetworkManager::new(&config);

        assert_eq!(manager.calculate_gateway("10.0.0.0/24"), "10.0.0.1");
        assert_eq!(manager.calculate_gateway("192.168.1.0/24"), "192.168.1.1");
        assert_eq!(manager.calculate_gateway("172.16.0.0/16"), "172.16.0.1");
    }

    #[test]
    fn test_deterministic_ip_generation() {
        let config = create_test_config();
        let manager = NetworkManager::new(&config);

        let ip1 = manager.generate_external_ip("team-1", "service-a");
        let ip2 = manager.generate_external_ip("team-1", "service-a");
        assert_eq!(ip1, ip2, "Same inputs should generate same IP");

        let ip3 = manager.generate_external_ip("team-2", "service-a");
        assert_ne!(ip1, ip3, "Different inputs should generate different IPs");
    }
}

