//! Inter-Primal Integration Discovery
//!
//! Runtime discovery of ecoPrimal services (bearDog, nestGate, songBird, squirrel).
//!
//! ## Deep Debt Principles
//!
//! 1. **Self-Knowledge Only**: ToadStool knows only itself
//! 2. **Runtime Discovery**: Other primals discovered at deployment time
//! 3. **No Hardcoding**: Zero hardcoded addresses/ports/names
//! 4. **Capability-Based**: Discover by capability, not by name
//! 5. **Graceful Degradation**: Works with or without other primals
//!
//! ## Integration Patterns
//!
//! ### bearDog Integration (Encryption)
//!
//! ```no_run
//! use toadstool_common::primal_integration::*;
//!
//! // Discover bearDog at runtime
//! let beardog = discover_encryption_service().await?;
//!
//! // Use encryption capability
//! let encrypted = beardog.encrypt(data).await?;
//! ```
//!
//! ### nestGate Integration (Compression/Persistence)
//!
//! ```no_run
//! use toadstool_common::primal_integration::*;
//!
//! // Discover nestGate at runtime
//! let nestgate = discover_storage_service().await?;
//!
//! // Use compression capability
//! let compressed = nestgate.compress(data).await?;
//!
//! // Use persistence capability
//! nestgate.store(key, value).await?;
//! ```
//!
//! ### songBird Integration (Coordination)
//!
//! ```no_run
//! use toadstool_common::primal_integration::*;
//!
//! // Discover songBird at runtime
//! let songbird = discover_coordination_service().await?;
//!
//! // Register capabilities
//! songbird.register_capabilities(capabilities).await?;
//! ```
//!
//! ## Discovery Methods
//!
//! 1. **Environment Variables** (highest priority)
//!    - `BEARDOG_ENDPOINT=http://beardog:6060`
//!    - `NESTGATE_ENDPOINT=http://nestgate:8080`
//!    - `SONGBIRD_ENDPOINT=http://songbird:9090`
//!    - `SQUIRREL_ENDPOINT=http://squirrel:7070`
//!
//! 2. **mDNS/DNS-SD** (local network)
//!    - `_encryption._tcp.local.` (bearDog)
//!    - `_storage._tcp.local.` (nestGate)
//!    - `_coordination._tcp.local.` (songBird)
//!
//! 3. **Kubernetes Service Discovery**
//!    - DNS: `beardog.default.svc.cluster.local`
//!    - DNS: `nestgate.default.svc.cluster.local`
//!    - DNS: `songbird.default.svc.cluster.local`
//!
//! 4. **Docker Compose Service Names**
//!    - `http://beardog:6060`
//!    - `http://nestgate:8080`
//!    - `http://songbird:9090`
//!
//! 5. **Runtime Registry** (consul, etcd)
//!    - Query by capability tag
//!    - Load balance across instances
//!
//! ## External System Integration
//!
//! ToadStool can also discover and use non-ecoPrimal services:
//!
//! - **Redis**: Cache, pub/sub
//! - **PostgreSQL**: Relational data
//! - **S3-compatible**: Object storage
//! - **Custom Services**: Via generic HTTP/gRPC discovery
//!
//! ```no_run
//! use toadstool_common::primal_integration::*;
//!
//! // Discover external services
//! let cache = discover_cache_service().await?;
//! let db = discover_database_service().await?;
//! let storage = discover_object_storage().await?;
//! ```

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Service endpoint discovered at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalEndpoint {
    /// Service identifier (e.g., "beardog-1", "nestgate-primary")
    pub service_id: String,

    /// Base URL (e.g., "http://beardog:6060")
    pub url: String,

    /// Capabilities this service provides
    pub capabilities: Vec<String>,

    /// Health status
    pub healthy: bool,

    /// Last health check timestamp
    pub last_check: std::time::SystemTime,
}

/// Discovery result with multiple endpoints (for load balancing)
pub type DiscoveryResult = Result<Vec<PrimalEndpoint>, DiscoveryError>;

/// Discovery errors
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("No service found with capability: {capability}")]
    NoServiceFound { capability: String },

    #[error("Service unhealthy: {service_id}")]
    ServiceUnhealthy { service_id: String },

    #[error("Discovery method failed: {method}: {reason}")]
    DiscoveryFailed { method: String, reason: String },

    #[error("Network error: {0}")]
    Network(String),
}

// ============================================================================
// bearDog Integration (Encryption)
// ============================================================================

/// Discover bearDog encryption service at runtime
///
/// Tries multiple discovery methods in order:
/// 1. BEARDOG_ENDPOINT environment variable
/// 2. mDNS (_encryption._tcp.local.)
/// 3. Kubernetes DNS (beardog.default.svc.cluster.local)
/// 4. Docker Compose (beardog:6060)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no bearDog service can be discovered
pub async fn discover_encryption_service() -> DiscoveryResult {
    discover_service_by_capability("encryption").await
}

/// Discover bearDog at specific path (for external/custom deployments)
pub async fn discover_beardog_at(base_path: &str) -> DiscoveryResult {
    // Check if bearDog is accessible at ../beardog/ (development)
    // Or at configured path (production)
    discover_filesystem_service(base_path, "beardog").await
}

