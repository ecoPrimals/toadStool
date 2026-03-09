// SPDX-License-Identifier: AGPL-3.0-only
//! Config file discovery backend

use std::time::SystemTime;

use tracing::{info, warn};

use crate::primal_identity::ServiceEndpoint;

use super::config::{capability_from_str, ConfigFile};
use super::types::{DiscoveredService, DiscoveryError, DiscoveryResult};

/// Resolve config path: explicit arg → env var → default locations.
#[must_use]
pub fn resolve_config_path(path: &str) -> String {
    if !path.is_empty() {
        path.to_string()
    } else if let Ok(p) = std::env::var("TOADSTOOL_DISCOVERY_CONFIG") {
        p
    } else if let Ok(runtime) = std::env::var("BIOMEOS_RUNTIME_DIR") {
        format!("{runtime}/discovery.json")
    } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        format!("{xdg}/biomeos/discovery.json")
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{home}/.config/biomeos/discovery.json")
    } else {
        "/etc/biomeos/discovery.json".to_string()
    }
}

/// Discover services from a config file at the given path.
pub async fn discover_from_config(path: &str) -> DiscoveryResult<Vec<DiscoveredService>> {
    let resolved_path = resolve_config_path(path);

    let content =
        tokio::fs::read(&resolved_path)
            .await
            .map_err(|e| DiscoveryError::MethodUnavailable {
                method: format!("cannot read discovery config {resolved_path:?}: {e}"),
            })?;

    let config_file: ConfigFile =
        serde_json::from_slice(&content).map_err(|e| DiscoveryError::InvalidResponse {
            reason: format!("malformed discovery config {resolved_path:?}: {e}"),
        })?;

    let now = SystemTime::now();
    let mut services = Vec::with_capacity(config_file.services.len());

    for svc in config_file.services {
        let caps: Vec<_> = svc
            .capabilities
            .iter()
            .map(|s| capability_from_str(s))
            .collect();

        let mut endpoints = Vec::with_capacity(svc.endpoints.len());
        for url in &svc.endpoints {
            match ServiceEndpoint::from_url_string(url) {
                Ok(ep) => endpoints.push(ep),
                Err(e) => {
                    warn!("Skipping malformed endpoint {url:?} in discovery config: {e}");
                }
            }
        }

        let id = svc
            .id
            .unwrap_or_else(|| format!("config-{}", svc.name.to_lowercase()));
        services.push(DiscoveredService {
            id,
            name: svc.name,
            version: svc.version,
            capabilities: caps,
            endpoints,
            metadata: svc.metadata,
            discovered_at: now,
            last_seen: now,
            healthy: true,
        });
    }

    info!(
        "Config discovery: loaded {} services from {:?}",
        services.len(),
        resolved_path
    );
    Ok(services)
}
