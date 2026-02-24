//! Graceful Degradation - Handle Missing Capabilities
//!
//! Provides strategies for gracefully handling situations where a requested
//! capability is not available.

use super::capability_types::{
    CapabilityHandle, CapabilityInfo, CapabilityType, HealthStatus, ServiceEndpoint,
};
use crate::{ToadStoolError, ToadStoolResult};
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
    pub async fn handle_missing_capability(
        &self,
        capability: CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        match self.strategy {
            DegradationStrategy::Fail => Err(ToadStoolError::not_found(format!(
                "Required capability not available: {capability:?}"
            ))),

            DegradationStrategy::Fallback => {
                // Try to provide a fallback
                self.try_fallback(capability).await
            }

            DegradationStrategy::Continue => {
                // Return a "no-op" capability handle
                self.create_noop_handle(capability).await
            }
        }
    }

    /// Try to provide a fallback implementation
    async fn try_fallback(&self, capability: CapabilityType) -> ToadStoolResult<CapabilityHandle> {
        // For now, we don't have fallback implementations
        // In the future, we could provide:
        // - Mock implementations for testing
        // - Degraded implementations (e.g., no compression storage)
        // - Local implementations (e.g., local file storage)

        Err(ToadStoolError::not_found(format!(
            "Capability not available and no fallback: {capability:?}"
        )))
    }

    /// Create a no-op handle for continue mode
    ///
    /// Returns a synthetic capability handle that represents "continue without this capability".
    /// The handle is valid but operations through it are no-ops. Useful for non-critical
    /// capabilities where the system can safely degrade.
    async fn create_noop_handle(
        &self,
        capability: CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        let provider_id = format!("noop-{}", uuid::Uuid::new_v4().as_simple());
        let provider = CapabilityInfo {
            provider_id: provider_id.clone(),
            capability: capability.clone(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("degradation".to_string(), "noop".to_string());
                m.insert("mode".to_string(), "graceful".to_string());
                m
            },
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Degraded,
        };
        Ok(CapabilityHandle::new(provider, capability))
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
    use crate::universal_adapter::{SecurityFeature, TrustLevel};

    #[tokio::test]
    async fn test_fail_strategy() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Fail);

        let capability = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::High,
        };

        let result = degradation.handle_missing_capability(capability).await;
        assert!(result.is_err(), "Should fail when capability not available");
    }

    #[tokio::test]
    async fn test_fallback_strategy() {
        let degradation = GracefulDegradation::with_strategy(DegradationStrategy::Fallback);

        let capability = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::High,
        };

        let result = degradation.handle_missing_capability(capability).await;
        // For now, fallback also fails (no implementations yet)
        assert!(result.is_err());
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

        let capability = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::High,
        };

        let result = degradation.handle_missing_capability(capability).await;
        assert!(
            result.is_ok(),
            "Continue strategy should return no-op handle"
        );
        let handle = result.unwrap();
        assert!(handle.provider_id().starts_with("noop-"));
        assert!(handle.is_healthy()); // Degraded is considered healthy for no-op
    }
}
