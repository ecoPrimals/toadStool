// SPDX-License-Identifier: AGPL-3.0-or-later
//! Registry discovery backend (HTTP + file/unix delegation)
//!
//! Registry protocol: GET {endpoint}/services → JSON array of `ConfigFileService`.
//! Resolution order: arg → `TOADSTOOL_REGISTRY_ENDPOINT` env → error.

use std::time::SystemTime;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use crate::primal_identity::ServiceEndpoint;

use super::config::{capability_from_str, ConfigFile};
use super::discovery_config::discover_from_config;
use super::types::{DiscoveredService, DiscoveryError, DiscoveryResult};

/// Discover services from a registry endpoint.
pub async fn discover_from_registry(endpoint: &str) -> DiscoveryResult<Vec<DiscoveredService>> {
    let resolved = if !endpoint.is_empty() {
        endpoint.to_string()
    } else if let Ok(env_ep) = std::env::var("TOADSTOOL_REGISTRY_ENDPOINT") {
        env_ep
    } else {
        return Err(DiscoveryError::MethodUnavailable {
            method: "registry endpoint not configured (set TOADSTOOL_REGISTRY_ENDPOINT)"
                .to_string(),
        });
    };

    // For Unix socket registries (file:// or unix://) delegate to config discovery
    if resolved.starts_with("file://") || resolved.starts_with("unix://") {
        let path = resolved
            .trim_start_matches("file://")
            .trim_start_matches("unix://");
        return discover_from_config(path).await;
    }

    // HTTP registry: use tokio TCP to issue a minimal HTTP/1.1 GET request
    let url = resolved
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let (host_port, path) = url.split_once('/').unwrap_or((url, "services"));
    let path = format!("/{path}");

    let mut stream = TcpStream::connect(host_port)
        .await
        .map_err(|source| DiscoveryError::NetworkError { source })?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|source| DiscoveryError::NetworkError { source })?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|source| DiscoveryError::NetworkError { source })?;

    // Strip HTTP headers — body starts after first blank line
    let blank = b"\r\n\r\n";
    let body = response
        .as_slice()
        .windows(blank.len())
        .position(|w| w == blank)
        .map_or(response.as_slice(), |pos| &response[pos + blank.len()..]);

    let config_file: ConfigFile =
        serde_json::from_slice(body).map_err(|e| DiscoveryError::InvalidResponse {
            reason: format!("malformed registry response from {resolved:?}: {e}"),
        })?;

    let now = SystemTime::now();
    let services = config_file
        .services
        .into_iter()
        .map(|svc| {
            let caps: Vec<_> = svc
                .capabilities
                .iter()
                .map(|s| capability_from_str(s))
                .collect();
            let endpoints: Vec<ServiceEndpoint> = svc
                .endpoints
                .iter()
                .filter_map(|url| ServiceEndpoint::from_url_string(url).ok())
                .collect();
            let id = svc
                .id
                .unwrap_or_else(|| format!("registry-{}", svc.name.to_lowercase()));
            DiscoveredService {
                id,
                name: svc.name,
                version: svc.version,
                capabilities: caps,
                endpoints,
                metadata: svc.metadata,
                discovered_at: now,
                last_seen: now,
                healthy: true,
            }
        })
        .collect::<Vec<_>>();

    info!(
        "Registry discovery: loaded {} services from {:?}",
        services.len(),
        resolved
    );
    Ok(services)
}
