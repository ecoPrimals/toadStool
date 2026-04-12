// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based service discovery.
//!
//! Discovers ecoPrimal services via env vars, mDNS, Kubernetes, Docker Compose, and registries.

use tracing::{debug, info, warn};

use crate::constants::discovery_ports::DISCOVERY_HTTP_FALLBACK;

use super::{DiscoveryError, DiscoveryResult, PrimalEndpoint};

mod backends;

#[cfg(test)]
mod tests;

pub use backends::{
    builtin_default_endpoint, try_discover_via_docker_compose, try_discover_via_filesystem,
    try_discover_via_kubernetes, try_discover_via_mdns, try_discover_via_registry,
};

/// Port for capability-based service discovery (K8s, Docker Compose).
/// Overridable via `TOADSTOOL_DISCOVERY_HTTP_PORT` environment variable.
pub fn discovery_http_port() -> u16 {
    std::env::var("TOADSTOOL_DISCOVERY_HTTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DISCOVERY_HTTP_FALLBACK)
}

/// Discover any service by capability at runtime.
///
/// Primals only have self-knowledge and discover other primals at runtime.
/// This is the core discovery function — discover by capability (e.g., `security`,
/// `storage`), not by primal name. Use [`super::capabilities`] constants for standard
/// capability identifiers.
///
/// Tries multiple methods:
/// 1. Environment variables (TOADSTOOL_{CAPABILITY}_ENDPOINT)
/// 2. mDNS/DNS-SD discovery
/// 3. Kubernetes service discovery
/// 4. Docker Compose service names
/// 5. Runtime registry (consul, etcd)
///
/// For socket-based discovery returning `Option<String>`, use
/// [`super::discover_service_socket_by_capability`].
///
/// # Errors
///
/// Returns error if no service with the requested capability can be discovered
pub fn discover_service_by_capability(capability: &str) -> DiscoveryResult {
    info!("🔍 Discovering service with capability: {}", capability);

    // Try environment variable first (highest priority)
    let env_var = format!("TOADSTOOL_{}_ENDPOINT", capability.to_uppercase());
    if let Ok(endpoint) = std::env::var(&env_var) {
        debug!("✅ Found {} via {}: {}", capability, env_var, endpoint);
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-env"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true, // Assume healthy, will verify on first use
            last_check: std::time::SystemTime::now(),
        }]);
    }

    // Try generic TOADSTOOL_SERVICE_{NAME}_URL pattern
    let generic_var = format!("TOADSTOOL_SERVICE_{}_URL", capability.to_uppercase());
    if let Ok(endpoint) = std::env::var(&generic_var) {
        debug!("✅ Found {} via {}: {}", capability, generic_var, endpoint);
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-service"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }]);
    }

    // Try capability-based discovery methods (mDNS, Kubernetes, Docker Compose, Registry)
    if let Some(endpoints) = try_discover_via_mdns(capability) {
        debug!(
            "✅ Found {} via mDNS: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_kubernetes(capability) {
        debug!(
            "✅ Found {} via Kubernetes DNS: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_docker_compose(capability) {
        debug!(
            "✅ Found {} via Docker Compose: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }
    if let Some(endpoints) = try_discover_via_registry(capability) {
        debug!(
            "✅ Found {} via registry: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }

    // Try filesystem-based discovery (development mode)
    if let Some(endpoints) = try_discover_via_filesystem(capability) {
        debug!(
            "✅ Found {} via filesystem: {} endpoint(s)",
            capability,
            endpoints.len()
        );
        return Ok(endpoints);
    }

    // Fallback to configurable defaults for known capabilities
    if let Some(builtin) = builtin_default_endpoint(capability) {
        let default_var = format!(
            "TOADSTOOL_{}_DEFAULT_ENDPOINT",
            capability.to_uppercase().replace('-', "_")
        );
        let endpoint = std::env::var(&default_var).unwrap_or(builtin);

        warn!(
            "⚠️  No {} service found via discovery, using default: {} (override via {})",
            capability, endpoint, default_var
        );
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{capability}-default"),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }]);
    }

    warn!("⚠️  No {} service found via discovery", capability);
    Err(DiscoveryError::NoServiceFound {
        capability: capability.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Capability-keyed discovery (convenience wrappers)
// ---------------------------------------------------------------------------

/// Discover a security / encryption-capable service at runtime
///
/// Tries multiple discovery methods in order (see [`discover_service_by_capability`]),
/// including legacy env names such as `BEARDOG_ENDPOINT` where still supported by deployment.
///
/// # Errors
///
/// Returns error if no matching service can be discovered
pub fn discover_encryption_service() -> DiscoveryResult {
    discover_service_by_capability("encryption")
}

/// Discover a storage-capable service at runtime
///
/// Same discovery pipeline as [`discover_service_by_capability`], including legacy env
/// fallbacks (e.g. `NESTGATE_ENDPOINT`) where configured.
///
/// # Errors
///
/// Returns error if no matching service can be discovered
pub fn discover_storage_service() -> DiscoveryResult {
    discover_service_by_capability("storage")
}

/// Discover a coordination / mesh service at runtime
///
/// # Errors
///
/// Returns error if no matching service can be discovered
pub fn discover_coordination_service() -> DiscoveryResult {
    discover_service_by_capability("coordination")
}

/// Discover an MCP / agent routing service at runtime
///
/// # Errors
///
/// Returns error if no matching service can be discovered
pub fn discover_mcp_service() -> DiscoveryResult {
    discover_service_by_capability("mcp")
}

// ---------------------------------------------------------------------------
// External system discovery
// ---------------------------------------------------------------------------

/// Discover Redis cache service
///
/// # Errors
///
/// Returns error if no cache service can be discovered
pub fn discover_cache_service() -> DiscoveryResult {
    discover_service_by_capability("cache")
}

/// Discover `PostgreSQL` database service
///
/// # Errors
///
/// Returns error if no database service can be discovered
pub fn discover_database_service() -> DiscoveryResult {
    discover_service_by_capability("database")
}

/// Discover S3-compatible object storage
///
/// # Errors
///
/// Returns error if no object storage service can be discovered
pub fn discover_object_storage() -> DiscoveryResult {
    discover_service_by_capability("object-storage")
}

// ---------------------------------------------------------------------------
// Async discovery (non-blocking wrappers)
// ---------------------------------------------------------------------------

/// Async version of [`discover_service_by_capability`].
///
/// Offloads the blocking discovery backends (mDNS probe, DNS resolution,
/// TCP registry query) to a blocking thread pool via `spawn_blocking`,
/// keeping the async executor free.
///
/// # Errors
///
/// Returns error if no service with the requested capability can be discovered.
pub async fn discover_service_by_capability_async(capability: &str) -> DiscoveryResult {
    let cap = capability.to_string();
    tokio::task::spawn_blocking(move || discover_service_by_capability(&cap))
        .await
        .map_err(|e| DiscoveryError::NoServiceFound {
            capability: format!("{capability} (task join: {e})"),
        })?
}
