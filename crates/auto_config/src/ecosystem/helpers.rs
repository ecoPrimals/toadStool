// SPDX-License-Identifier: AGPL-3.0-or-later
//! Merge discovery sources and resolve capability endpoints from environment variables.

use std::collections::HashMap;

use crate::ecosystem_types::{DiscoveredServices, DiscoverySummary, ServiceInfo};

/// Merge parallel discovery sources into a single [`DiscoveredServices`] snapshot.
pub(crate) fn assemble_discovered_services(
    local_result: crate::ToadStoolResult<HashMap<String, ServiceInfo>>,
    network_result: crate::ToadStoolResult<HashMap<String, ServiceInfo>>,
    wellknown_result: crate::ToadStoolResult<HashMap<String, ServiceInfo>>,
    mdns_result: crate::ToadStoolResult<HashMap<String, ServiceInfo>>,
) -> DiscoveredServices {
    let mut discovered_services = HashMap::new();
    let mut discovery_summary = DiscoverySummary::default();

    if let Ok(local) = local_result {
        discovered_services.extend(local);
    }
    if let Ok(network) = network_result {
        discovered_services.extend(network);
    }
    if let Ok(wellknown) = wellknown_result {
        discovered_services.extend(wellknown);
    }
    if let Ok(mdns) = mdns_result {
        discovered_services.extend(mdns);
    }

    discovery_summary.total_services_found = discovered_services.len();
    discovery_summary.discovery_methods_used = vec![
        "local".to_string(),
        "network_scan".to_string(),
        "wellknown_ports".to_string(),
        "mdns".to_string(),
    ];

    DiscoveredServices {
        discovered_services,
        discovery_summary,
        discovery_timestamp: std::time::SystemTime::now(),
    }
}

/// Try env var for capability (capability-based, then legacy for backward compat)
pub(crate) fn get_capability_endpoint(
    capability_key: &str,
    legacy_keys: &[&str],
) -> Option<String> {
    let cap_var = format!("{}_ENDPOINT", capability_key.to_uppercase());
    if let Ok(endpoint) = std::env::var(&cap_var) {
        return Some(endpoint);
    }
    for legacy in legacy_keys {
        if let Ok(endpoint) = std::env::var(format!("{legacy}_ENDPOINT")) {
            return Some(endpoint);
        }
    }
    None
}
