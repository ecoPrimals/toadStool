//! Capability-based socket discovery

use std::path::PathBuf;

#[allow(deprecated)]
use crate::interned_strings::primals;

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

/// Helper: Map capability to biomeOS standard path (fallback during transition)
#[allow(deprecated)]
fn capability_to_biomeos_fallback(
    capability: &crate::primal_identity::Capability,
) -> Result<PathBuf, SocketDiscoveryError> {
    use crate::primal_identity::Capability;

    let service_name = match capability {
        Capability::Crypto(_) => primals::BEARDOG,
        Capability::Storage(_) => primals::NESTGATE,
        Capability::Coordination(_) => primals::SONGBIRD,
        Capability::Compute(_) => primals::TOADSTOOL,
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
