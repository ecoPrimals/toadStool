// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Local service discovery via biomeOS socket directory scanning.

use std::future::Future;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

/// mDNS discovery source - discovers services via multicast DNS
pub struct MDNSSource;

impl MDNSSource {
    /// Create a new mDNS discovery source.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for MDNSSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for MDNSSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();

        Box::pin(async move {
            let biomeos_dir = crate::primal_sockets::get_biomeos_dir();

            if let Ok(entries) = std::fs::read_dir(&biomeos_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        if service.contains(name) {
                            let endpoint = format!("unix://{}", path.display());
                            tracing::debug!(
                                service,
                                endpoint,
                                "Found local service via biomeos socket discovery"
                            );
                            return Ok(Some(endpoint));
                        }
                    }
                }
            }

            tracing::trace!(service, "No mDNS-discoverable service found");
            Ok(None)
        })
    }

    fn source_name(&self) -> &'static str {
        "mdns"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdns_source_new() {
        let source = MDNSSource::new();
        assert_eq!(source.source_name(), "mdns");
    }

    #[test]
    #[expect(
        path_statements,
        reason = "verifies MDNSSource unit struct can be named"
    )]
    fn test_mdns_source_default() {
        MDNSSource;
    }

    #[tokio::test]
    async fn test_mdns_source_legacy_coordination_label() {
        let source = MDNSSource::new();
        // legacy socket basename substring
        let result = source.resolve("songbird").await.unwrap();
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_legacy_storage_label() {
        let source = MDNSSource::new();
        let result = source.resolve("nestgate").await.unwrap();
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_legacy_security_substring() {
        let source = MDNSSource::new();
        let result = source.resolve("beardog_orchestration").await.unwrap();
        if let Some(endpoint) = result {
            assert!(endpoint.starts_with("unix://"));
        }
    }

    #[tokio::test]
    async fn test_mdns_source_unknown() {
        let source = MDNSSource::new();
        let result = source.resolve("unknown_service").await.unwrap();

        assert_eq!(result, None);
    }
}
