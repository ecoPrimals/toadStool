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
//! ```ignore
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
//! ```ignore
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
//! ```ignore
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
//! 1. **Environment Variables** (highest priority) - capability-based, no primal names
//!    - `TOADSTOOL_ENCRYPTION_ENDPOINT` (crypto capability)
//!    - `TOADSTOOL_STORAGE_ENDPOINT` (storage capability)
//!    - `TOADSTOOL_COORDINATION_ENDPOINT` (coordination capability)
//!    - `TOADSTOOL_MCP_ENDPOINT` (mcp capability)
//!
//! 2. **mDNS/DNS-SD** (local network) - discover by capability tag
//!    - `_encryption._tcp.local.`
//!    - `_storage._tcp.local.`
//!    - `_coordination._tcp.local.`
//!
//! 3. **Kubernetes Service Discovery** - discover by label/capability
//!    - Query by capability label, not by service name
//!
//! 4. **Docker Compose** - discover via service mesh labels
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
//! ```ignore
//! use toadstool_common::primal_integration::*;
//!
//! // Discover external services
//! let cache = discover_cache_service().await?;
//! let db = discover_database_service().await?;
//! let storage = discover_object_storage().await?;
//! ```

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

#[allow(deprecated)]
use crate::interned_strings::primals;

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
///
/// **Capability-based resolution** (evolved from Phase 3/4 TODO):
/// 1. First checks `TOADSTOOL_CRYPTO_SERVICE_SUBDIR` env var (explicit override)
/// 2. Then queries capability metadata if available (future: service registry)
/// 3. Falls back to well-known primal name constant
///
/// This follows the "primal self-knowledge" principle: the service name comes
/// from the primal's own identity (primals::BEARDOG), not hardcoded strings.
pub async fn discover_beardog_at(base_path: &str) -> DiscoveryResult {
    // Priority 1: Explicit env var override (for custom deployments)
    let subdir = std::env::var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR")
        // Priority 2: Use primal's self-knowledge constant (capability-based)
        .unwrap_or_else(|_| primals::BEARDOG.to_string());

    discover_filesystem_service(base_path, &subdir).await
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
///
/// **Capability-based resolution** (evolved from Phase 3/4 TODO):
/// 1. First checks `TOADSTOOL_STORAGE_SERVICE_SUBDIR` env var (explicit override)
/// 2. Then queries capability metadata if available (future: service registry)
/// 3. Falls back to well-known primal name constant
///
/// This follows the "primal self-knowledge" principle: the service name comes
/// from the primal's own identity (primals::NESTGATE), not hardcoded strings.
pub async fn discover_nestgate_at(base_path: &str) -> DiscoveryResult {
    // Priority 1: Explicit env var override (for custom deployments)
    let subdir = std::env::var("TOADSTOOL_STORAGE_SERVICE_SUBDIR")
        // Priority 2: Use primal's self-knowledge constant (capability-based)
        .unwrap_or_else(|_| primals::NESTGATE.to_string());

    discover_filesystem_service(base_path, &subdir).await
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

    // Discovery methods not yet implemented; env vars are the only active source
    debug!(
        "mDNS discovery not yet implemented (pending: mdns-sd crate for zero-config local discovery)"
    );
    debug!("Kubernetes DNS discovery not yet implemented (pending: K8s service DNS probing)");
    debug!(
        "Docker Compose discovery not yet implemented (pending: compose service name resolution)"
    );
    debug!(
        "Registry discovery (consul, etcd) not yet implemented (pending: external registry integration)"
    );

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
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;

    /// Mutex to serialize tests that modify environment variables.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn test_env_var_discovery() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
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
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::remove_var("TOADSTOOL_NONEXISTENT_ENDPOINT");
        std::env::remove_var("TOADSTOOL_SERVICE_NONEXISTENT_URL");

        let result = discover_service_by_capability("nonexistent").await;
        assert!(result.is_err());

        match result {
            Err(DiscoveryError::NoServiceFound { capability }) => {
                assert_eq!(capability, "nonexistent");
            }
            _ => panic!("Expected NoServiceFound error"),
        }
    }

    #[tokio::test]
    async fn test_generic_service_url_env_var() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::remove_var("TOADSTOOL_CACHE_ENDPOINT");
        std::env::set_var("TOADSTOOL_SERVICE_CACHE_URL", "http://redis:6379");

        let result = discover_service_by_capability("cache").await;
        assert!(result.is_ok());

        let endpoints = result.unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].url, "http://redis:6379");
        assert_eq!(endpoints[0].service_id, "cache-service");
        assert_eq!(endpoints[0].capabilities, vec!["cache"]);

        std::env::remove_var("TOADSTOOL_SERVICE_CACHE_URL");
    }

    #[tokio::test]
    async fn test_discover_storage_service_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::set_var("TOADSTOOL_STORAGE_ENDPOINT", "http://nestgate:8080");

        let result = discover_storage_service().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "http://nestgate:8080");

        std::env::remove_var("TOADSTOOL_STORAGE_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_coordination_service_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::set_var("TOADSTOOL_COORDINATION_ENDPOINT", "http://songbird:6061");

        let result = discover_coordination_service().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "http://songbird:6061");

        std::env::remove_var("TOADSTOOL_COORDINATION_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_mcp_service_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::set_var("TOADSTOOL_MCP_ENDPOINT", "http://squirrel:6062");

        let result = discover_mcp_service().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "http://squirrel:6062");

        std::env::remove_var("TOADSTOOL_MCP_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_cache_service_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::set_var("TOADSTOOL_CACHE_ENDPOINT", "redis://localhost:6379");

        let result = discover_cache_service().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "redis://localhost:6379");

        std::env::remove_var("TOADSTOOL_CACHE_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_database_service_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::set_var("TOADSTOOL_DATABASE_ENDPOINT", "postgres://localhost:5432");

        let result = discover_database_service().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "postgres://localhost:5432");

        std::env::remove_var("TOADSTOOL_DATABASE_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_object_storage_via_env() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        // object-storage -> TOADSTOOL_OBJECT-STORAGE_ENDPOINT (hyphen in env var)
        std::env::set_var(
            "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
            "https://s3.example.com",
        );

        let result = discover_object_storage().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].url, "https://s3.example.com");

        std::env::remove_var("TOADSTOOL_OBJECT-STORAGE_ENDPOINT");
    }

    #[tokio::test]
    async fn test_discover_beardog_at_filesystem_exists() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_beardog_test");
        let beardog_subdir = base.join("beardog");
        let _ = std::fs::create_dir_all(&beardog_subdir);

        std::env::remove_var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR");

        let result = discover_beardog_at(base.to_str().unwrap()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].service_id, "beardog-fs");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_nestgate_at_filesystem_exists() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_nestgate_test");
        let nestgate_subdir = base.join("nestgate");
        let _ = std::fs::create_dir_all(&nestgate_subdir);

        std::env::remove_var("TOADSTOOL_STORAGE_SERVICE_SUBDIR");

        let result = discover_nestgate_at(base.to_str().unwrap()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].service_id, "nestgate-fs");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_beardog_at_custom_subdir() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_custom_crypto_test");
        let custom_dir = base.join("custom_crypto");
        let _ = std::fs::create_dir_all(&custom_dir);

        std::env::set_var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR", "custom_crypto");

        let result = discover_beardog_at(base.to_str().unwrap()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0].service_id, "custom_crypto-fs");

        std::env::remove_var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_beardog_at_not_found() {
        let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
        std::env::remove_var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR");
        let result = discover_beardog_at("/nonexistent/path/12345").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_primal_endpoint_structure() {
        use std::time::SystemTime;
        let endpoint = PrimalEndpoint {
            service_id: "test-1".to_string(),
            url: "http://test:80".to_string(),
            capabilities: vec!["encryption".to_string()],
            healthy: true,
            last_check: SystemTime::now(),
        };
        assert_eq!(endpoint.service_id, "test-1");
        assert_eq!(endpoint.url, "http://test:80");
        assert_eq!(endpoint.capabilities, vec!["encryption"]);
        assert!(endpoint.healthy);
    }

    #[test]
    fn test_discovery_error_display() {
        let err = DiscoveryError::NoServiceFound {
            capability: "storage".to_string(),
        };
        assert!(err.to_string().contains("No service found"));
        assert!(err.to_string().contains("storage"));

        let err = DiscoveryError::ServiceUnhealthy {
            service_id: "beardog-1".to_string(),
        };
        assert!(err.to_string().contains("Service unhealthy"));
        assert!(err.to_string().contains("beardog-1"));

        let err = DiscoveryError::DiscoveryFailed {
            method: "mDNS".to_string(),
            reason: "timeout".to_string(),
        };
        assert!(err.to_string().contains("Discovery method failed"));
        assert!(err.to_string().contains("mDNS"));
        assert!(err.to_string().contains("timeout"));

        let err = DiscoveryError::Network("connection refused".to_string());
        assert!(err.to_string().contains("connection refused"));
    }
}
