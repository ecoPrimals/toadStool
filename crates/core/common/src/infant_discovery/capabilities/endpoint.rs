// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use super::discovery_traits::DiscoveryError;

/// Endpoint resolution chain - tries multiple sources
pub struct EndpointResolver {
    pub(super) sources: Vec<Box<dyn EndpointSource>>,
}

impl EndpointResolver {
    /// Create new resolver with default sources
    #[must_use]
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Add an endpoint source to the resolution chain
    pub fn add_source(&mut self, source: Box<dyn EndpointSource>) {
        self.sources.push(source);
    }

    /// Resolve an endpoint by trying each source in order
    ///
    /// # Errors
    ///
    /// Returns `DiscoveryError::CapabilityNotFound` if no source can resolve the service.
    pub async fn resolve(&self, service: &str) -> Result<String, DiscoveryError> {
        for source in &self.sources {
            if let Some(endpoint) = source.resolve(service).await? {
                return Ok(endpoint);
            }
        }
        Err(DiscoveryError::CapabilityNotFound(service.to_string()))
    }
}

impl Default for EndpointResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Endpoint source trait - one way to find service endpoints
///
/// Migrated from `async_trait` to native async for zero-cost abstraction.
pub trait EndpointSource: Send + Sync {
    /// Try to resolve service endpoint from this source
    fn resolve(
        &self,
        service: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Option<String>, DiscoveryError>> + Send + '_>,
    >;

    /// Source name (for logging/debugging)
    fn source_name(&self) -> &str;
}
