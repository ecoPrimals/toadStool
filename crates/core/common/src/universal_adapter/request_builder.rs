// SPDX-License-Identifier: AGPL-3.0-or-later
//! Request Builder - Fluent API for Capability Requests
//!
//! Provides a fluent builder pattern for constructing capability requests.

use super::capability_types::{
    CapabilityType, CoordinationFeature, IntelligenceFeature, ModelType, SecurityFeature,
    StorageFeature, TrustLevel,
};

/// Builder for capability requests
pub struct CapabilityRequestBuilder;

impl CapabilityRequestBuilder {
    /// Create a new request builder
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Request security capability
    #[must_use]
    pub fn security(self) -> SecurityRequestBuilder {
        SecurityRequestBuilder {
            features: vec![],
            min_trust_level: TrustLevel::Medium,
        }
    }

    /// Request storage capability
    #[must_use]
    pub fn storage(self) -> StorageRequestBuilder {
        StorageRequestBuilder {
            features: vec![],
            min_throughput_mbps: None,
        }
    }

    /// Request coordination capability
    #[must_use]
    pub fn coordination(self) -> CoordinationRequestBuilder {
        CoordinationRequestBuilder {
            features: vec![],
            max_latency_ms: None,
        }
    }

    /// Request intelligence capability
    #[must_use]
    pub fn intelligence(self) -> IntelligenceRequestBuilder {
        IntelligenceRequestBuilder {
            features: vec![],
            model_types: vec![],
        }
    }
}

impl Default for CapabilityRequestBuilder {
    fn default() -> Self {
        Self
    }
}

/// Builder for security capability requests
pub struct SecurityRequestBuilder {
    features: Vec<SecurityFeature>,
    min_trust_level: TrustLevel,
}

impl SecurityRequestBuilder {
    /// Add encryption feature
    #[must_use]
    pub fn with_encryption(mut self) -> Self {
        self.features.push(SecurityFeature::Encryption);
        self
    }

    /// Add signing feature
    #[must_use]
    pub fn with_signing(mut self) -> Self {
        self.features.push(SecurityFeature::Signing);
        self
    }

    /// Add audit feature
    #[must_use]
    pub fn with_audit(mut self) -> Self {
        self.features.push(SecurityFeature::Audit);
        self
    }

    /// Set minimum trust level
    #[must_use]
    pub const fn min_trust_level(mut self, level: TrustLevel) -> Self {
        self.min_trust_level = level;
        self
    }

    /// Build the capability type
    #[must_use]
    pub fn build(self) -> CapabilityType {
        CapabilityType::Security {
            features: self.features,
            min_trust_level: self.min_trust_level,
        }
    }
}

/// Builder for storage capability requests
pub struct StorageRequestBuilder {
    features: Vec<StorageFeature>,
    min_throughput_mbps: Option<u64>,
}

impl StorageRequestBuilder {
    /// Add compression feature
    #[must_use]
    pub fn with_compression(mut self) -> Self {
        self.features.push(StorageFeature::Compression);
        self
    }

    /// Add encryption feature
    #[must_use]
    pub fn with_encryption(mut self) -> Self {
        self.features.push(StorageFeature::Encryption);
        self
    }

    /// Add versioning feature
    #[must_use]
    pub fn with_versioning(mut self) -> Self {
        self.features.push(StorageFeature::Versioning);
        self
    }

    /// Set minimum throughput
    #[must_use]
    pub const fn min_throughput_mbps(mut self, throughput: u64) -> Self {
        self.min_throughput_mbps = Some(throughput);
        self
    }

    /// Build the capability type
    #[must_use]
    pub fn build(self) -> CapabilityType {
        CapabilityType::Storage {
            features: self.features,
            min_throughput_mbps: self.min_throughput_mbps,
        }
    }
}

/// Builder for coordination capability requests
pub struct CoordinationRequestBuilder {
    features: Vec<CoordinationFeature>,
    max_latency_ms: Option<u64>,
}

impl CoordinationRequestBuilder {
    /// Add service discovery feature
    #[must_use]
    pub fn with_service_discovery(mut self) -> Self {
        self.features.push(CoordinationFeature::ServiceDiscovery);
        self
    }

    /// Add load balancing feature
    #[must_use]
    pub fn with_load_balancing(mut self) -> Self {
        self.features.push(CoordinationFeature::LoadBalancing);
        self
    }

    /// Set maximum latency
    #[must_use]
    pub const fn max_latency_ms(mut self, latency: u64) -> Self {
        self.max_latency_ms = Some(latency);
        self
    }

    /// Build the capability type
    #[must_use]
    pub fn build(self) -> CapabilityType {
        CapabilityType::Coordination {
            features: self.features,
            max_latency_ms: self.max_latency_ms,
        }
    }
}

/// Builder for intelligence capability requests
pub struct IntelligenceRequestBuilder {
    features: Vec<IntelligenceFeature>,
    model_types: Vec<ModelType>,
}

impl IntelligenceRequestBuilder {
    /// Add natural language feature
    #[must_use]
    pub fn with_natural_language(mut self) -> Self {
        self.features.push(IntelligenceFeature::NaturalLanguage);
        self
    }

    /// Add code generation feature
    #[must_use]
    pub fn with_code_generation(mut self) -> Self {
        self.features.push(IntelligenceFeature::CodeGeneration);
        self
    }

    /// Add LLM model type
    #[must_use]
    pub fn with_llm(mut self) -> Self {
        self.model_types.push(ModelType::LLM);
        self
    }

    /// Build the capability type
    #[must_use]
    pub fn build(self) -> CapabilityType {
        CapabilityType::Intelligence {
            features: self.features,
            model_types: self.model_types,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_builder() {
        let capability = CapabilityRequestBuilder::new()
            .security()
            .with_encryption()
            .with_signing()
            .min_trust_level(TrustLevel::High)
            .build();

        assert!(matches!(capability, CapabilityType::Security { .. }));
        if let CapabilityType::Security {
            features,
            min_trust_level,
        } = &capability
        {
            assert_eq!(features.len(), 2);
            assert_eq!(*min_trust_level, TrustLevel::High);
        }
    }

    #[test]
    fn test_storage_builder() {
        let capability = CapabilityRequestBuilder::new()
            .storage()
            .with_compression()
            .with_encryption()
            .min_throughput_mbps(100)
            .build();

        assert!(matches!(capability, CapabilityType::Storage { .. }));
        if let CapabilityType::Storage {
            features,
            min_throughput_mbps,
        } = &capability
        {
            assert_eq!(features.len(), 2);
            assert_eq!(*min_throughput_mbps, Some(100));
        }
    }

    #[test]
    fn test_coordination_builder() {
        let capability = CapabilityRequestBuilder::new()
            .coordination()
            .with_service_discovery()
            .with_load_balancing()
            .max_latency_ms(10)
            .build();

        assert!(matches!(capability, CapabilityType::Coordination { .. }));
        if let CapabilityType::Coordination {
            features,
            max_latency_ms,
        } = &capability
        {
            assert_eq!(features.len(), 2);
            assert_eq!(*max_latency_ms, Some(10));
        }
    }

    #[test]
    fn test_intelligence_builder() {
        let capability = CapabilityRequestBuilder::new()
            .intelligence()
            .with_natural_language()
            .with_llm()
            .build();

        assert!(matches!(capability, CapabilityType::Intelligence { .. }));
        if let CapabilityType::Intelligence {
            features,
            model_types,
        } = &capability
        {
            assert_eq!(features.len(), 1);
            assert_eq!(model_types.len(), 1);
        }
    }
}
