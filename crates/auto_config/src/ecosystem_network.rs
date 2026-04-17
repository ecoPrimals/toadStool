// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Ecosystem network discovery
//!
//! Network scanning, probing, and service discovery on local networks.

use std::collections::HashMap;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::debug;

use crate::ecosystem_types::{ServiceInfo, ServicePattern, ServiceStatus};
use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::network::HTTP_PROTOCOL;
use toadstool_config::defaults::network::{
    COMMON_SCAN_SUFFIXES, PROBE_DEFAULT_PORT, RFC1918_SCAN_RANGES,
};
use toadstool_config::env_config::EnvironmentConfig;

/// TCP connect timeout for `probe_service`. Production uses 2s; tests use a
/// short value so discovery probes fail fast under `cargo test`.
const TCP_PROBE_CONNECT_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_millis(100)
} else {
    Duration::from_secs(2)
};

/// `/24`-style network base (IPv4) used only when `range` lacks a valid prefix (see `scan_network_range`).
pub const DEFAULT_SCAN_SUBNET: &str = "192.168.1.0";

/// Get local network ranges for scanning
///
/// Returns a list of CIDR-style network ranges to scan for ecosystem services.
#[must_use]
pub fn get_local_network_ranges() -> Vec<String> {
    let ranges: Vec<String> = RFC1918_SCAN_RANGES
        .iter()
        .map(|s| (*s).to_string())
        .collect();

    debug!("Using default network ranges: {:?}", ranges);
    ranges
}

/// Probe a service endpoint to see if it's available and get info
///
/// # Errors
/// Returns `ToadStoolError` if the URL is invalid, TCP connection fails, or
/// service info cannot be retrieved.
pub async fn probe_service(
    endpoint: &str,
    pattern: &ServicePattern,
) -> ToadStoolResult<ServiceInfo> {
    let url = endpoint
        .parse::<url::Url>()
        .map_err(|_| ToadStoolError::network(format!("Invalid URL: {endpoint}")))?;

    let config = EnvironmentConfig::from_env();
    let host = url.host_str().unwrap_or(&config.network.bind_address);
    let port = url.port().unwrap_or(PROBE_DEFAULT_PORT);
    let socket_addr = format!("{host}:{port}");

    if timeout(TCP_PROBE_CONNECT_TIMEOUT, TcpStream::connect(&socket_addr))
        .await
        .is_err()
    {
        return Err(ToadStoolError::network(format!(
            "Cannot connect to {socket_addr}"
        )));
    }

    Ok(get_service_info(endpoint, pattern))
}

/// Get detailed service information
///
/// **EVOLUTION**: HTTP probing removed, use environment-based discovery
#[must_use]
pub fn get_service_info(endpoint: &str, pattern: &ServicePattern) -> ServiceInfo {
    tracing::info!("Creating service info for {} at {}", pattern.name, endpoint);

    ServiceInfo {
        name: pattern.name.clone(),
        endpoint: endpoint.to_string(),
        service_type: format!("{:?}", pattern.service_type),
        version: std::env::var(format!(
            "{}_VERSION",
            pattern.name.to_uppercase().replace('-', "_")
        ))
        .unwrap_or_else(|_| "unknown".to_string()),
        capabilities: pattern.required_capabilities.clone(),
        status: ServiceStatus::Healthy,
        discovered_via: "environment_config".to_string(),
        response_time_ms: 0,
    }
}

