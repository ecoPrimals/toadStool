//! # Primal Unix Socket Discovery
//!
//! Pure Rust unix socket path resolution for primal-to-primal communication.
//!
//! ## TRUE PRIMAL Architecture
//!
//! - **No HTTP**: All primal communication via unix sockets
//! - **Songbird Handles External**: Only Songbird uses HTTP/TLS for external
//! - **Local IPC**: Fast, secure, pure Rust
//! - **Discovery-Based**: Socket paths from environment/runtime
//!
//! ## biomeOS Socket Standard (Jan 30, 2026)
//!
//! All primals use standardized socket paths for discovery and integration:
//! - **Standard Path**: `/run/user/$UID/biomeos/{primal}.sock`
//! - **Validated**: Tower Atomic (BearDog + Songbird) production-ready
//! - **Enables**: Node Atomic, Nest Atomic, Full NUCLEUS deployment

use std::path::PathBuf;

/// Get runtime directory for socket files
///
/// Priority:
/// 1. XDG_RUNTIME_DIR environment variable
/// 2. /run/user/<uid> (Linux standard)
/// 3. /tmp with username fallback (dev/testing only)
///
/// **TRUE PRIMAL**: Environment-based, no hardcoding
///
/// **EVOLVED**: Pure Rust UID detection (no unsafe, no libc!)
pub fn get_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| {
        // Try Linux standard path first - EVOLVED to pure Rust!
        if let Ok(uid) = crate::uid_detector::get_user_id() {
            let linux_standard = format!("/run/user/{}", uid);

            // Check if Linux standard path exists
            if std::path::Path::new(&linux_standard).exists() {
                return linux_standard;
            }
        }

        // Fallback to /tmp for dev/testing (containers, etc.)
        let username = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
        format!("/tmp/toadstool-runtime-{}", username)
    })
}

/// Get biomeos directory path (standard subdirectory for all primal sockets)
///
/// biomeOS socket standard: All sockets in `biomeos/` subdirectory
/// - Enables organized discovery
/// - Security: Proper permissions (0700)
/// - Integration: Predictable paths for all primals
pub fn get_biomeos_dir() -> PathBuf {
    let runtime_dir = get_runtime_dir();
    PathBuf::from(runtime_dir).join("biomeos")
}

/// Ensure biomeos directory exists with proper permissions
///
/// Creates the standard biomeos socket directory if it doesn't exist.
/// Sets permissions to 0700 (user-only access) for security.
///
/// **Used internally** before socket binding to ensure directory is ready.
pub fn ensure_biomeos_dir() -> std::io::Result<PathBuf> {
    let biomeos_dir = get_biomeos_dir();
    std::fs::create_dir_all(&biomeos_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o700);
        std::fs::set_permissions(&biomeos_dir, perms)?;
    }

    Ok(biomeos_dir)
}

/// Get family ID from environment
///
/// Priority:
/// 1. BIOMEOS_FAMILY_ID (orchestrator-provided)
/// 2. TOADSTOOL_FAMILY (instance-specific)
/// 3. "default" fallback
pub fn get_family_id() -> String {
    std::env::var("BIOMEOS_FAMILY_ID")
        .or_else(|_| std::env::var("TOADSTOOL_FAMILY"))
        .unwrap_or_else(|_| "default".to_string())
}

/// Get BearDog unix socket path (biomeOS standard)
///
/// **DEPRECATED**: Use `discover_crypto_socket()` for capability-based discovery.
///
/// This function violates Deep Debt principles by hardcoding the "beardog" primal name.
/// New code should discover crypto services by capability, not by hardcoded name.
///
/// # Migration Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_crypto_socket;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // OLD (hardcoded)
/// // let socket = get_beardog_socket_path();
///
/// // NEW (capability-based)
/// let socket = discover_crypto_socket().await?;
/// # Ok(())
/// # }
/// ```
///
/// Priority:
/// 1. BEARDOG_SOCKET environment variable (absolute path)
/// 2. biomeOS standard: `{runtime_dir}/biomeos/beardog.sock`
#[deprecated(
    since = "0.2.0",
    note = "Use discover_crypto_socket() for capability-based discovery. See docs for migration."
)]
pub fn get_beardog_socket_path() -> PathBuf {
    // Try new capability discovery in sync context
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if let Ok(path) = handle.block_on(discover_crypto_socket()) {
            return path;
        }
    }

    // Fallback to old behavior for backward compatibility
    if let Ok(socket) = std::env::var("BEARDOG_SOCKET") {
        return PathBuf::from(socket);
    }

    get_biomeos_dir().join("beardog.sock")
}

