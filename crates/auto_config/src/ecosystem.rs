//! # Ecosystem Discovery for Auto-Configuration
//!
//! Discovers available ecosystem services (Songbird, `BearDog`, `NestGate`, Squirrel, biomeOS)
//! and automatically configures optimal integration settings.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::env_config::EnvironmentConfig;

/// Ecosystem discovery system for finding and configuring primal services
pub struct EcosystemDiscoverer {
    /// Known service patterns and ports
    service_patterns: HashMap<String, ServicePattern>,
    /// Discovery timeout
    _discovery_timeout: Duration,
    /// Last discovery results (cached)
    last_discovery: Option<DiscoveredServices>,
}

impl EcosystemDiscoverer {
    /// Create a new ecosystem discoverer
    #[must_use]
    pub fn new() -> Self {
        let mut service_patterns = HashMap::new();
        let config = EnvironmentConfig::from_env();

        // Songbird - Network coordination primal
        service_patterns.insert(
            "songbird".to_string(),
            ServicePattern {
                name: "songbird".to_string(),
                description: "Network coordination and orchestration".to_string(),
                default_ports: vec![config.network.songbird_port],
                health_endpoints: vec!["/health".to_string(), "/api/health".to_string()],
                service_type: ServiceType::NetworkCoordination,
                required_capabilities: vec!["network".to_string(), "coordination".to_string()],
            },
        );

        // BearDog - Security primal
        service_patterns.insert(
            "beardog".to_string(),
            ServicePattern {
                name: "beardog".to_string(),
                description: "Security and threat detection".to_string(),
                default_ports: vec![config.network.beardog_port],
                health_endpoints: vec!["/health".to_string(), "/api/security/health".to_string()],
                service_type: ServiceType::Security,
                required_capabilities: vec!["security".to_string(), "authentication".to_string()],
            },
        );

        // NestGate - Storage primal
        service_patterns.insert(
            "nestgate".to_string(),
            ServicePattern {
                name: "nestgate".to_string(),
                description: "Distributed storage and data management".to_string(),
                default_ports: vec![config.network.nestgate_port],
                health_endpoints: vec!["/health".to_string(), "/api/storage/health".to_string()],
                service_type: ServiceType::Storage,
                required_capabilities: vec!["storage".to_string(), "data_management".to_string()],
            },
        );

        // Squirrel - AI primal
        service_patterns.insert(
            "squirrel".to_string(),
            ServicePattern {
                name: "squirrel".to_string(),
                description: "AI and machine learning services".to_string(),
                default_ports: vec![config.network.squirrel_port],
                health_endpoints: vec!["/health".to_string(), "/api/ai/health".to_string()],
                service_type: ServiceType::AI,
                required_capabilities: vec!["ai".to_string(), "machine_learning".to_string()],
            },
        );

        // BiomeOS - Universal OS
        service_patterns.insert(
            "biomeos".to_string(),
            ServicePattern {
                name: "biomeos".to_string(),
                description: "Universal operating system and environment management".to_string(),
                default_ports: vec![8005, 8085, 9005],
                health_endpoints: vec!["/health".to_string(), "/api/biome/health".to_string()],
                service_type: ServiceType::OperatingSystem,
                required_capabilities: vec!["os_management".to_string(), "environment".to_string()],
            },
        );

        // Other ToadStool instances (recursive hosting)
        service_patterns.insert(
            "toadstool".to_string(),
            ServicePattern {
                name: "toadstool".to_string(),
                description: "Other ToadStool universal compute instances".to_string(),
                default_ports: vec![config.network.toadstool_port],
                health_endpoints: vec!["/health".to_string(), "/api/v2/health".to_string()],
                service_type: ServiceType::Compute,
                required_capabilities: vec![
                    "compute".to_string(),
                    "universal_execution".to_string(),
                ],
            },
        );

        Self {
            service_patterns,
            _discovery_timeout: Duration::from_secs(30),
            last_discovery: None,
        }
    }