/// Scan a network range for services
///
/// # Errors
/// Returns `ToadStoolError` if scanning encounters errors.
pub async fn scan_network_range(
    service_patterns: &HashMap<String, ServicePattern>,
    range: &str,
) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
    let mut services = HashMap::new();

    let base_ip = range.split('/').next().unwrap_or(DEFAULT_SCAN_SUBNET);
    let ip_parts: Vec<&str> = base_ip.split('.').collect();

    if ip_parts.len() != 4 {
        return Ok(services);
    }

    let base = format!("{}.{}.{}", ip_parts[0], ip_parts[1], ip_parts[2]);

    for &ip_suffix in COMMON_SCAN_SUFFIXES {
        let ip = format!("{base}.{ip_suffix}");

        for (capability_key, pattern) in service_patterns {
            for &port in &pattern.default_ports {
                let endpoint = format!("{HTTP_PROTOCOL}{ip}:{port}");

                if let Ok(service_info) = probe_service(&endpoint, pattern).await {
                    debug!("Found {} capability at {}", capability_key, endpoint);
                    services.insert(format!("{capability_key}_{ip}_{port}"), service_info);
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

/// Discover services on the local network
///
/// Scans configured network ranges and probes for ecosystem services.
///
/// # Errors
/// Returns `ToadStoolError` if network scanning fails.
pub async fn discover_network_services(
    service_patterns: &HashMap<String, ServicePattern>,
) -> ToadStoolResult<HashMap<String, ServiceInfo>> {
    let mut services = HashMap::new();

    let network_ranges = get_local_network_ranges();

    for network_range in network_ranges {
        let range_services = scan_network_range(service_patterns, &network_range).await?;
        services.extend(range_services);
    }

    debug!("Network discovery found {} services", services.len());
    Ok(services)
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_config::defaults::network::RFC1918_SCAN_RANGES;

    #[tokio::test]
    async fn test_network_range_parsing() {
        let ranges = get_local_network_ranges();

        assert!(!ranges.is_empty());
        assert!(ranges.contains(&RFC1918_SCAN_RANGES[0].to_string()));
    }

    #[tokio::test]
    async fn test_scan_network_range_invalid_cidr_returns_empty() {
        let service_patterns = std::collections::HashMap::new();
        let result = scan_network_range(&service_patterns, "not-a-valid-cidr").await;
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_scan_network_range_malformed_ip_returns_empty() {
        let service_patterns = std::collections::HashMap::new();
        let result = scan_network_range(&service_patterns, "1.2.3/24").await;
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_probe_service_invalid_url_returns_err() {
        use crate::ecosystem_types::ServicePattern;
        let pattern = ServicePattern {
            name: "test".to_string(),
            description: String::new(),
            service_type: crate::ecosystem_types::ServiceType::Compute,
            default_ports: vec![8080],
            health_endpoints: vec![],
            required_capabilities: vec![],
        };
        let result = probe_service("not-a-valid-url!!!", &pattern).await;
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Invalid") || err_str.contains("URL"));
    }

    #[tokio::test]
    async fn test_get_service_info_creates_info() {
        use crate::ecosystem_types::{ServicePattern, ServiceType};
        let pattern = ServicePattern {
            name: "compute-svc".to_string(),
            description: String::new(),
            service_type: ServiceType::Compute,
            default_ports: vec![9000],
            health_endpoints: vec![],
            required_capabilities: vec!["compute".to_string()],
        };
        let info = get_service_info("http://192.168.1.1:9000", &pattern);
        assert_eq!(info.name, "compute-svc");
        assert_eq!(info.endpoint, "http://192.168.1.1:9000");
        assert!(matches!(
            info.status,
            crate::ecosystem_types::ServiceStatus::Healthy
        ));
    }

    #[tokio::test]
    async fn test_get_local_network_ranges_has_four_defaults() {
        let ranges = get_local_network_ranges();
        assert_eq!(ranges.len(), RFC1918_SCAN_RANGES.len());
        assert!(ranges.contains(&RFC1918_SCAN_RANGES[0].to_string()));
        assert!(ranges.contains(&RFC1918_SCAN_RANGES[2].to_string()));
    }

    #[tokio::test]
    async fn test_scan_network_range_valid_cidr_empty_patterns() {
        let service_patterns = std::collections::HashMap::new();
        let result = scan_network_range(&service_patterns, "192.168.1.0/24").await;
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_discover_network_services_empty_patterns() {
        let service_patterns = std::collections::HashMap::new();
        let result = discover_network_services(&service_patterns).await;
        assert!(result.is_ok());
        let services = result.unwrap();
        assert!(services.is_empty());
    }

    #[test]
    fn test_get_local_network_ranges_no_network_io() {
        let ranges = get_local_network_ranges();
        assert!(!ranges.is_empty());
        for r in &ranges {
            assert!(r.contains('/'));
        }
    }

    #[tokio::test]
    async fn test_get_service_info_version_from_env() {
        use crate::ecosystem_types::{ServicePattern, ServiceType};
        temp_env::with_var("COMPUTE_SVC_VERSION", Some("2.0.0"), || {
            let pattern = ServicePattern {
                name: "compute-svc".to_string(),
                description: String::new(),
                service_type: ServiceType::Compute,
                default_ports: vec![8080],
                health_endpoints: vec![],
                required_capabilities: vec![],
            };
            let info = get_service_info("http://localhost:8080", &pattern);
            assert_eq!(info.version, "2.0.0");
        });
    }
}
