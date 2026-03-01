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
//! - **`PostgreSQL`**: Relational data
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

/// Service endpoint discovered at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalEndpoint {
    /// Service identifier (e.g., "beardog-1", "nestgate-primary")
    pub service_id: String,

    /// Base URL; discovered via capability resolution at runtime.
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
/// 1. `BEARDOG_ENDPOINT` environment variable
/// 2. mDNS (_encryption._tcp.local.)
/// 3. Kubernetes DNS (beardog.default.svc.cluster.local)
/// 4. Docker Compose (beardog:6060)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no bearDog service can be discovered
pub fn discover_encryption_service() -> DiscoveryResult {
    discover_service_by_capability("encryption")
}

/// Discover encryption service at a specific filesystem base path.
///
/// **Capability-based resolution** — toadStool has no knowledge of which primal
/// provides encryption. It discovers by capability, not by name:
///
/// 1. `TOADSTOOL_CRYPTO_SERVICE_SUBDIR` env var (explicit deployment override)
/// 2. Canonical capability subdirectory `"security"` (any service may publish here)
///
/// # Errors
///
/// Returns error if service not found at the given path
pub fn discover_beardog_at(base_path: &str) -> DiscoveryResult {
    // Default to "beardog" — the primal's canonical filesystem directory name.
    // Override via TOADSTOOL_CRYPTO_SERVICE_SUBDIR for custom layouts.
    let subdir = std::env::var("TOADSTOOL_CRYPTO_SERVICE_SUBDIR")
        .unwrap_or_else(|_| crate::constants::ecosystem::well_known::BEARDOG.to_string());

    discover_filesystem_service(base_path, &subdir)
}

// ============================================================================
// nestGate Integration (Compression/Persistence)
// ============================================================================

/// Discover nestGate storage service at runtime
///
/// Tries multiple discovery methods in order:
/// 1. `NESTGATE_ENDPOINT` environment variable
/// 2. mDNS (_storage._tcp.local.)
/// 3. Kubernetes DNS (nestgate.default.svc.cluster.local)
/// 4. Docker Compose (nestgate:8080)
/// 5. Runtime registry (consul, etcd)
///
/// # Errors
///
/// Returns error if no nestGate service can be discovered
pub fn discover_storage_service() -> DiscoveryResult {
    discover_service_by_capability("storage")
}

/// Discover storage service at a specific filesystem base path.
///
/// **Capability-based resolution** — toadStool has no knowledge of which primal
/// provides storage. It discovers by capability, not by name:
///
/// 1. `TOADSTOOL_STORAGE_SERVICE_SUBDIR` env var (explicit deployment override)
/// 2. Canonical capability subdirectory `"storage"` (any service may publish here)
///
/// # Errors
///
/// Returns error if service not found at the given path
pub fn discover_nestgate_at(base_path: &str) -> DiscoveryResult {
    // Default to "nestgate" — the primal's canonical filesystem directory name.
    // Override via TOADSTOOL_STORAGE_SERVICE_SUBDIR for custom layouts.
    let subdir = std::env::var("TOADSTOOL_STORAGE_SERVICE_SUBDIR")
        .unwrap_or_else(|_| crate::constants::ecosystem::well_known::NESTGATE.to_string());

    discover_filesystem_service(base_path, &subdir)
}

// ============================================================================
// songBird Integration (Coordination)
// ============================================================================

/// Discover songBird coordination service at runtime
///
/// # Errors
///
/// Returns error if no songBird service can be discovered
pub fn discover_coordination_service() -> DiscoveryResult {
    discover_service_by_capability("coordination")
}

// ============================================================================
// squirrel Integration (MCP/Agents)
// ============================================================================

