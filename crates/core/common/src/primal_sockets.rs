//! # Primal Unix Socket Discovery
//!
//! Pure Rust unix socket path resolution for primal-to-primal communication.
//!
//! ## Fallback Constants (Transition Period)
//!
//! The `resolve_*_socket_fallback` and `resolve_socket_path_for_service` functions
//! use biomeOS standard socket filenames (e.g. `beardog.sock`, `songbird.sock`).
//! These are **fallback constants** for the transition period until full
//! capability-based discovery is deployed. Production systems should use
//! `discover_socket_for_capability()` or `{PRIMAL}_SOCKET` env vars.
//! Self-identification as `toadstool` is self-knowledge (allowed).
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

#[allow(deprecated)]
use crate::interned_strings::primals;

// ═══════════════════════════════════════════════════════════════════════════
// Fallback Constants (Transition Period)
// Pending: Remove once all primals register with capability discovery (Phase 3/4)
// ═══════════════════════════════════════════════════════════════════════════
//
// Used when capability discovery finds no service. Prefer `{PRIMAL}_SOCKET` env
// or `discover_socket_for_capability()`. Self-knowledge: "toadstool" is OK.

/// Fallback socket filename for crypto capability (env: BEARDOG_SOCKET overrides)
const FALLBACK_CRYPTO_SOCKET: &str = "beardog.sock";
/// Fallback socket filename for coordination capability (env: SONGBIRD_SOCKET overrides)
const FALLBACK_COORDINATION_SOCKET: &str = "songbird.sock";
/// Fallback socket filename for storage capability (env: NESTGATE_SOCKET overrides)
const FALLBACK_STORAGE_SOCKET: &str = "nestgate.sock";
/// Fallback socket filename for MCP capability (env: SQUIRREL_SOCKET overrides)
const FALLBACK_MCP_SOCKET: &str = "squirrel.sock";
/// Self-knowledge: ToadStool's own socket (env: TOADSTOOL_SOCKET overrides)
const SELF_SOCKET: &str = "toadstool.sock";

// ═══════════════════════════════════════════════════════════════════════════
// Environment Snapshot (Deep Fix for Concurrency)
// ═══════════════════════════════════════════════════════════════════════════
//
// Production code creates this via `SocketPathEnv::from_env()`.
// Tests create this with explicit values - no env var mutation needed.
// This eliminates the ENV_MUTEX anti-pattern.

/// Environment snapshot for socket path resolution.
///
/// Production code creates this via `SocketPathEnv::from_env()`.
/// Tests create this with explicit values - no env var mutation needed.
#[derive(Debug, Clone, Default)]
pub struct SocketPathEnv {
    pub xdg_runtime_dir: Option<String>,
    pub user: Option<String>,
    pub biomeos_family_id: Option<String>,
    pub beardog_socket: Option<String>,
    pub songbird_socket: Option<String>,
    pub nestgate_socket: Option<String>,
    pub squirrel_socket: Option<String>,
    pub toadstool_socket: Option<String>,
    pub biomeos_socket_path: Option<String>,
    pub nucleus_socket: Option<String>,
}

impl SocketPathEnv {
    /// Capture current environment (production use)
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            xdg_runtime_dir: std::env::var("XDG_RUNTIME_DIR").ok(),
            user: std::env::var("USER").ok(),
            biomeos_family_id: std::env::var("BIOMEOS_FAMILY_ID")
                .or_else(|_| std::env::var("TOADSTOOL_FAMILY"))
                .ok(),
            beardog_socket: std::env::var("BEARDOG_SOCKET").ok(),
            songbird_socket: std::env::var("SONGBIRD_SOCKET").ok(),
            nestgate_socket: std::env::var("NESTGATE_SOCKET").ok(),
            squirrel_socket: std::env::var("SQUIRREL_SOCKET").ok(),
            toadstool_socket: std::env::var("TOADSTOOL_SOCKET").ok(),
            biomeos_socket_path: std::env::var("BIOMEOS_SOCKET_PATH").ok(),
            nucleus_socket: std::env::var("NUCLEUS_SOCKET").ok(),
        }
    }

    /// Create for testing with a specific runtime dir
    #[cfg(test)]
    #[must_use]
    pub fn with_runtime_dir(dir: &str) -> Self {
        Self {
            xdg_runtime_dir: Some(dir.to_string()),
            ..Default::default()
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Pure resolution functions (no env access - inject SocketPathEnv)
// ═══════════════════════════════════════════════════════════════════════════

/// Pure logic: resolve runtime dir from environment snapshot
#[must_use]
pub fn resolve_runtime_dir(env: &SocketPathEnv) -> String {
    if let Some(ref xdg) = env.xdg_runtime_dir {
        return xdg.clone();
    }
    // Try Linux standard path
    if let Ok(uid) = crate::uid_detector::get_user_id() {
        let linux_standard = format!("/run/user/{}", uid);
        if std::path::Path::new(&linux_standard).exists() {
            return linux_standard;
        }
    }
    let username = env.user.as_deref().unwrap_or("default");
    format!("/tmp/toadstool-runtime-{}", username)
}

/// Pure logic: resolve biomeos dir
#[must_use]
pub fn resolve_biomeos_dir(env: &SocketPathEnv) -> PathBuf {
    PathBuf::from(resolve_runtime_dir(env)).join("biomeos")
}

/// Pure logic: resolve family ID from environment snapshot
#[must_use]
pub fn resolve_family_id(env: &SocketPathEnv) -> String {
    env.biomeos_family_id
        .clone()
        .unwrap_or_else(|| "default".to_string())
}

/// Pure logic: resolve beardog socket fallback (non-discovery path)
#[must_use]
pub fn resolve_beardog_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.beardog_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_CRYPTO_SOCKET)
}

