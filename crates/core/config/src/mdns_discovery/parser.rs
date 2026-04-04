// SPDX-License-Identifier: AGPL-3.0-only
//! TXT record parsing for mDNS-SD–style capability strings.
//!
//! When full [`mdns-sd`](https://crates.io/crates/mdns-sd) integration lands, responses will
//! deliver TXT records as strings; this module centralizes the mapping from those strings to
//! [`Capability`](toadstool_common::primal_identity::Capability) values.
//!
//! # Record format
//!
//! Capability entries use the prefix `capability=` followed by a category and variant separated
//! by `:`, for example `capability=coordination:service-discovery`. Unknown combinations are
//! logged in tests and skipped.
//!
//! # Future work
//!
//! - Parse additional metadata (version, TLS hints) into [`DiscoveredService`] fields.
//! - Share parsing with any live network listener once browsing is implemented.

use std::collections::HashMap;

use tracing::warn;

use toadstool_common::primal_identity::{
    AuthCapability, Capability, ComputeCapability, CoordinationCapability, DiscoveredService,
    DiscoveryCapability, ServiceEndpoint, StorageCapability,
};

/// Parse mDNS TXT records to extract [`Capability`] values.
pub(crate) fn parse_capabilities(txt_records: &[String]) -> Vec<Capability> {
    let mut capabilities = Vec::new();

    for record in txt_records {
        if let Some(cap_str) = record.strip_prefix("capability=") {
            // Parse capability from string
            // Format: "capability=coordination:service-discovery", "capability=storage:object", etc.
            let parts: Vec<&str> = cap_str.split(':').collect();
            match parts.as_slice() {
                ["coordination", "service-discovery" | _] => capabilities.push(
                    Capability::Coordination(CoordinationCapability::ServiceDiscovery),
                ),
                ["storage", "object" | _] => {
                    capabilities.push(Capability::Storage(StorageCapability::ObjectStorage));
                }
                ["compute", "native" | _] => {
                    capabilities.push(Capability::Compute(ComputeCapability::NativeExecution));
                }
                ["authentication", _] => {
                    capabilities.push(Capability::Authentication(AuthCapability::UserAuth));
                }
                ["discovery", "mdns"] => {
                    capabilities.push(Capability::Discovery(DiscoveryCapability::MdnsDiscovery));
                }
                ["discovery", _] => capabilities.push(Capability::Discovery(
                    DiscoveryCapability::CapabilityDiscovery,
                )),
                _ => warn!("Unknown capability in mDNS record: {}", cap_str),
            }
        }
    }

    capabilities
}

/// Convert mDNS-style fields into a [`DiscoveredService`] (tests and future browse path).
pub(crate) fn mdns_to_discovered_service(
    id: String,
    address: std::net::IpAddr,
    port: u16,
    txt_records: &[String],
) -> DiscoveredService {
    let capabilities = parse_capabilities(txt_records);

    let endpoint = ServiceEndpoint {
        address: address.to_string(),
        port,
        protocol: "http".to_string(), // Default to HTTP
        path: None,
        metadata: HashMap::new(),
    };

    DiscoveredService {
        id: Some(id),
        capabilities,
        endpoints: vec![endpoint],
        healthy: true,
        metadata: HashMap::new(),
    }
}
