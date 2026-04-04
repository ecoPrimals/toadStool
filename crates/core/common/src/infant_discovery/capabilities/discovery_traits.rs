// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2025 ecoPrimals

use super::discovered::{DiscoveredService, DiscoveryPreferences};
use std::time::Duration;

/// Capability discovery trait - implemented by discovery engines
pub trait CapabilityDiscovery: Send + Sync {
    /// Discover a service by capability name (not primal name!)
    ///
    /// # Examples
    /// ```ignore
    /// // ✅ GOOD - capability-based
    /// let ai = discovery.discover("ai_processing").await?;
    /// let auth = discovery.discover("authentication").await?;
    ///
    /// // ❌ BAD - primal name hardcoding
    /// // let songbird = SongbirdClient::new();  // DON'T DO THIS
    /// ```
    fn discover(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    >;

    /// Discover with preferences
    fn discover_with_preferences(
        &self,
        capability: &str,
        preferences: DiscoveryPreferences,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<DiscoveredService, DiscoveryError>> + Send + '_,
        >,
    >;

    /// Discover all services providing a capability
    fn discover_all(
        &self,
        capability: &str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<DiscoveredService>, DiscoveryError>>
                + Send
                + '_,
        >,
    >;

    /// Check if a capability is available
    fn is_available<'a>(
        &'a self,
        capability: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async move { self.discover(capability).await.is_ok() })
    }
}

/// Errors during capability discovery
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// Requested capability was not found by any source
    #[error("Capability '{0}' not found")]
    CapabilityNotFound(String),

    /// Discovery operation exceeded timeout
    #[error("Discovery timeout after {0:?}")]
    Timeout(Duration),

    /// No services with acceptable health were found
    #[error("No healthy services found for capability '{0}'")]
    NoHealthyServices(String),

    /// No discovered service supports the required protocol
    #[error("Protocol '{0}' not supported by any discovered service")]
    ProtocolNotSupported(String),

    /// A discovery source failed during resolution
    #[error("Discovery source failed: {0}")]
    SourceFailed(String),

    /// Configuration was invalid or missing
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