/// Get Songbird unix socket path (biomeOS standard)
///
/// **DEPRECATED**: Use `discover_coordination_socket()` for capability-based discovery.
///
/// This function violates Deep Debt principles by hardcoding the "songbird" primal name.
/// New code should discover coordination services by capability, not by hardcoded name.
///
/// # Migration Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_coordination_socket;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // OLD (hardcoded)
/// // let socket = get_songbird_socket_path();
///
/// // NEW (capability-based)
/// let socket = discover_coordination_socket().await?;
/// # Ok(())
/// # }
/// ```
///
/// Priority:
/// 1. SONGBIRD_SOCKET environment variable (absolute path)
/// 2. biomeOS standard: `{runtime_dir}/biomeos/songbird.sock`
#[deprecated(
    since = "0.2.0",
    note = "Use discover_coordination_socket() for capability-based discovery. See docs for migration."
)]
pub fn get_songbird_socket_path() -> PathBuf {
    // Try new capability discovery in sync context
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if let Ok(path) = handle.block_on(discover_coordination_socket()) {
            return path;
        }
    }

    // Fallback to old behavior for backward compatibility
    if let Ok(socket) = std::env::var("SONGBIRD_SOCKET") {
        return PathBuf::from(socket);
    }

    get_biomeos_dir().join("songbird.sock")
}

/// Get NestGate unix socket path (biomeOS standard)
///
/// **DEPRECATED**: Use `discover_storage_socket()` for capability-based discovery.
///
/// This function violates Deep Debt principles by hardcoding the "nestgate" primal name.
/// New code should discover storage services by capability, not by hardcoded name.
///
/// # Migration Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_storage_socket;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // OLD (hardcoded)
/// // let socket = get_nestgate_socket_path();
///
/// // NEW (capability-based)
/// let socket = discover_storage_socket().await?;
/// # Ok(())
/// # }
/// ```
///
/// Priority:
/// 1. NESTGATE_SOCKET environment variable (absolute path)
/// 2. biomeOS standard: `{runtime_dir}/biomeos/nestgate.sock`
#[deprecated(
    since = "0.2.0",
    note = "Use discover_storage_socket() for capability-based discovery. See docs for migration."
)]
pub fn get_nestgate_socket_path() -> PathBuf {
    // Try new capability discovery in sync context
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        if let Ok(path) = handle.block_on(discover_storage_socket()) {
            return path;
        }
    }

    // Fallback to old behavior for backward compatibility
    if let Ok(socket) = std::env::var("NESTGATE_SOCKET") {
        return PathBuf::from(socket);
    }

    get_biomeos_dir().join("nestgate.sock")
}

