//! Service discovery functionality for ecosystem integration
//!
//! This module handles discovering services on the network using:
//! - Environment variables (TOADSTOOL_*_SERVICE_URL)
//! - mDNS/Bonjour discovery
//! - Service mesh queries (Consul, etcd, K8s)
//! - Configuration files
//!
//! **NO HARDCODED PORTS OR SERVICE NAMES** - Pure infant discovery.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use super::types::*;

/// Discover service by capability using environment variables
///
/// Checks for environment variables in the format:
/// - `TOADSTOOL_CRYPTO_SERVICE_URL` for crypto capabilities
/// - `TOADSTOOL_STORAGE_SERVICE_URL` for storage capabilities
/// - `TOADSTOOL_COORDINATION_SERVICE_URL` for coordination capabilities
///
/// # Example
/// ```bash
/// export TOADSTOOL_CRYPTO_SERVICE_URL="http://10.0.0.5:9876"
/// export TOADSTOOL_STORAGE_SERVICE_URL="http://nestgate.local:8082"
/// ```
pub fn discover_from_environment(capability_category: &str) -> Option<String> {
    let env_var = format!(
        "TOADSTOOL_{}_SERVICE_URL",
        capability_category.to_uppercase()
    );

    std::env::var(&env_var).ok().and_then(|url| {
        if url.is_empty() {
            None
        } else {
            debug!(
                "Discovered {} service from environment: {}",
                capability_category, url
            );
            Some(url)
        }
    })
}