    /// Discover all available ecosystem services
    pub async fn discover_services(&mut self) -> ToadStoolResult<DiscoveredServices> {
        info!("🌐 Starting ecosystem service discovery...");

        let mut discovered_services = HashMap::new();
        let mut discovery_summary = DiscoverySummary::default();

        // Phase 1: Local discovery (localhost and common IPs)
        info!("  🔍 Phase 1: Local service discovery...");
        let local_services = self.discover_local_services().await?;
        discovered_services.extend(local_services);

        // Phase 2: Network discovery (local network scanning)
        info!("  🌍 Phase 2: Network service discovery...");
        let network_services = self.discover_network_services().await?;
        discovered_services.extend(network_services);

        // Phase 3: Well-known service discovery (standard ports)
        info!("  📡 Phase 3: Well-known service discovery...");
        let wellknown_services = self.discover_wellknown_services().await?;
        discovered_services.extend(wellknown_services);

        // Phase 4: mDNS/Zeroconf discovery (if available)
        info!("  📢 Phase 4: mDNS/Zeroconf discovery...");
        let mdns_services = self.discover_mdns_services()?;
        discovered_services.extend(mdns_services);

        // Update discovery summary
        discovery_summary.total_services_found = discovered_services.len();
        discovery_summary.discovery_methods_used = vec![
            "local".to_string(),
            "network_scan".to_string(),
            "wellknown_ports".to_string(),
            "mdns".to_string(),
        ];

        let services = DiscoveredServices {
            discovered_services,
            discovery_summary,
            discovery_timestamp: chrono::Utc::now(),
        };

        // Cache the results
        self.last_discovery = Some(services.clone());

        info!("✅ Ecosystem discovery complete:");
        for (name, service) in &services.discovered_services {
            info!(
                "   🔗 {} -> {} ({})",
                name, service.endpoint, service.service_type
            );
        }

        Ok(services)
    }

    /// Discover services on localhost and common local IPs
    async fn discover_local_services(&self) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        let mut services = HashMap::new();
        let config = EnvironmentConfig::from_env();
        let local_ips = vec![
            config.network.bind_address.clone(),
            "127.0.0.1".to_string(),
            "0.0.0.0".to_string(),
        ];

        for ip in local_ips {
            for (service_name, pattern) in &self.service_patterns {
                for &port in &pattern.default_ports {
                    let endpoint = format!("http://{ip}:{port}");

                    if let Ok(service_info) = self.probe_service(&endpoint, pattern).await {
                        debug!("Found local service: {} at {}", service_name, endpoint);
                        services.insert(format!("{service_name}_{port}"), service_info);
                    }
                }
            }
        }

