//! Graceful Degradation - Handle Missing Capabilities
//!
//! Provides strategies for gracefully handling situations where a requested
//! capability is not available.

use super::capability_types::{CapabilityHandle, CapabilityType};
use crate::{ToadStoolError, ToadStoolResult};

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
    pub fn new() -> Self {
        Self {
            strategy: DegradationStrategy::Fallback,
        }
    }

    /// Create with specific strategy
    pub fn with_strategy(strategy: DegradationStrategy) -> Self {
        Self { strategy }
    }

    /// Handle a missing capability
    pub async fn handle_missing_capability(
        &self,
        capability: CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        match self.strategy {
            DegradationStrategy::Fail => Err(ToadStoolError::not_found(format!(
                "Required capability not available: {:?}",
                capability
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
            "Capability not available and no fallback: {:?}",
            capability
        )))
    }

    /// Create a no-op handle for continue mode
    async fn create_noop_handle(
        &self,
        capability: CapabilityType,
    ) -> ToadStoolResult<CapabilityHandle> {
        // This would create a handle that does nothing
        // Useful for non-critical capabilities

        Err(ToadStoolError::not_found(format!(
            "No-op mode not yet implemented for: {:?}",
            capability
        )))
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
}
