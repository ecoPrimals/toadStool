// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based socket discovery

use std::path::PathBuf;

use super::env::SocketPathEnv;
use super::paths;

/// Error type for socket discovery operations
#[derive(Debug, thiserror::Error)]
pub enum SocketDiscoveryError {
    /// Discovery backend failed to initialize or query
    #[error("Discovery service failed: {0}")]
    DiscoveryFailed(String),

    /// No Unix socket found for the requested capability
    #[error("No Unix socket found for capability: {0:?}")]
    NoSocketFound(String),

    /// Endpoint format was invalid
    #[error("Invalid socket endpoint: {0}")]
    InvalidEndpoint(String),
}

/// Discover Unix socket path for a service providing the requested capability
///
/// # Errors
///
/// Returns [`SocketDiscoveryError`] if discovery fails or no Unix socket is found for the capability.
pub async fn discover_socket_for_capability(
    capability: crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::capability_discovery::CapabilityDiscovery;

    let discovery = CapabilityDiscovery::new_async()
        .await
        .map_err(|e| SocketDiscoveryError::DiscoveryFailed(e.to_string()))?;

    let services = discovery
        .find_by_capability(capability.clone())
        .await
        .map_err(|e| SocketDiscoveryError::DiscoveryFailed(e.to_string()))?;

    for service in services {
        for endpoint in &service.endpoints {
            if endpoint.protocol == "unix" {
                if let Some(path_str) = endpoint.metadata.get("path") {
                    return Ok(PathBuf::from(path_str));
                }
                if endpoint.address.starts_with('/') {
                    return Ok(PathBuf::from(&endpoint.address));
                }
            }
        }
    }

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
/// # Errors
///
/// Returns [`SocketDiscoveryError`] if discovery fails or no socket is found.
pub async fn discover_crypto_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CryptoCapability};
    discover_socket_for_capability(Capability::Crypto(CryptoCapability::Encryption)).await
}

/// Convenience: Discover storage service socket
///
/// # Errors
///
/// Returns [`SocketDiscoveryError`] if discovery fails or no socket is found.
pub async fn discover_storage_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, StorageCapability};
    discover_socket_for_capability(Capability::Storage(StorageCapability::ObjectStorage)).await
}

/// Convenience: Discover coordination service socket
///
/// # Errors
///
/// Returns [`SocketDiscoveryError`] if discovery fails or no socket is found.
pub async fn discover_coordination_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CoordinationCapability};
    discover_socket_for_capability(Capability::Coordination(
        CoordinationCapability::ServiceDiscovery,
    ))
    .await
}

/// Map a capability to a biomeOS-standard socket path via environment-based fallback.
///
/// **Sovereignty**: No hardcoded primal names. The mapping is driven by environment
/// variables (`BIOMEOS_CRYPTO_SOCKET`, `BIOMEOS_STORAGE_SOCKET`, etc.) and falls back
/// to the well-known capability category names (not primal identities).
pub fn capability_to_biomeos_fallback(
    capability: &crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::Capability;

    let category_name = match capability {
        Capability::Crypto(_) => "crypto",
        Capability::Storage(_) => "storage",
        Capability::Coordination(_) => "coordination",
        Capability::Compute(_) => "compute",
        _ => {
            return Err(SocketDiscoveryError::NoSocketFound(format!(
                "No fallback path for capability: {capability:?}"
            )));
        }
    };

    let env = SocketPathEnv::from_env();
    Ok(paths::resolve_capability_socket_fallback(
        category_name,
        &env,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_discovery_error_discovery_failed() {
        let err = SocketDiscoveryError::DiscoveryFailed("backend init failed".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Discovery"));
        assert!(msg.contains("backend init failed"));
    }

    #[test]
    fn test_socket_discovery_error_no_socket_found() {
        let err = SocketDiscoveryError::NoSocketFound("Capability::Crypto(Encryption)".to_string());
        let msg = err.to_string();
        assert!(msg.contains("socket") || msg.contains("Crypto"));
    }

    #[test]
    fn test_socket_discovery_error_invalid_endpoint() {
        let err = SocketDiscoveryError::InvalidEndpoint("bad path".to_string());
        let msg = err.to_string();
        assert!(msg.contains("Invalid"));
        assert!(msg.contains("bad path"));
    }

    #[test]
    fn test_socket_discovery_error_debug() {
        let err = SocketDiscoveryError::NoSocketFound("test".to_string());
        let _ = format!("{err:?}");
    }

    #[test]
    fn test_socket_discovery_error_is_std_error() {
        use std::error::Error;
        let err = SocketDiscoveryError::DiscoveryFailed("test".to_string());
        assert!(err.source().is_none());
    }

    /// Test capability fallback uses capability category names (not primal names)
    #[test]
    fn test_capability_fallback_path_crypto() {
        use crate::primal_sockets::{SocketPathEnv, resolve_socket_path_for_service};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("crypto", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("crypto"));
        assert!(path.to_string_lossy().ends_with("crypto.sock"));
    }

    /// Test capability fallback - Storage category
    #[test]
    fn test_capability_fallback_path_storage() {
        use crate::primal_sockets::{SocketPathEnv, resolve_socket_path_for_service};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("storage", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("storage"));
    }

    /// Test capability fallback - Coordination category
    #[test]
    fn test_capability_fallback_path_coordination() {
        use crate::primal_sockets::{SocketPathEnv, resolve_socket_path_for_service};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("coordination", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("coordination"));
    }

    /// Test capability fallback - Compute category
    #[test]
    fn test_capability_fallback_path_compute() {
        use crate::primal_sockets::{SocketPathEnv, resolve_socket_path_for_service};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("compute", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("compute"));
    }

    #[test]
    fn test_capability_to_biomeos_fallback_crypto() {
        use crate::primal_identity::{Capability, CryptoCapability};
        let path =
            capability_to_biomeos_fallback(&Capability::Crypto(CryptoCapability::Encryption));
        assert!(path.is_ok());
        let path = path.expect("path");
        assert!(path.to_string_lossy().contains("crypto"));
    }

    #[test]
    fn test_capability_to_biomeos_fallback_storage() {
        use crate::primal_identity::{Capability, StorageCapability};
        let path =
            capability_to_biomeos_fallback(&Capability::Storage(StorageCapability::ObjectStorage));
        assert!(path.is_ok());
        let path = path.expect("path");
        assert!(path.to_string_lossy().contains("storage"));
    }

    #[test]
    fn test_capability_to_biomeos_fallback_unknown_returns_error() {
        use crate::primal_identity::{Capability, DiscoveryCapability};
        let result = capability_to_biomeos_fallback(&Capability::Discovery(
            DiscoveryCapability::MdnsDiscovery,
        ));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No fallback") || err.to_string().contains("Discovery"));
    }

    #[test]
    fn test_capability_to_biomeos_fallback_compute() {
        use crate::primal_identity::{Capability, ComputeCapability};
        let path = capability_to_biomeos_fallback(&Capability::Compute(
            ComputeCapability::NativeExecution,
        ));
        assert!(path.is_ok());
        let path = path.expect("path");
        assert!(path.to_string_lossy().contains("compute"));
    }

    #[test]
    fn test_capability_to_biomeos_fallback_coordination() {
        use crate::primal_identity::{Capability, CoordinationCapability};
        let path = capability_to_biomeos_fallback(&Capability::Coordination(
            CoordinationCapability::ServiceDiscovery,
        ));
        assert!(path.is_ok());
        let path = path.expect("path");
        assert!(path.to_string_lossy().contains("coordination"));
    }
}
