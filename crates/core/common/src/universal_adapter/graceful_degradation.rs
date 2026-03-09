// SPDX-License-Identifier: AGPL-3.0-only
//! Graceful Degradation - Handle Missing Capabilities
//!
//! Provides strategies for gracefully handling situations where a requested
//! capability is not available.

use super::capability_types::{
    CapabilityHandle, CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint,
};
use crate::{
    platform_paths::{PathEnv, PlatformPaths},
    ToadStoolError, ToadStoolResult,
};
use std::collections::HashMap;

/// Graceful degradation strategy
pub struct GracefulDegradation {
    strategy: DegradationStrategy,
}

/// Degradation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationStrategy {
    /// Fail immediately if capability not available
    Fail,

    /// Use fallback implementation (if available)
    Fallback,

    /// Continue without the capability (degraded mode)
    Continue,
}

impl GracefulDegradation {
    /// Create a new graceful degradation handler
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strategy: DegradationStrategy::Fallback,
        }
    }

    /// Create with specific strategy
    #[must_use]
    pub const fn with_strategy(strategy: DegradationStrategy) -> Self {
        Self { strategy }
    }

    /// Handle a missing capability
    ///
    /// # Errors
    ///
    /// Returns [`ToadStoolError`] when strategy is `Fail` or `Fallback` and no fallback is available.
    pub fn handle_missing_capability(
        &self,
        capability: &CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        match self.strategy {
            DegradationStrategy::Fail => Err(ToadStoolError::not_found(format!(
                "Required capability not available: {capability:?}"
            ))),

            DegradationStrategy::Fallback => Self::try_fallback(capability),

            DegradationStrategy::Continue => Ok(Self::create_noop_handle(capability)),
        }
    }

    /// Try to provide a fallback implementation for the given capability.
    ///
    /// Provides local fallbacks where possible:
    /// - **Storage**: Local filesystem using platform data directory (degraded; no
    ///   compression, versioning, or other advanced features)
    ///
    /// Returns error for capability types with no fallback (Security, Coordination,
    /// Intelligence, Compute, Network, Monitoring).
    fn try_fallback(capability: &CapabilityType) -> ToadStoolResult<CapabilityHandle> {
        match capability {
            CapabilityType::Storage { .. } => Ok(Self::local_storage_fallback(capability)),
            _ => Err(ToadStoolError::not_found(format!(
                "Capability not available and no fallback: {capability:?}"
            ))),
        }
    }

    /// Local filesystem fallback for Storage capability.
    ///
    /// Uses platform data directory (e.g., `XDG_DATA_HOME`/toadstool). Degraded mode:
    /// no compression, versioning, deduplication, or other advanced features.
    fn local_storage_fallback(capability: &CapabilityType) -> CapabilityHandle {
        let env = PathEnv::from_env();
        let paths = PlatformPaths::new(&env);
        let storage_path = paths.toadstool_data_dir().join("fallback-storage");

        let provider_id = format!("local-fallback-{}", uuid::Uuid::new_v4().as_simple());
        let path_str = storage_path.to_string_lossy().to_string();
        let provider = CapabilityInfo {
            provider_id,
            capability: capability.clone(),
            metadata: HashMap::from([
                ("degradation".to_string(), "local-filesystem".to_string()),
                ("mode".to_string(), "graceful".to_string()),
                ("path".to_string(), path_str.clone()),
            ]),
            endpoint: ServiceEndpoint::Custom {
                protocol: "file".to_string(),
                address: path_str,
            },
            health: HealthStatus::Degraded,
        };
        CapabilityHandle::new(provider, capability.clone())
    }

    /// Create a no-op handle for continue mode.
    ///
    /// Returns a synthetic capability handle that represents "continue without this capability".
    /// The handle is valid but operations through it are no-ops. Useful for non-critical
    /// capabilities where the system can safely degrade.
    fn create_noop_handle(capability: &CapabilityType) -> CapabilityHandle {
        let provider_id = format!("noop-{}", uuid::Uuid::new_v4().as_simple());
        let provider = CapabilityInfo {
            provider_id,
            capability: capability.clone(),
            metadata: HashMap::from([
                ("degradation".to_string(), "noop".to_string()),
                ("mode".to_string(), "graceful".to_string()),
            ]),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Degraded,
        };
        CapabilityHandle::new(provider, capability.clone())
    }
}

impl Default for GracefulDegradation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal_adapter::{SecurityFeature, StorageFeature, TrustLevel};

    fn security_capability() -> CapabilityType {
        CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::High,
        }
    }

    #[tokio::test]
    async fn test_fail_strategy() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Fail);
        let result = degradation.handle_missing_capability(&security_capability());
        assert!(result.is_err(), "Should fail when capability not available");
    }

    #[tokio::test]
    async fn test_fallback_strategy_security_fails() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Fallback);
        let result = degradation.handle_missing_capability(&security_capability());
        assert!(result.is_err(), "Security has no fallback");
    }

    #[tokio::test]
    async fn test_fallback_strategy_storage_succeeds() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Fallback);
        let storage = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: None,
        };
        let result = degradation.handle_missing_capability(&storage);
        assert!(result.is_ok(), "Storage should have local fallback");
        let handle = result.expect("storage fallback");
        assert!(handle.provider_id().starts_with("local-fallback-"));
        assert!(handle.is_healthy());
    }

    #[test]
    fn test_strategy_types() {
        assert_ne!(DegradationStrategy::Fail, DegradationStrategy::Fallback);
        assert_ne!(DegradationStrategy::Fallback, DegradationStrategy::Continue);
    }

    #[test]
    fn test_default_strategy() {
        let degradation = GracefulDegradation::new();
        assert_eq!(degradation.strategy, DegradationStrategy::Fallback);
    }

    #[tokio::test]
    async fn test_continue_strategy_returns_noop_handle() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Continue);
        let result = degradation.handle_missing_capability(&security_capability());
        assert!(
            result.is_ok(),
            "Continue strategy should return no-op handle"
        );
        let handle = result.expect("continue strategy returns handle");
        assert!(handle.provider_id().starts_with("noop-"));
        assert!(handle.is_healthy());
    }
}
