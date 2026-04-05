// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability discovery via coordination service.
//!
//! Queries the discovery service to find providers for a given capability.

use crate::primal_identity::Capability;
use crate::unix_jsonrpc_client::UnixJsonRpcClient;
use std::path::PathBuf;

use super::error::{CapabilityError, Result};
use super::provider::CapabilityProvider;
use super::serialize;

/// Query discovery service for all providers of a capability
pub async fn query_providers(capability: Capability) -> Result<Vec<CapabilityProvider>> {
    let discovery_socket = std::env::var("BIOMEOS_COORDINATION_SOCKET")
        .or_else(|_| std::env::var("COORDINATION_SOCKET"))
        .or_else(|_| std::env::var("SONGBIRD_SOCKET")) // legacy
        .unwrap_or_else(|_| {
            let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
                .unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
            format!("{runtime_dir}/biomeos/coordination.sock")
        });

    let client = UnixJsonRpcClient::new(&discovery_socket);

    let params = serde_json::json!({
        "capability": serialize::capability_to_string(&capability)
    });

    let response = client
        .call("ipc.find_capability", params)
        .await
        .map_err(|_| CapabilityError::DiscoveryUnavailable)?;

    let services = response["services"]
        .as_array()
        .ok_or_else(|| CapabilityError::InvalidResponse("No services array".into()))?;

    let mut providers = Vec::new();

    for service in services {
        let service_name = service["name"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No name field".into()))?
            .to_string();

        let endpoint = service["endpoint"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No endpoint field".into()))?;

        let socket_path = PathBuf::from(endpoint);

        let capabilities = service["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(serialize::string_to_capability)
                    .collect()
            })
            .unwrap_or_default();

        providers.push(CapabilityProvider::from_service_info(
            service_name,
            socket_path,
            capabilities,
        ));
    }

    Ok(providers)
}

/// Discover multiple providers for a capability
///
/// Useful for load balancing or failover scenarios
///
/// # Errors
///
/// Returns [`CapabilityError`] if:
/// - Discovery service is unavailable (socket unreachable)
/// - Response is invalid (missing services array, name, or endpoint fields)
pub async fn discover_all(capability: Capability) -> Result<Vec<CapabilityProvider>> {
    query_providers(capability).await
}