/// Get Squirrel unix socket path (biomeOS standard)
///
/// **Migration Path**: This function is deprecated for direct use, but still used internally
/// by `get_socket_path_for_service("squirrel")` for backward compatibility.
///
/// Priority:
/// 1. SQUIRREL_SOCKET environment variable (absolute path)
/// 2. biomeOS standard: `{runtime_dir}/biomeos/squirrel.sock`
///
/// **biomeOS Socket Standard**: Waiting for Squirrel team implementation
#[allow(deprecated)]
pub fn get_squirrel_socket_path() -> PathBuf {
    // Priority 1: Direct socket path (explicit override)
    if let Ok(socket) = std::env::var("SQUIRREL_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: biomeOS standard path
    get_biomeos_dir().join("squirrel.sock")
}

/// Get BiomeOS NUCLEUS socket path (biomeOS standard)
///
/// Priority:
/// 1. NUCLEUS_SOCKET environment variable (absolute path)
/// 2. BIOMEOS_SOCKET_PATH environment variable (orchestrator-provided)
/// 3. biomeOS standard: `{runtime_dir}/biomeos/nucleus.sock`
///
/// **biomeOS Socket Standard**: NUCLEUS orchestrator socket
pub fn get_nucleus_socket_path() -> PathBuf {
    // Priority 1: NUCLEUS-specific socket (explicit override)
    if let Ok(socket) = std::env::var("NUCLEUS_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: BiomeOS-provided socket path (orchestrator)
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        return PathBuf::from(socket);
    }

    // Priority 3: biomeOS standard path
    get_biomeos_dir().join("nucleus.sock")
}

/// Get ToadStool unix socket path (our own server) - biomeOS standard
///
/// Priority:
/// 1. TOADSTOOL_SOCKET environment variable (absolute path)
/// 2. BIOMEOS_SOCKET_PATH environment variable (orchestrator-provided)
/// 3. biomeOS standard: `{runtime_dir}/biomeos/toadstool.sock`
///
/// **biomeOS Socket Standard**: Enables Node Atomic (Tower + Toadstool)
pub fn get_toadstool_socket_path() -> PathBuf {
    // Priority 1: ToadStool-specific socket (explicit override)
    if let Ok(socket) = std::env::var("TOADSTOOL_SOCKET") {
        return PathBuf::from(socket);
    }

    // Priority 2: BiomeOS-provided socket path (orchestrator)
    if let Ok(socket) = std::env::var("BIOMEOS_SOCKET_PATH") {
        return PathBuf::from(socket);
    }

    // Priority 3: biomeOS standard path
    get_biomeos_dir().join("toadstool.sock")
}

/// Get socket path for any service by name (biomeOS standard)
///
/// **TRUE PRIMAL**: Generic socket resolution for ANY service
///
/// Maps service names to socket paths using biomeOS standard.
/// Falls back to generic pattern for unknown services.
///
/// This is the **preferred** method for socket path resolution as it:
/// - Works with ANY service name (discovered or known)
/// - Respects environment variables
/// - Has consistent fallback behavior
/// - **Uses biomeOS standard paths** for discovery and integration
///
/// **biomeOS Socket Standard**: All sockets in `biomeos/` subdirectory
#[allow(deprecated)] // Calls deprecated functions internally for backward compat
pub fn get_socket_path_for_service(service_name: &str) -> PathBuf {
    // Map known service names to specific socket paths (for environment variable support)
    match service_name.to_lowercase().as_str() {
        "beardog" | "bear-dog" => get_beardog_socket_path(),
        "songbird" | "song-bird" => get_songbird_socket_path(),
        "nestgate" | "nest-gate" => get_nestgate_socket_path(),
        "squirrel" => get_squirrel_socket_path(),
        "toadstool" | "toad-stool" => get_toadstool_socket_path(),
        "nucleus" | "biomeos" => get_nucleus_socket_path(),

        // Generic pattern for unknown services (TRUE PRIMAL - works with ANY service!)
        _ => {
            // Try service-specific environment variable first
            let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
            if let Ok(socket) = std::env::var(&env_var) {
                return PathBuf::from(socket);
            }

            // Fall back to biomeOS standard pattern (simple, no family suffix)
            get_biomeos_dir().join(format!("{}.sock", service_name.to_lowercase()))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// DEEP DEBT EVOLUTION: Capability-Based Discovery (Feb 2026)
// ═══════════════════════════════════════════════════════════════════════════
//
// NEW: Discover services by capability, not by hardcoded name.
// This eliminates the Deep Debt violation where primals have knowledge of
// other specific primals.
//
// Principle: Self-knowledge only. Discover others at runtime via capabilities.

/// Error type for socket discovery operations
#[derive(Debug, thiserror::Error)]
pub enum SocketDiscoveryError {
    #[error("Discovery service failed: {0}")]
    DiscoveryFailed(String),

    #[error("No Unix socket found for capability: {0:?}")]
    NoSocketFound(String),

    #[error("Invalid socket endpoint: {0}")]
    InvalidEndpoint(String),
}

/// Discover Unix socket path for a service providing the requested capability
///
/// **Deep Debt Compliant**: Discovers by capability, not by name.
/// Works with ANY service providing the requested capability.
///
/// This function queries the capability discovery system and returns the Unix
/// socket path for the first service that provides the requested capability.
///
/// # Arguments
///
/// * `capability` - The capability to search for (e.g., `Capability::Crypto(CryptoCapability::Encryption)`)
///
/// # Returns
///
/// * `Ok(PathBuf)` - Path to the Unix socket for a service providing this capability
/// * `Err(SocketDiscoveryError)` - If no service found or discovery fails
///
/// # Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_socket_for_capability;
/// use toadstool_common::primal_identity::{Capability, CryptoCapability};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Discover ANY crypto service (beardog, HSM, cloud KMS, etc.)
/// let socket = discover_socket_for_capability(
///     Capability::Crypto(CryptoCapability::Encryption)
/// ).await?;
/// println!("Found crypto service at: {}", socket.display());
/// # Ok(())
/// # }
/// ```
///
/// # Deep Debt Principles
///
/// - **Self-knowledge only**: No hardcoded primal names
/// - **Runtime discovery**: Services discovered at runtime
/// - **Capability-based**: Match by what they do, not who they are
/// - **Vendor agnostic**: Swap implementations without code changes
pub async fn discover_socket_for_capability(
    capability: crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::capability_discovery::CapabilityDiscovery;

    // Try capability-based discovery first
    let discovery = CapabilityDiscovery::new()
        .map_err(|e| SocketDiscoveryError::DiscoveryFailed(e.to_string()))?;

    let services = discovery
        .find_by_capability(capability.clone())
        .await
        .map_err(|e| SocketDiscoveryError::DiscoveryFailed(e.to_string()))?;

    // Find first service with Unix socket endpoint
    for service in services {
        for endpoint in &service.endpoints {
            // Check for Unix socket protocol
            if endpoint.protocol == "unix" {
                // Extract path from endpoint metadata
                if let Some(path_str) = endpoint.metadata.get("path") {
                    return Ok(PathBuf::from(path_str));
                }
                // Or construct from address if it's a path
                if endpoint.address.starts_with('/') {
                    return Ok(PathBuf::from(&endpoint.address));
                }
            }
        }
    }

    // Fallback: Try biomeOS standard path based on capability
    // This maintains backward compatibility during transition
    let fallback_path = capability_to_biomeos_fallback(&capability)?;
    if fallback_path.exists() {
        tracing::debug!(
            "Using fallback path for capability {:?}: {}",
            capability,
            fallback_path.display()
        );
        return Ok(fallback_path);
    }

    Err(SocketDiscoveryError::NoSocketFound(format!(
        "{capability:?}"
    )))
}

/// Convenience: Discover crypto service socket
///
/// Discovers ANY service providing encryption capability (beardog, HSM, KMS, etc.)
///
/// **Replaces**: `get_beardog_socket_path()` (Deep Debt violation)
///
/// # Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_crypto_socket;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let socket = discover_crypto_socket().await?;
/// // Use socket for crypto operations
/// # Ok(())
/// # }
/// ```
pub async fn discover_crypto_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CryptoCapability};
    discover_socket_for_capability(Capability::Crypto(CryptoCapability::Encryption)).await
}

/// Convenience: Discover storage service socket
///
/// Discovers ANY service providing object storage capability (nestgate, S3, etc.)
///
/// **Replaces**: `get_nestgate_socket_path()` (Deep Debt violation)
///
/// # Example
///
/// ```rust,no_run
/// use toadstool_common::primal_sockets::discover_storage_socket;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let socket = discover_storage_socket().await?;
/// // Use socket for storage operations
/// # Ok(())
/// # }
/// ```
pub async fn discover_storage_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, StorageCapability};
    discover_socket_for_capability(Capability::Storage(StorageCapability::ObjectStorage)).await
}