// ============================================================================
// nestGate Integration (Compression/Persistence)
// ============================================================================

/// Discover nestGate storage service at runtime
///
/// Tries multiple discovery methods in order:
/// 1. NESTGATE_ENDPOINT environment variable
/// 2. mDNS (_storage._tcp.local.)
/// 3. Kubernetes DNS (nestgate.default.svc.cluster.local)
/// 4. Docker Compose (nestgate:8080)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no nestGate service can be discovered
pub async fn discover_storage_service() -> DiscoveryResult {
    discover_service_by_capability("storage").await
}

/// Discover nestGate at specific path (for external/custom deployments)
pub async fn discover_nestgate_at(base_path: &str) -> DiscoveryResult {
    // Check if nestGate is accessible at ../nestgate/ (development)
    // Or at configured path (production)
    discover_filesystem_service(base_path, "nestgate").await
}

// ============================================================================
// songBird Integration (Coordination)
// ============================================================================

/// Discover songBird coordination service at runtime
///
/// # Errors
///
/// Returns error if no songBird service can be discovered
pub async fn discover_coordination_service() -> DiscoveryResult {
    discover_service_by_capability("coordination").await
}

// ============================================================================
// squirrel Integration (MCP/Agents)
// ============================================================================

/// Discover squirrel MCP platform at runtime
///
/// # Errors
///
/// Returns error if no squirrel service can be discovered
pub async fn discover_mcp_service() -> DiscoveryResult {
    discover_service_by_capability("mcp").await
}

// ============================================================================
// Generic Discovery
// ============================================================================

/// Discover any service by capability at runtime
///
/// This is the core discovery function. Tries multiple methods:
/// 1. Environment variables (TOADSTOOL_{CAPABILITY}_ENDPOINT)
/// 2. mDNS/DNS-SD discovery
/// 3. Kubernetes service discovery
/// 4. Docker Compose service names
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no service with the requested capability can be discovered
pub async fn discover_service_by_capability(capability: &str) -> DiscoveryResult {
    info!("🔍 Discovering service with capability: {}", capability);

    // Try environment variable first (highest priority)
    let env_var = format!("TOADSTOOL_{}_ENDPOINT", capability.to_uppercase());
    if let Ok(endpoint) = std::env::var(&env_var) {
        debug!("✅ Found {} via {}: {}", capability, env_var, endpoint);
        return Ok(vec![PrimalEndpoint {
            service_id: format!("{}-env", capability),
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
            service_id: format!("{}-service", capability),
            url: endpoint,
            capabilities: vec![capability.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }]);
    }

    // TODO: Implement mDNS discovery (Phase 4)
    // TODO: Implement Kubernetes DNS discovery
    // TODO: Implement Docker Compose discovery
    // TODO: Implement registry discovery (consul, etcd)

    warn!("⚠️  No {} service found via discovery", capability);
    Err(DiscoveryError::NoServiceFound {
        capability: capability.to_string(),
    })
}

/// Discover service at filesystem path (development mode)
///
/// Checks if service binary/directory exists at relative path
async fn discover_filesystem_service(base_path: &str, service_name: &str) -> DiscoveryResult {
    let full_path = std::path::Path::new(base_path).join(service_name);

    if full_path.exists() {
        info!(
            "✅ Found {} at filesystem path: {:?}",
            service_name, full_path
        );
        Ok(vec![PrimalEndpoint {
            service_id: format!("{}-fs", service_name),
            url: format!("file://{}", full_path.display()),
            capabilities: vec![service_name.to_string()],
            healthy: true,
            last_check: std::time::SystemTime::now(),
        }])
    } else {
        Err(DiscoveryError::NoServiceFound {
            capability: service_name.to_string(),
        })
    }
}

// ============================================================================
// External System Discovery
// ============================================================================

/// Discover Redis cache service
pub async fn discover_cache_service() -> DiscoveryResult {
    discover_service_by_capability("cache").await
}

/// Discover PostgreSQL database service
pub async fn discover_database_service() -> DiscoveryResult {
    discover_service_by_capability("database").await
}

/// Discover S3-compatible object storage
pub async fn discover_object_storage() -> DiscoveryResult {
    discover_service_by_capability("object-storage").await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_var_discovery() {
        std::env::set_var("TOADSTOOL_ENCRYPTION_ENDPOINT", "http://beardog:6060");

        let result = discover_encryption_service().await;
        assert!(result.is_ok());

        let endpoints = result.unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].url, "http://beardog:6060");

        std::env::remove_var("TOADSTOOL_ENCRYPTION_ENDPOINT");
    }

    #[tokio::test]
    async fn test_no_service_found() {
        std::env::remove_var("TOADSTOOL_NONEXISTENT_ENDPOINT");

        let result = discover_service_by_capability("nonexistent").await;
        assert!(result.is_err());

        match result {
            Err(DiscoveryError::NoServiceFound { capability }) => {
                assert_eq!(capability, "nonexistent");
            }
            _ => panic!("Expected NoServiceFound error"),
        }
    }
}