/// Discover service from configuration file
///
/// Looks for configuration using Pure Rust etcetera in:
/// 1. `~/.toadstool/services.toml`
/// 2. `./.toadstool/config.toml`
/// 3. `/etc/toadstool/services.toml`
///
/// # Example config format (TOML)
/// ```toml
/// [services.crypto]
/// url = "http://beardog.local:9876"
/// priority = 90
///
/// [services.storage]
/// url = "http://nestgate.local:8082"
/// priority = 80
/// ```
pub fn discover_from_config(capability_category: &str) -> Option<String> {
    use etcetera::{choose_base_strategy, BaseStrategy};

    // Try multiple config locations (user home first, system config last-resort fallback)
    let config_paths = if let Ok(strategy) = choose_base_strategy() {
        vec![
            Some(strategy.home_dir().join(".toadstool/services.toml")),
            Some(std::path::PathBuf::from(".toadstool/config.toml")),
            Some(std::path::PathBuf::from(
                crate::ecosystem::constants::paths::SYSTEM_SERVICES_CONFIG,
            )),
        ]
    } else {
        vec![
            Some(std::path::PathBuf::from(".toadstool/config.toml")),
            Some(std::path::PathBuf::from(
                crate::ecosystem::constants::paths::SYSTEM_SERVICES_CONFIG,
            )),
        ]
    };

    for path in config_paths.into_iter().flatten() {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(config) =
                toml::from_str::<HashMap<String, HashMap<String, toml::Value>>>(&contents)
            {
                if let Some(services) = config.get("services") {
                    if let Some(service) = services.get(capability_category) {
                        if let Some(toml::Value::String(url)) = service.get("url") {
                            debug!(
                                "Discovered {} service from config {}: {}",
                                capability_category,
                                path.display(),
                                url
                            );
                            return Some(url.clone());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Discover service by capability (no hardcoded ports!)
///
/// Discovery order:
/// 1. Environment variables (TOADSTOOL_*_SERVICE_URL)
/// 2. Configuration files (~/.toadstool/services.toml)
/// 3. mDNS discovery (_capability._tcp.local)
/// 4. Service mesh query (if available)
///
/// # Example
/// ```ignore
/// // Forward-looking example - API under development
/// # async fn example() -> anyhow::Result<()> {
/// // Discovers crypto service from any source
/// let endpoints = discover_service_by_capability("crypto").await?;
/// # Ok(())
/// # }
/// ```
///
/// **Status**: Currently unused but part of capability-based architecture.
/// Will be used when full capability routing is implemented.
#[allow(dead_code)]
#[allow(deprecated)] // ServiceEndpoint still uses EcosystemService for backward compat
pub async fn discover_service_by_capability(
    capability_category: &str,
) -> Result<Vec<ServiceEndpoint>> {
    let mut services = Vec::new();

    // Try environment variables first
    if let Some(url) = discover_from_environment(capability_category) {
        if let Ok(addr) = parse_service_url(&url) {
            services.push(ServiceEndpoint {
                service_type: EcosystemService::Unknown(capability_category.to_string()),
                address: addr,
                version: Arc::from(crate::ecosystem::constants::common::UNKNOWN_VERSION),
                capabilities: vec![capability_category.to_string()],
                trust_level: TrustLevel::Configured,
            });
            return Ok(services);
        }
    }

    // Try configuration files
    if let Some(url) = discover_from_config(capability_category) {
        if let Ok(addr) = parse_service_url(&url) {
            services.push(ServiceEndpoint {
                service_type: EcosystemService::Unknown(capability_category.to_string()),
                address: addr,
                version: Arc::from(crate::ecosystem::constants::common::UNKNOWN_VERSION),
                capabilities: vec![capability_category.to_string()],
                trust_level: TrustLevel::Configured,
            });
            return Ok(services);
        }
    }

    // Try mDNS discovery
    if let Ok(discovered) = discover_via_mdns(capability_category).await {
        services.extend(discovered);
        if !services.is_empty() {
            return Ok(services);
        }
    }

    // FUTURE: Service mesh integration (Consul, etcd, K8s) planned for v0.3.0
    // This requires deploying to a service mesh environment first.
    // See: docs/planning/SERVICE_MESH_INTEGRATION.md (to be created)

    // No service found
    warn!("No service found for capability: {}", capability_category);
    Ok(services)
}

/// Parse service URL into SocketAddr
///
/// Used by capability-based service discovery when converting discovered
/// service URLs into socket addresses for connection.
#[allow(dead_code)]
fn parse_service_url(url: &str) -> Result<SocketAddr> {
    // Handle various URL formats
    let url = url.trim();

    // Remove protocol prefix if present
    let without_protocol = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);

    // Parse as SocketAddr
    without_protocol
        .parse()
        .with_context(|| format!("Failed to parse service URL: {}", url))
}

/// Discover service via mDNS/Bonjour
///
/// Searches for services advertising the capability via mDNS.
/// Service names follow the pattern: `_<capability>-service._tcp.local`
///
/// # Example
/// ```text
/// _crypto-service._tcp.local     -> crypto capability
/// _storage-service._tcp.local    -> storage capability
/// _coord-service._tcp.local      -> coordination capability
/// ```
///
/// # Returns
/// A list of discovered service endpoints with `TrustLevel::Advertised`.
///
/// # Timeout
/// Discover services via mDNS for local network service discovery.
///
/// Discovery runs for 2 seconds to allow time for mDNS responses.
///
/// Discover services via mDNS using `toadstool::discovery::MdnsDiscoveryService`.
///
/// Delegates to the production mDNS implementation in toadstool-core (uses mdns-sd).
/// Discovery runs for 2 seconds, matching Bonjour browse behavior.
#[allow(dead_code)]
#[allow(deprecated)]
async fn discover_via_mdns(capability_category: &str) -> Result<Vec<ServiceEndpoint>> {
    let mdns = match toadstool::discovery::MdnsDiscoveryService::new() {
        Ok(m) => m,
        Err(e) => {
            debug!("mDNS unavailable (no multicast interface?): {e}");
            return Ok(Vec::new());
        }
    };

    let discovered = mdns
        .discover_by_capability(capability_category, Duration::from_secs(2))
        .await
        .unwrap_or_default();

    let endpoints: Vec<ServiceEndpoint> = discovered
        .into_iter()
        .filter_map(|svc| {
            let addr: SocketAddr = svc.endpoint.parse().ok()?;
            let caps: Vec<String> = svc.capabilities.iter().map(|c| c.name.clone()).collect();
            Some(ServiceEndpoint {
                service_type: EcosystemService::Unknown(capability_category.to_string()),
                address: addr,
                version: Arc::from(svc.version.as_str()),
                capabilities: caps,
                trust_level: TrustLevel::Advertised,
            })
        })
        .collect();

    if !endpoints.is_empty() {
        info!(
            "mDNS discovered {} service(s) for capability '{}'",
            endpoints.len(),
            capability_category
        );
    }

    Ok(endpoints)
}

// ============================================================================
// ⚠️ DEPRECATED LEGACY DISCOVERY (Hardcoded Ports)
// ============================================================================
// The functions below are DEPRECATED and use hardcoded service ports.
// They violate the infant discovery principle.
//
// **DO NOT USE** in new code. Use `discover_service_by_capability()` instead.
//
// These will be removed after `integrator_impl.rs` is migrated to the new
// capability-based discovery system.
// ============================================================================

// ✅ REMOVED: get_standard_service_ports() function (December 2, 2025)
// This function violated infant discovery principles with hardcoded ports.
// Use toadstool_config::ports::PortRegistry for dynamic port configuration.
// Use toadstool_config::services::ServiceRegistry for service discovery.

// ✅ REMOVED: scan_for_service() - deprecated since 0.1.0
// Use discover_service_by_capability() instead for capability-based discovery

// ✅ REMOVED: is_service_reachable() - was only used by deprecated scan_for_service()

/// Verify a discovered service by checking its health endpoint
///
/// # Errors
/// Returns an error if the service verification encounters network issues
/// or connection failures. Note: Verification timeout or unreachable service
/// returns `Ok(false)` rather than an error.
#[must_use = "Service verification result should be checked"]
pub async fn verify_service(service: &ServiceEndpoint) -> Result<bool> {
    // Try to connect with a longer timeout for verification
    match timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(&service.address),
    )
    .await
    {
        Ok(Ok(_)) => {
            info!("✅ Service verified: {}", service.address);
            Ok(true)
        }
        Ok(Err(e)) => {
            warn!("⚠️  Service verification failed: {}", e);
            Ok(false)
        }
        Err(_) => {
            warn!("⚠️  Service verification timeout");
            Ok(false)
        }
    }
}

// ⚠️ DEPRECATED: Service name parsing removed
// Services are now discovered by CAPABILITY, not by name.
// See: crates/cli/src/ecosystem/capabilities/ for the new system.

// health_check() removed - was unused. Service health is checked via
// is_service_reachable() and verify_service() which are actively used.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_from_environment_found() {
        std::env::set_var("TOADSTOOL_CRYPTO_SERVICE_URL", "http://10.0.0.5:9876");
        let result = discover_from_environment("crypto");
        std::env::remove_var("TOADSTOOL_CRYPTO_SERVICE_URL");
        assert_eq!(result, Some("http://10.0.0.5:9876".to_string()));
    }

    #[test]
    fn test_discover_from_environment_not_found() {
        std::env::remove_var("TOADSTOOL_TESTCAP_SERVICE_URL");
        let result = discover_from_environment("testcap");
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_from_environment_empty_value() {
        std::env::set_var("TOADSTOOL_EMPTY_SERVICE_URL", "");
        let result = discover_from_environment("empty");
        std::env::remove_var("TOADSTOOL_EMPTY_SERVICE_URL");
        assert!(result.is_none());
    }

    #[test]
    fn test_discover_from_environment_uppercase() {
        std::env::set_var(
            "TOADSTOOL_STORAGE_SERVICE_URL",
            "http://nestgate.local:8082",
        );
        let result = discover_from_environment("storage");
        std::env::remove_var("TOADSTOOL_STORAGE_SERVICE_URL");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_service_url_with_http_prefix() {
        let addr = parse_service_url("http://127.0.0.1:8080").unwrap();
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_parse_service_url_plain_socket_addr() {
        let addr = parse_service_url("127.0.0.1:9090").unwrap();
        assert_eq!(addr.port(), 9090);
    }

    #[test]
    fn test_parse_service_url_invalid_returns_err() {
        let result = parse_service_url("not_a_url");
        assert!(result.is_err());
    }

    #[test]
    fn test_discover_from_config_no_config_returns_none() {
        // When no config file exists at expected paths, returns None
        let result = discover_from_config("nonexistent_capability_xyz");
        assert!(result.is_none());
    }
}