/// Convenience: Discover coordination service socket
///
/// Discovers ANY service providing coordination capability (songbird, k8s, consul, etc.)
///
/// **Replaces**: `get_songbird_socket_path()` (Deep Debt violation)
pub async fn discover_coordination_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CoordinationCapability};
    discover_socket_for_capability(Capability::Coordination(
        CoordinationCapability::ServiceDiscovery,
    ))
    .await
}

/// Helper: Map capability to biomeOS standard path (fallback during transition)
///
/// This is a transition helper that will be removed once all services
/// properly register their capabilities with the discovery system.
fn capability_to_biomeos_fallback(
    capability: &crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::Capability;

    // Map capabilities to service names for biomeOS standard paths
    // This is ONLY a fallback - eventually all services will register properly
    let service_name = match capability {
        Capability::Crypto(_) => "beardog",
        Capability::Storage(_) => "nestgate",
        Capability::Coordination(_) => "songbird",
        Capability::Compute(_) => "toadstool",
        _ => {
            return Err(SocketDiscoveryError::NoSocketFound(format!(
                "No fallback path for capability: {capability:?}"
            )))
        }
    };

    Ok(get_socket_path_for_service(service_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_dir_from_xdg() {
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        assert_eq!(get_runtime_dir(), "/run/user/1000");
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn test_runtime_dir_fallback() {
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::set_var("USER", "testuser");

        // New behavior: Tries Linux standard /run/user/<uid> first
        // If that doesn't exist, falls back to /tmp/toadstool-runtime-<username>
        let runtime_dir = get_runtime_dir();

        // Could be either /run/user/<uid> (if it exists) or /tmp fallback
        assert!(
            runtime_dir.starts_with("/run/user/")
                || runtime_dir == "/tmp/toadstool-runtime-testuser",
            "Expected /run/user/<uid> or /tmp fallback, got: {}",
            runtime_dir
        );

        std::env::remove_var("USER");
    }

    #[test]
    fn test_beardog_socket_from_env() {
        std::env::set_var("BEARDOG_SOCKET", "/custom/beardog.sock");
        assert_eq!(
            get_beardog_socket_path(),
            PathBuf::from("/custom/beardog.sock")
        );
        std::env::remove_var("BEARDOG_SOCKET");
    }

    #[test]
    fn test_beardog_socket_default() {
        std::env::remove_var("BEARDOG_SOCKET");
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        std::env::set_var("BIOMEOS_FAMILY_ID", "nat0");

        let path = get_beardog_socket_path();
        // Updated for biomeOS socket standard
        assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));

        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::remove_var("BIOMEOS_FAMILY_ID");
    }

    #[test]
    fn test_songbird_socket_biomeos_standard() {
        std::env::remove_var("SONGBIRD_SOCKET");
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");

        let path = get_songbird_socket_path();
        // biomeOS standard: uses biomeos subdirectory
        assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/songbird.sock"));

        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn test_toadstool_socket_biomeos_standard() {
        std::env::remove_var("TOADSTOOL_SOCKET");
        std::env::remove_var("BIOMEOS_SOCKET_PATH");
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");

        let path = get_toadstool_socket_path();
        // biomeOS standard: uses biomeos subdirectory
        assert_eq!(path, PathBuf::from("/run/user/1000/biomeos/toadstool.sock"));

        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn test_biomeos_directory_path() {
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");

        let biomeos_dir = get_biomeos_dir();
        assert_eq!(biomeos_dir, PathBuf::from("/run/user/1000/biomeos"));

        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn test_all_primals_have_unique_paths() {
        let beardog = get_beardog_socket_path();
        let songbird = get_songbird_socket_path();
        let nestgate = get_nestgate_socket_path();
        let squirrel = get_squirrel_socket_path();
        let toadstool = get_toadstool_socket_path();

        // All should be different (different filenames)
        assert_ne!(beardog, songbird);
        assert_ne!(beardog, nestgate);
        assert_ne!(beardog, squirrel);
        assert_ne!(beardog, toadstool);
        assert_ne!(songbird, nestgate);
        assert_ne!(songbird, squirrel);
        assert_ne!(songbird, toadstool);
        assert_ne!(nestgate, squirrel);
        assert_ne!(nestgate, toadstool);
        assert_ne!(squirrel, toadstool);

        // All should be in biomeos subdirectory
        assert!(beardog.to_str().unwrap().contains("/biomeos/"));
        assert!(songbird.to_str().unwrap().contains("/biomeos/"));
        assert!(nestgate.to_str().unwrap().contains("/biomeos/"));
        assert!(squirrel.to_str().unwrap().contains("/biomeos/"));
        assert!(toadstool.to_str().unwrap().contains("/biomeos/"));
    }
}