        debug!("Local discovery found {} services", services.len());
        Ok(services)
    }

    /// Discover services on the local network
    async fn discover_network_services(&self) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        let mut services = HashMap::new();

        // Get local network ranges to scan
        let network_ranges = self.get_local_network_ranges()?;

        for network_range in network_ranges {
            let range_services = self.scan_network_range(&network_range).await?;
            services.extend(range_services);
        }

        debug!("Network discovery found {} services", services.len());
        Ok(services)
    }

    /// Discover services on well-known ports
    async fn discover_wellknown_services(&self) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        let mut services = HashMap::new();

        // Common service discovery ports
        let wellknown_hosts = vec![
            "api.toadstool.dev".to_string(),
            "services.local".to_string(),
            "ecosystem.local".to_string(),
        ];

        for host in wellknown_hosts {
            for (service_name, pattern) in &self.service_patterns {
                for &port in &pattern.default_ports {
                    let endpoint = format!("http://{host}:{port}");

                    if let Ok(service_info) = self.probe_service(&endpoint, pattern).await {
                        debug!("Found well-known service: {} at {}", service_name, endpoint);
                        services.insert(format!("{service_name}_{host}"), service_info);
                    }
                }
            }
        }

        debug!("Well-known discovery found {} services", services.len());
        Ok(services)
    }

    /// Discover services using mDNS/Zeroconf
    fn discover_mdns_services(&self) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        let services = HashMap::new();

        // This would be implemented with a proper mDNS library
        // For now, we'll do a simplified version
        debug!("mDNS discovery not fully implemented, using fallback");

        // Fallback: try common mDNS service names
        let mdns_services = vec![
            "_toadstool._tcp.local",
            "_songbird._tcp.local",
            "_beardog._tcp.local",
            "_nestgate._tcp.local",
            "_squirrel._tcp.local",
            "_biomeos._tcp.local",
        ];

        for service_name in mdns_services {
            // In a real implementation, this would query mDNS
            // For now, we'll skip this functionality
            debug!("Would query mDNS for: {}", service_name);
        }

        debug!("mDNS discovery found {} services", services.len());
        Ok(services)
    }

    /// Probe a service endpoint to see if it's available and get info
    async fn probe_service(
        &self,
        endpoint: &str,
        pattern: &ServicePattern,
    ) -> ToadStoolResult<ServiceInfo> {
        // First, try to connect to the endpoint
        let url = endpoint
            .parse::<url::Url>()
            .map_err(|_| ToadStoolError::network(format!("Invalid URL: {endpoint}")))?;

        let config = EnvironmentConfig::from_env();
        let host = url.host_str().unwrap_or(&config.network.bind_address);
        let port = url.port().unwrap_or(80);
        let socket_addr = format!("{host}:{port}");

        // Try to establish a TCP connection
        if timeout(Duration::from_secs(2), TcpStream::connect(&socket_addr))
            .await
            .is_err()
        {
            return Err(ToadStoolError::network(format!(
                "Cannot connect to {socket_addr}"
            )));
        }

        // Try to get service info via HTTP
        let service_info = self.get_service_info(endpoint, pattern).await?;

        Ok(service_info)
    }

    /// Get detailed service information via HTTP
    async fn get_service_info(
        &self,
        endpoint: &str,
        pattern: &ServicePattern,
    ) -> ToadStoolResult<ServiceInfo> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| ToadStoolError::network(format!("HTTP client error: {e}")))?;

        // Try each health endpoint
        for health_endpoint in &pattern.health_endpoints {
            let health_url = format!("{endpoint}{health_endpoint}");

            if let Ok(Ok(response)) =
                timeout(Duration::from_secs(3), client.get(&health_url).send()).await
            {
                if response.status().is_success() {
                    // Try to parse service information
                    if let Ok(text) = response.text().await {
                        if let Ok(health_info) = serde_json::from_str::<serde_json::Value>(&text) {
                            return Ok(ServiceInfo {
                                name: pattern.name.clone(),
                                endpoint: endpoint.to_string(),
                                service_type: format!("{:?}", pattern.service_type),
                                version: health_info
                                    .get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                capabilities: pattern.required_capabilities.clone(),
                                status: ServiceStatus::Healthy,
                                discovered_via: "http_probe".to_string(),
                                response_time_ms: 0, // Would measure actual response time
                            });
                        }
                    }

                    // Fallback: create basic service info
                    return Ok(ServiceInfo {
                        name: pattern.name.clone(),
                        endpoint: endpoint.to_string(),
                        service_type: format!("{:?}", pattern.service_type),
                        version: "unknown".to_string(),
                        capabilities: pattern.required_capabilities.clone(),
                        status: ServiceStatus::Healthy,
                        discovered_via: "http_probe".to_string(),
                        response_time_ms: 0,
                    });
                }
            }
        }

        Err(ToadStoolError::network("No healthy endpoints found"))
    }

    /// Get local network ranges for scanning
    fn get_local_network_ranges(&self) -> ToadStoolResult<Vec<String>> {
        let ranges = vec![
            "192.168.1.0/24".to_string(),
            "192.168.0.0/24".to_string(),
            "10.0.0.0/24".to_string(),
            "172.16.0.0/24".to_string(),
        ];

        // In a real implementation, this would:
        // 1. Get actual network interfaces
        // 2. Calculate network ranges from interface IPs
        // 3. Use more sophisticated network discovery

        debug!("Using default network ranges: {:?}", ranges);
        Ok(ranges)
    }

    /// Scan a network range for services
    async fn scan_network_range(
        &self,
        range: &str,
    ) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
        let mut services = HashMap::new();

        // Parse CIDR range (simplified implementation)
        let base_ip = range.split('/').next().unwrap_or("192.168.1.0");
        let ip_parts: Vec<&str> = base_ip.split('.').collect();

        if ip_parts.len() != 4 {
            return Ok(services);
        }

        let base = format!("{}.{}.{}", ip_parts[0], ip_parts[1], ip_parts[2]);

        // Scan a subset of IPs (don't scan entire range to avoid being slow)
        let scan_ips = vec![1, 2, 10, 20, 50, 100, 200, 254];

        for ip_suffix in scan_ips {
            let ip = format!("{base}.{ip_suffix}");

            for (service_name, pattern) in &self.service_patterns {
                for &port in &pattern.default_ports {
                    let endpoint = format!("http://{ip}:{port}");

                    if let Ok(service_info) = self.probe_service(&endpoint, pattern).await {
                        debug!("Found network service: {} at {}", service_name, endpoint);
                        services.insert(format!("{service_name}_{ip}_{port}"), service_info);
                    }
                }
            }
        }

        debug!(
            "Network range {} scan found {} services",
            range,
            services.len()
        );
        Ok(services)
    }

    /// Get the last discovery results (cached)
    #[must_use]
    pub fn get_last_discovery(&self) -> Option<&DiscoveredServices> {
        self.last_discovery.as_ref()
    }

    /// Clear the discovery cache
    pub fn clear_cache(&mut self) {
        self.last_discovery = None;
    }
}

