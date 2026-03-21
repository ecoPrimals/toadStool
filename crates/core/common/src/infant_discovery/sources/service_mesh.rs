// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

//! Service mesh endpoint source (delegates network discovery to Songbird).

use std::future::Future;
use std::pin::Pin;

use crate::infant_discovery::capabilities::{DiscoveryError, EndpointSource};

/// Service mesh source - discovers services via Songbird (comms primal)
///
/// ## Evolution (Feb 15, 2026)
///
/// Service mesh discovery is now delegated to Songbird (comms primal).
/// Vendor-specific options (Consul, etcd, Kubernetes) removed - they are
/// Songbird's concern, not ToadStool's.
///
/// ToadStool only reports mDNS capability requirements to Songbird.
pub struct ServiceMeshSource {
    mesh_type: ServiceMeshType,
}

/// Service mesh type
///
/// ## Evolution (Feb 15, 2026)
///
/// Vendor-specific types (Consul, etcd, Kubernetes) deprecated.
/// Service discovery is Songbird's responsibility.
#[derive(Debug, Clone, Copy)]
pub enum ServiceMeshType {
    /// Auto-detect (delegates to Songbird)
    Auto,
    /// Consul service mesh (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Consul,
    /// etcd key-value store (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Etcd,
    /// Kubernetes service discovery (deprecated - use Songbird)
    #[deprecated(since = "0.16.0", note = "Use Songbird for service mesh discovery")]
    Kubernetes,
}

impl ServiceMeshSource {
    /// Create new service mesh source with auto-detection
    ///
    /// Discovery is delegated to Songbird (comms primal).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mesh_type: ServiceMeshType::Auto,
        }
    }

    /// Create with specific mesh type
    ///
    /// Note: Vendor-specific types are deprecated. Use Auto to delegate to Songbird.
    #[must_use]
    pub const fn with_type(mesh_type: ServiceMeshType) -> Self {
        Self { mesh_type }
    }
}

impl Default for ServiceMeshSource {
    fn default() -> Self {
        Self::new()
    }
}

impl EndpointSource for ServiceMeshSource {
    #[allow(deprecated)]
    fn resolve(
        &self,
        service: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>> {
        let service = service.to_string();
        let mesh_type = self.mesh_type;

        Box::pin(async move {
            match mesh_type {
                ServiceMeshType::Auto => {
                    tracing::trace!(service, "Service mesh discovery delegated to Songbird");
                    Ok(None)
                }
                ServiceMeshType::Consul | ServiceMeshType::Etcd | ServiceMeshType::Kubernetes => {
                    tracing::warn!(
                        service,
                        "Vendor-specific service mesh deprecated - use Songbird"
                    );
                    Ok(None)
                }
            }
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
        let source = ServiceMeshSource::default();
        assert!(matches!(source.mesh_type, ServiceMeshType::Auto));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_consul_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Consul);
        assert!(matches!(source.mesh_type, ServiceMeshType::Consul));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_etcd_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Etcd);
        assert!(matches!(source.mesh_type, ServiceMeshType::Etcd));
    }

    #[test]
    #[allow(deprecated)]
    fn test_service_mesh_source_with_kubernetes_deprecated() {
        let source = ServiceMeshSource::with_type(ServiceMeshType::Kubernetes);
        assert!(matches!(source.mesh_type, ServiceMeshType::Kubernetes));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_service_mesh_deprecated_returns_none() {
        let consul = ServiceMeshSource::with_type(ServiceMeshType::Consul);
        let etcd = ServiceMeshSource::with_type(ServiceMeshType::Etcd);
        let k8s = ServiceMeshSource::with_type(ServiceMeshType::Kubernetes);

        assert_eq!(consul.resolve("test-service").await.unwrap(), None);
        assert_eq!(etcd.resolve("test-service").await.unwrap(), None);
        assert_eq!(k8s.resolve("test-service").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_service_mesh_auto_delegates_to_songbird() {
        let source = ServiceMeshSource::new();
        let result = source.resolve("auto-service").await.unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn test_service_mesh_type_debug() {
        let mesh_type = ServiceMeshType::Auto;
        let debug = format!("{mesh_type:?}");
        assert!(debug.contains("Auto"));
    }
}