/// Discover squirrel MCP platform at runtime
///
/// # Errors
///
/// Returns error if no squirrel service can be discovered
pub fn discover_mcp_service() -> DiscoveryResult {
    discover_service_by_capability("mcp")
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

/// Built-in default endpoints for known capabilities.
/// Returns `None` for all capabilities — discovered via capability resolution at runtime.
/// Caller sets `TOADSTOOL_{CAPABILITY}_ENDPOINT` or discovers via mDNS/registry.
fn builtin_default_endpoint(_capability: &str) -> Option<String> {
    None
}

/// Discover service at filesystem path (development mode)
///
/// Checks if service binary/directory exists at relative path
fn discover_filesystem_service(base_path: &str, service_name: &str) -> DiscoveryResult {
    let full_path = std::path::Path::new(base_path).join(service_name);

    if full_path.exists() {
        info!(
            "✅ Found {} at filesystem path: {:?}",
            service_name, full_path
        );
        Ok(vec![PrimalEndpoint {
            service_id: format!("{service_name}-fs"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_var_discovery() {
        temp_env::with_var(
            "TOADSTOOL_ENCRYPTION_ENDPOINT",
            Some("http://beardog:6060"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_encryption_service();
                        assert!(result.is_ok());
                        let endpoints = result.unwrap();
                        assert_eq!(endpoints.len(), 1);
                        assert_eq!(endpoints[0].url, "http://beardog:6060");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_encryption_fallback_default() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENCRYPTION_ENDPOINT", None::<&str>),
                ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
                ("TOADSTOOL_ENCRYPTION_DEFAULT_ENDPOINT", None::<&str>),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_encryption_service();
                        assert!(
                            result.is_err(),
                            "encryption discovery must fail when no env/discovery provides endpoint"
                        );
                        assert!(matches!(
                            result.unwrap_err(),
                            DiscoveryError::NoServiceFound { capability } if capability == "encryption"
                        ));
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_encryption_explicit_endpoint() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_SERVICE_ENCRYPTION_URL", None::<&str>),
                (
                    "TOADSTOOL_ENCRYPTION_ENDPOINT",
                    Some("http://custom-beardog:9090"),
                ),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_encryption_service();
                        assert!(result.is_ok());
                        let endpoints = result.unwrap();
                        assert_eq!(endpoints[0].url, "http://custom-beardog:9090");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_no_service_found() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_NONEXISTENT_ENDPOINT", None::<&str>),
                ("TOADSTOOL_SERVICE_NONEXISTENT_URL", None::<&str>),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_service_by_capability("nonexistent");
                        assert!(result.is_err());
                        match result {
                            Err(DiscoveryError::NoServiceFound { capability }) => {
                                assert_eq!(capability, "nonexistent");
                            }
                            _ => panic!("Expected NoServiceFound error"),
                        }
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_generic_service_url_env_var() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_CACHE_ENDPOINT", None::<&str>),
                ("TOADSTOOL_SERVICE_CACHE_URL", Some("http://redis:6379")),
            ],
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_service_by_capability("cache");
                        assert!(result.is_ok());
                        let endpoints = result.unwrap();
                        assert_eq!(endpoints.len(), 1);
                        assert_eq!(endpoints[0].url, "http://redis:6379");
                        assert_eq!(endpoints[0].service_id, "cache-service");
                        assert_eq!(endpoints[0].capabilities, vec!["cache"]);
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_storage_service_via_env() {
        temp_env::with_var(
            "TOADSTOOL_STORAGE_ENDPOINT",
            Some("http://nestgate:8080"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_storage_service();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "http://nestgate:8080");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_coordination_service_via_env() {
        temp_env::with_var(
            "TOADSTOOL_COORDINATION_ENDPOINT",
            Some("http://songbird:6061"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_coordination_service();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "http://songbird:6061");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_mcp_service_via_env() {
        temp_env::with_var(
            "TOADSTOOL_MCP_ENDPOINT",
            Some("http://squirrel:6062"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_mcp_service();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "http://squirrel:6062");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_cache_service_via_env() {
        temp_env::with_var(
            "TOADSTOOL_CACHE_ENDPOINT",
            Some("redis://localhost:6379"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_cache_service();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "redis://localhost:6379");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_database_service_via_env() {
        temp_env::with_var(
            "TOADSTOOL_DATABASE_ENDPOINT",
            Some("postgres://localhost:5432"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_database_service();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "postgres://localhost:5432");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_object_storage_via_env() {
        temp_env::with_var(
            "TOADSTOOL_OBJECT-STORAGE_ENDPOINT",
            Some("https://s3.example.com"),
            || {
                std::thread::spawn(|| {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async {
                        let result = discover_object_storage();
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].url, "https://s3.example.com");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
    }

    #[tokio::test]
    async fn test_discover_beardog_at_filesystem_exists() {
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_beardog_test");
        let beardog_subdir = base.join("beardog");
        let _ = std::fs::create_dir_all(&beardog_subdir);
        let base_str = base.to_str().unwrap().to_string();

        temp_env::with_var_unset("TOADSTOOL_CRYPTO_SERVICE_SUBDIR", || {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async move {
                    let result = discover_beardog_at(&base_str);
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].service_id, "beardog-fs");
                });
            })
            .join()
            .expect("test thread");
        });
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_nestgate_at_filesystem_exists() {
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_nestgate_test");
        let nestgate_subdir = base.join("nestgate");
        let _ = std::fs::create_dir_all(&nestgate_subdir);
        let base_str = base.to_str().unwrap().to_string();

        temp_env::with_var_unset("TOADSTOOL_STORAGE_SERVICE_SUBDIR", || {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async move {
                    let result = discover_nestgate_at(&base_str);
                    assert!(result.is_ok());
                    assert_eq!(result.unwrap()[0].service_id, "nestgate-fs");
                });
            })
            .join()
            .expect("test thread");
        });
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_beardog_at_custom_subdir() {
        let temp_dir = std::env::temp_dir();
        let base = temp_dir.join("toadstool_custom_crypto_test");
        let custom_dir = base.join("custom_crypto");
        let _ = std::fs::create_dir_all(&custom_dir);
        let base_str = base.to_str().unwrap().to_string();

        temp_env::with_var(
            "TOADSTOOL_CRYPTO_SERVICE_SUBDIR",
            Some("custom_crypto"),
            || {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("runtime");
                    rt.block_on(async move {
                        let result = discover_beardog_at(&base_str);
                        assert!(result.is_ok());
                        assert_eq!(result.unwrap()[0].service_id, "custom_crypto-fs");
                    });
                })
                .join()
                .expect("test thread");
            },
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn test_discover_beardog_at_not_found() {
        temp_env::with_var_unset("TOADSTOOL_CRYPTO_SERVICE_SUBDIR", || {
            std::thread::spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("runtime");
                rt.block_on(async {
                    let result = discover_beardog_at("/nonexistent/path/12345");
                    assert!(result.is_err());
                });
            })
            .join()
            .expect("test thread");
        });
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