/// Pure logic: resolve songbird socket fallback (non-discovery path)
#[must_use]
pub fn resolve_songbird_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.songbird_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_COORDINATION_SOCKET)
}

/// Pure logic: resolve nestgate socket fallback (non-discovery path)
#[must_use]
pub fn resolve_nestgate_socket_fallback(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.nestgate_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_STORAGE_SOCKET)
}

/// Pure logic: resolve squirrel socket
#[must_use]
pub fn resolve_squirrel_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.squirrel_socket {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(FALLBACK_MCP_SOCKET)
}

/// Pure logic: resolve nucleus socket
#[must_use]
pub fn resolve_nucleus_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.nucleus_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join("nucleus.sock")
}

/// Pure logic: resolve toadstool socket
#[must_use]
pub fn resolve_toadstool_socket(env: &SocketPathEnv) -> PathBuf {
    if let Some(ref socket) = env.toadstool_socket {
        return PathBuf::from(socket);
    }
    if let Some(ref socket) = env.biomeos_socket_path {
        return PathBuf::from(socket);
    }
    resolve_biomeos_dir(env).join(SELF_SOCKET)
}

/// Pure logic: resolve socket path for any service by name
///
/// `service_socket_override` is for the generic `{SERVICE}_SOCKET` env var case;
/// pass `Some(path)` when the wrapper found an override, `None` otherwise.
#[must_use]
#[allow(deprecated)] // primals::* constants used for transition-period fallback matching
pub fn resolve_socket_path_for_service(
    service_name: &str,
    env: &SocketPathEnv,
    service_socket_override: Option<PathBuf>,
) -> PathBuf {
    if let Some(path) = service_socket_override {
        return path;
    }
    // Alias matching uses fallback constants (transition period - see module doc)
    match service_name.to_lowercase().as_str() {
        s if s == primals::BEARDOG || s == "bear-dog" => resolve_beardog_socket_fallback(env),
        s if s == primals::SONGBIRD || s == "song-bird" => resolve_songbird_socket_fallback(env),
        s if s == primals::NESTGATE || s == "nest-gate" => resolve_nestgate_socket_fallback(env),
        s if s == primals::SQUIRREL => resolve_squirrel_socket(env),
        s if s == primals::TOADSTOOL || s == "toad-stool" => resolve_toadstool_socket(env), // Self-knowledge OK
        "nucleus" | "biomeos" => resolve_nucleus_socket(env),
        _ => resolve_biomeos_dir(env).join(format!("{}.sock", service_name.to_lowercase())),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Public API (thin wrappers - single env snapshot at call site)
// ═══════════════════════════════════════════════════════════════════════════

/// Get runtime directory for socket files
///
/// Priority:
/// 1. XDG_RUNTIME_DIR environment variable
/// 2. /run/user/`uid` (Linux standard)
/// 3. /tmp with username fallback (dev/testing only)
///
/// **TRUE PRIMAL**: Environment-based, no hardcoding
///
/// **EVOLVED**: Pure Rust UID detection (no unsafe, no libc!)
pub fn get_runtime_dir() -> String {
    resolve_runtime_dir(&SocketPathEnv::from_env())
}

/// Get biomeos directory path (standard subdirectory for all primal sockets)
///
/// biomeOS socket standard: All sockets in `biomeos/` subdirectory
/// - Enables organized discovery
/// - Security: Proper permissions (0700)
/// - Integration: Predictable paths for all primals
pub fn get_biomeos_dir() -> PathBuf {
    resolve_biomeos_dir(&SocketPathEnv::from_env())
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
    resolve_family_id(&SocketPathEnv::from_env())
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
    // Spawn a separate thread to avoid nested runtime panics
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discover_crypto_socket()).ok()
    })
    .join()
    .ok()
    .flatten();

    if let Some(path) = discovery_result {
        return path;
    }

    resolve_beardog_socket_fallback(&SocketPathEnv::from_env())
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
    // Spawn a separate thread to avoid nested runtime panics
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discover_coordination_socket()).ok()
    })
    .join()
    .ok()
    .flatten();

    if let Some(path) = discovery_result {
        return path;
    }

    resolve_songbird_socket_fallback(&SocketPathEnv::from_env())
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
    // Spawn a separate thread to avoid nested runtime panics
    let discovery_result = std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(discover_storage_socket()).ok()
    })
    .join()
    .ok()
    .flatten();

    if let Some(path) = discovery_result {
        return path;
    }

    resolve_nestgate_socket_fallback(&SocketPathEnv::from_env())
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
    resolve_squirrel_socket(&SocketPathEnv::from_env())
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
    resolve_nucleus_socket(&SocketPathEnv::from_env())
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
    resolve_toadstool_socket(&SocketPathEnv::from_env())
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
    match service_name.to_lowercase().as_str() {
        s if s == primals::BEARDOG || s == "bear-dog" => get_beardog_socket_path(),
        s if s == primals::SONGBIRD || s == "song-bird" => get_songbird_socket_path(),
        s if s == primals::NESTGATE || s == "nest-gate" => get_nestgate_socket_path(),
        s if s == primals::SQUIRREL => get_squirrel_socket_path(),
        s if s == primals::TOADSTOOL || s == "toad-stool" => get_toadstool_socket_path(),
        "nucleus" | "biomeos" => get_nucleus_socket_path(),

        // Generic pattern for unknown services (TRUE PRIMAL - works with ANY service!)
        _ => {
            let env = SocketPathEnv::from_env();
            let env_var = format!("{}_SOCKET", service_name.to_uppercase().replace('-', "_"));
            let override_path = std::env::var(&env_var).ok().map(PathBuf::from);
            resolve_socket_path_for_service(service_name, &env, override_path)
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
#[allow(deprecated)] // primals::* constants used for transition-period capability mapping
fn capability_to_biomeos_fallback(
    capability: &crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::Capability;

    // Map capabilities to service names for biomeOS standard paths.
    // Fallback only; remove once all primals register with discovery (Phase 3/4)
    let service_name = match capability {
        Capability::Crypto(_) => primals::BEARDOG, // FALLBACK_CRYPTO_SOCKET stem
        Capability::Storage(_) => primals::NESTGATE, // FALLBACK_STORAGE_SOCKET stem
        Capability::Coordination(_) => primals::SONGBIRD, // FALLBACK_COORDINATION_SOCKET stem
        Capability::Compute(_) => primals::TOADSTOOL, // SELF_SOCKET - self-knowledge OK
        _ => {
            return Err(SocketDiscoveryError::NoSocketFound(format!(
                "No fallback path for capability: {capability:?}"
            )))
        }
    };

    let env = SocketPathEnv::from_env();
    Ok(resolve_socket_path_for_service(service_name, &env, None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_env() -> SocketPathEnv {
        SocketPathEnv {
            xdg_runtime_dir: Some("/run/user/1000".to_string()),
            user: Some("testuser".to_string()),
            biomeos_family_id: None,
            beardog_socket: None,
            songbird_socket: None,
            nestgate_socket: None,
            squirrel_socket: None,
            toadstool_socket: None,
            biomeos_socket_path: None,
            nucleus_socket: None,
        }
    }

    // ───── SocketPathEnv ─────────────────────────────────────────────────

    #[test]
    fn socket_path_env_default_is_all_none() {
        let env = SocketPathEnv::default();
        assert!(env.xdg_runtime_dir.is_none());
        assert!(env.user.is_none());
        assert!(env.biomeos_family_id.is_none());
        assert!(env.beardog_socket.is_none());
        assert!(env.songbird_socket.is_none());
        assert!(env.toadstool_socket.is_none());
    }

    #[test]
    fn socket_path_env_with_runtime_dir() {
        let env = SocketPathEnv::with_runtime_dir("/tmp/test-runtime");
        assert_eq!(env.xdg_runtime_dir, Some("/tmp/test-runtime".to_string()));
        assert!(env.user.is_none());
    }

    #[test]
    fn socket_path_env_from_env_captures_vars() {
        // Just verify it doesn't panic; actual values depend on environment
        let env = SocketPathEnv::from_env();
        let _ = format!("{:?}", env);
    }

    // ───── resolve_runtime_dir ───────────────────────────────────────────

    #[test]
    fn resolve_runtime_dir_uses_xdg_when_set() {
        let env = test_env();
        assert_eq!(resolve_runtime_dir(&env), "/run/user/1000");
    }

    #[test]
    fn resolve_runtime_dir_falls_back_to_tmp_with_username() {
        let env = SocketPathEnv {
            xdg_runtime_dir: None,
            user: Some("alice".to_string()),
            ..Default::default()
        };
        let dir = resolve_runtime_dir(&env);
        // Either /run/user/<uid> if it exists, or /tmp/toadstool-runtime-alice
        assert!(dir.contains("alice") || dir.starts_with("/run/user/"));
    }

    #[test]
    fn resolve_runtime_dir_falls_back_to_default_user() {
        let env = SocketPathEnv::default();
        let dir = resolve_runtime_dir(&env);
        // Should produce some path, not panic
        assert!(!dir.is_empty());
    }

    // ───── resolve_biomeos_dir ───────────────────────────────────────────

    #[test]
    fn resolve_biomeos_dir_appends_biomeos() {
        let env = test_env();
        let dir = resolve_biomeos_dir(&env);
        assert_eq!(dir, PathBuf::from("/run/user/1000/biomeos"));
    }

    // ───── resolve_family_id ─────────────────────────────────────────────

    #[test]
    fn resolve_family_id_uses_env_value() {
        let env = SocketPathEnv {
            biomeos_family_id: Some("my-family".to_string()),
            ..test_env()
        };
        assert_eq!(resolve_family_id(&env), "my-family");
    }

    #[test]
    fn resolve_family_id_defaults_to_default() {
        let env = test_env();
        assert_eq!(resolve_family_id(&env), "default");
    }

    // ───── resolve_beardog_socket_fallback ────────────────────────────────

    #[test]
    fn resolve_beardog_socket_uses_env_override() {
        let env = SocketPathEnv {
            beardog_socket: Some("/custom/beardog.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_beardog_socket_fallback(&env),
            PathBuf::from("/custom/beardog.sock")
        );
    }

    #[test]
    fn resolve_beardog_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_beardog_socket_fallback(&env),
            PathBuf::from("/run/user/1000/biomeos/beardog.sock")
        );
    }

    // ───── resolve_songbird_socket_fallback ───────────────────────────────

    #[test]
    fn resolve_songbird_socket_uses_env_override() {
        let env = SocketPathEnv {
            songbird_socket: Some("/custom/songbird.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_songbird_socket_fallback(&env),
            PathBuf::from("/custom/songbird.sock")
        );
    }

    #[test]
    fn resolve_songbird_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_songbird_socket_fallback(&env),
            PathBuf::from("/run/user/1000/biomeos/songbird.sock")
        );
    }

    // ───── resolve_nestgate_socket_fallback ───────────────────────────────

    #[test]
    fn resolve_nestgate_socket_uses_env_override() {
        let env = SocketPathEnv {
            nestgate_socket: Some("/custom/nestgate.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_nestgate_socket_fallback(&env),
            PathBuf::from("/custom/nestgate.sock")
        );
    }

    #[test]
    fn resolve_nestgate_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_nestgate_socket_fallback(&env),
            PathBuf::from("/run/user/1000/biomeos/nestgate.sock")
        );
    }

    // ───── resolve_squirrel_socket ───────────────────────────────────────

    #[test]
    fn resolve_squirrel_socket_uses_env_override() {
        let env = SocketPathEnv {
            squirrel_socket: Some("/custom/squirrel.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_squirrel_socket(&env),
            PathBuf::from("/custom/squirrel.sock")
        );
    }

    #[test]
    fn resolve_squirrel_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_squirrel_socket(&env),
            PathBuf::from("/run/user/1000/biomeos/squirrel.sock")
        );
    }

    // ───── resolve_nucleus_socket ────────────────────────────────────────

    #[test]
    fn resolve_nucleus_socket_uses_env_override() {
        let env = SocketPathEnv {
            nucleus_socket: Some("/custom/nucleus.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_nucleus_socket(&env),
            PathBuf::from("/custom/nucleus.sock")
        );
    }

    #[test]
    fn resolve_nucleus_socket_uses_biomeos_socket_path() {
        let env = SocketPathEnv {
            biomeos_socket_path: Some("/var/run/biomeos.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_nucleus_socket(&env),
            PathBuf::from("/var/run/biomeos.sock")
        );
    }

    #[test]
    fn resolve_nucleus_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_nucleus_socket(&env),
            PathBuf::from("/run/user/1000/biomeos/nucleus.sock")
        );
    }

    // ───── resolve_toadstool_socket ──────────────────────────────────────

    #[test]
    fn resolve_toadstool_socket_uses_env_override() {
        let env = SocketPathEnv {
            toadstool_socket: Some("/custom/toadstool.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_toadstool_socket(&env),
            PathBuf::from("/custom/toadstool.sock")
        );
    }

    #[test]
    fn resolve_toadstool_socket_uses_biomeos_socket_path() {
        let env = SocketPathEnv {
            biomeos_socket_path: Some("/var/run/toadstool.sock".to_string()),
            ..test_env()
        };
        assert_eq!(
            resolve_toadstool_socket(&env),
            PathBuf::from("/var/run/toadstool.sock")
        );
    }

    #[test]
    fn resolve_toadstool_socket_uses_biomeos_fallback() {
        let env = test_env();
        assert_eq!(
            resolve_toadstool_socket(&env),
            PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
        );
    }

    // ───── resolve_socket_path_for_service ───────────────────────────────

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_override_takes_precedence() {
        let env = test_env();
        let override_path = PathBuf::from("/override/custom.sock");
        let result = resolve_socket_path_for_service("beardog", &env, Some(override_path.clone()));
        assert_eq!(result, override_path);
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_beardog() {
        let env = test_env();
        let result = resolve_socket_path_for_service("beardog", &env, None);
        assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_beardog_alias() {
        let env = test_env();
        let result = resolve_socket_path_for_service("bear-dog", &env, None);
        assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_songbird() {
        let env = test_env();
        let result = resolve_socket_path_for_service("songbird", &env, None);
        assert_eq!(
            result,
            PathBuf::from("/run/user/1000/biomeos/songbird.sock")
        );
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_toadstool() {
        let env = test_env();
        let result = resolve_socket_path_for_service("toadstool", &env, None);
        assert_eq!(
            result,
            PathBuf::from("/run/user/1000/biomeos/toadstool.sock")
        );
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_nucleus() {
        let env = test_env();
        let result = resolve_socket_path_for_service("nucleus", &env, None);
        assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/nucleus.sock"));
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_unknown_falls_through() {
        let env = test_env();
        let result = resolve_socket_path_for_service("myservice", &env, None);
        assert_eq!(
            result,
            PathBuf::from("/run/user/1000/biomeos/myservice.sock")
        );
    }

    #[test]
    #[allow(deprecated)]
    fn resolve_service_socket_case_insensitive() {
        let env = test_env();
        let result = resolve_socket_path_for_service("BearDog", &env, None);
        assert_eq!(result, PathBuf::from("/run/user/1000/biomeos/beardog.sock"));
    }

    // ───── Public API wrappers ───────────────────────────────────────────

    #[test]
    fn get_runtime_dir_returns_valid_path() {
        let dir = get_runtime_dir();
        assert!(!dir.is_empty());
    }

    #[test]
    fn get_biomeos_dir_contains_biomeos() {
        let dir = get_biomeos_dir();
        assert!(dir.to_string_lossy().contains("biomeos"));
    }

    #[test]
    fn get_family_id_returns_string() {
        let id = get_family_id();
        assert!(!id.is_empty());
    }

    #[test]
    #[allow(deprecated)]
    fn get_beardog_socket_path_is_path() {
        let path = get_beardog_socket_path();
        assert!(path.to_string_lossy().contains("beardog"));
    }

    #[test]
    #[allow(deprecated)]
    fn get_songbird_socket_path_is_path() {
        let path = get_songbird_socket_path();
        assert!(path.to_string_lossy().contains("songbird"));
    }

    #[test]
    #[allow(deprecated)]
    fn get_nestgate_socket_path_is_path() {
        let path = get_nestgate_socket_path();
        assert!(path.to_string_lossy().contains("nestgate"));
    }

    #[test]
    fn get_squirrel_socket_path_is_path() {
        let path = get_squirrel_socket_path();
        assert!(path.to_string_lossy().contains("squirrel"));
    }

    #[test]
    fn get_toadstool_socket_path_is_path() {
        let path = get_toadstool_socket_path();
        assert!(path.to_string_lossy().contains("toadstool"));
    }

    #[test]
    fn get_nucleus_socket_path_is_path() {
        let path = get_nucleus_socket_path();
        let s = path.to_string_lossy();
        assert!(s.contains("nucleus") || s.contains("biomeos"));
    }
}
