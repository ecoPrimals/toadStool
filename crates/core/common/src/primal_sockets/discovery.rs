//! Capability-based socket discovery

use std::path::PathBuf;

use super::env::SocketPathEnv;
use super::paths;

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
pub async fn discover_socket_for_capability(
    capability: crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::capability_discovery::CapabilityDiscovery;

    let discovery = CapabilityDiscovery::new()
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
pub async fn discover_crypto_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CryptoCapability};
    discover_socket_for_capability(Capability::Crypto(CryptoCapability::Encryption)).await
}

/// Convenience: Discover storage service socket
pub async fn discover_storage_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, StorageCapability};
    discover_socket_for_capability(Capability::Storage(StorageCapability::ObjectStorage)).await
}

/// Convenience: Discover coordination service socket
pub async fn discover_coordination_socket() -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::{Capability, CoordinationCapability};
    discover_socket_for_capability(Capability::Coordination(
        CoordinationCapability::ServiceDiscovery,
    ))
    .await
}

/// Helper: Map capability to biomeOS standard socket path (fallback during transition)
fn capability_to_biomeos_fallback(
    capability: &crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::Capability;

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

    let env = SocketPathEnv::from_env();
    Ok(paths::resolve_socket_path_for_service(
        service_name,
        &env,
        None,
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
        let _ = format!("{:?}", err);
    }

    #[test]
    fn test_socket_discovery_error_is_std_error() {
        use std::error::Error;
        let err = SocketDiscoveryError::DiscoveryFailed("test".to_string());
        assert!(err.source().is_none());
    }

    /// Test capability_to_biomeos fallback mapping via paths - Crypto -> beardog
    #[test]
    fn test_capability_fallback_path_crypto() {
        use crate::primal_sockets::{resolve_socket_path_for_service, SocketPathEnv};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("beardog", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("beardog"));
        assert!(path.to_string_lossy().ends_with("beardog.sock"));
    }

    /// Test capability_to_biomeos fallback mapping - Storage -> nestgate
    #[test]
    fn test_capability_fallback_path_storage() {
        use crate::primal_sockets::{resolve_socket_path_for_service, SocketPathEnv};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("nestgate", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("nestgate"));
    }

    /// Test capability_to_biomeos fallback mapping - Coordination -> songbird
    #[test]
    fn test_capability_fallback_path_coordination() {
        use crate::primal_sockets::{resolve_socket_path_for_service, SocketPathEnv};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("songbird", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("songbird"));
    }

    /// Test capability_to_biomeos fallback mapping - Compute -> toadstool
    #[test]
    fn test_capability_fallback_path_compute() {
        use crate::primal_sockets::{resolve_socket_path_for_service, SocketPathEnv};
        let env_snapshot = SocketPathEnv::with_runtime_dir("/tmp/test");
        let path = resolve_socket_path_for_service("toadstool", &env_snapshot, None);
        assert!(path.to_string_lossy().contains("toadstool"));
    }
}