impl Default for EcosystemDiscoverer {
    fn default() -> Self {
        Self::new()
    }
}

/// Pattern for discovering a specific service type
#[derive(Debug, Clone)]
pub struct ServicePattern {
    pub name: String,
    pub description: String,
    pub default_ports: Vec<u16>,
    pub health_endpoints: Vec<String>,
    pub service_type: ServiceType,
    pub required_capabilities: Vec<String>,
}

/// Type of ecosystem service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceType {
    NetworkCoordination,
    Security,
    Storage,
    AI,
    OperatingSystem,
    Compute,
    Unknown,
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::NetworkCoordination => write!(f, "Network Coordination"),
            ServiceType::Security => write!(f, "Security"),
            ServiceType::Storage => write!(f, "Storage"),
            ServiceType::AI => write!(f, "AI"),
            ServiceType::OperatingSystem => write!(f, "Operating System"),
            ServiceType::Compute => write!(f, "Compute"),
            ServiceType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Discovered services container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServices {
    /// Map of service identifier to service information
    pub discovered_services: HashMap<String, ServiceInfo>,
    /// Summary of the discovery process
    pub discovery_summary: DiscoverySummary,
    /// When the discovery was performed
    pub discovery_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Information about a discovered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Service name (e.g., "songbird", "beardog")
    pub name: String,
    /// Full endpoint URL
    pub endpoint: String,
    /// Type of service
    pub service_type: String,
    /// Service version
    pub version: String,
    /// Service capabilities
    pub capabilities: Vec<String>,
    /// Current service status
    pub status: ServiceStatus,
    /// How the service was discovered
    pub discovered_via: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Summary of the discovery process
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoverySummary {
    /// Total number of services found
    pub total_services_found: usize,
    /// Discovery methods that were used
    pub discovery_methods_used: Vec<String>,
    /// Services found by type
    pub services_by_type: HashMap<String, usize>,
    /// Any errors encountered during discovery
    pub discovery_errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecosystem_discoverer_creation() {
        let discoverer = EcosystemDiscoverer::new();
        assert_eq!(discoverer.service_patterns.len(), 6); // 5 primals + toadstool
        assert!(discoverer.service_patterns.contains_key("songbird"));
        assert!(discoverer.service_patterns.contains_key("beardog"));
        assert!(discoverer.service_patterns.contains_key("nestgate"));
        assert!(discoverer.service_patterns.contains_key("squirrel"));
        assert!(discoverer.service_patterns.contains_key("biomeos"));
        assert!(discoverer.service_patterns.contains_key("toadstool"));
    }

    #[test]
    fn test_service_pattern_structure() {
        let discoverer = EcosystemDiscoverer::new();
        let songbird_pattern = discoverer.service_patterns.get("songbird").unwrap();

        assert_eq!(songbird_pattern.name, "songbird");
        assert!(!songbird_pattern.default_ports.is_empty());
        assert!(!songbird_pattern.health_endpoints.is_empty());
        assert!(matches!(
            songbird_pattern.service_type,
            ServiceType::NetworkCoordination
        ));
    }

    #[tokio::test]
    async fn test_network_range_parsing() {
        let discoverer = EcosystemDiscoverer::new();
        let ranges = discoverer.get_local_network_ranges().unwrap();

        assert!(!ranges.is_empty());
        assert!(ranges.contains(&"192.168.1.0/24".to_string()));
    }

    #[test]
    fn test_service_info_serialization() {
        let service_info = ServiceInfo {
            name: "test_service".to_string(),
            endpoint: toadstool_config::network::get_toadstool_endpoint(),
            service_type: "Test".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec!["test".to_string()],
            status: ServiceStatus::Healthy,
            discovered_via: "test".to_string(),
            response_time_ms: 100,
        };

        let json = serde_json::to_string(&service_info).unwrap();
        assert!(json.contains("test_service"));
        // Environment-aware: check for port in 8080-8089 range (default is 8084)
        assert!(json.contains(":808") || json.contains("127.0.0.1"));
    }

    #[test]
    fn test_discovery_summary_default() {
        let summary = DiscoverySummary::default();
        assert_eq!(summary.total_services_found, 0);
        assert!(summary.discovery_methods_used.is_empty());
        assert!(summary.services_by_type.is_empty());
        assert!(summary.discovery_errors.is_empty());
    }

    #[test]
    fn test_service_status_variants() {
        let statuses = vec![
            ServiceStatus::Healthy,
            ServiceStatus::Degraded,
            ServiceStatus::Unhealthy,
            ServiceStatus::Unknown,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            assert!(!json.is_empty());
        }
    }
}
