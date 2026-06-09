// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Service mesh endpoint source (delegates network discovery to the coordination service).

use std::future::Future;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

/// Service mesh source - discovers services via the coordination service
///
/// Service mesh discovery is delegated to the coordination service.
/// ToadStool only reports mDNS capability requirements upstream.
pub struct ServiceMeshSource;

impl ServiceMeshSource {
    /// Create new service mesh source
    ///
    /// Discovery is delegated to the coordination service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ServiceMeshSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for ServiceMeshSource {
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();

        Box::pin(async move {
            tracing::trace!(
                service,
                "Service mesh discovery delegated to coordination service"
            );
            Ok(None)
        })
    }

    fn source_name(&self) -> &'static str {
        "service_mesh"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_mesh_source_new() {
        let source = ServiceMeshSource::new();
        assert_eq!(source.source_name(), "service_mesh");
    }

    #[test]
    fn test_service_mesh_source_default() {
        let _source = ServiceMeshSource;
    }

    #[tokio::test]
    async fn test_service_mesh_auto_delegates_to_coordination() {
        let source = ServiceMeshSource::new();
        let result = source.resolve("auto-service").await.unwrap();

        assert_eq!(result, None);
    }
}
