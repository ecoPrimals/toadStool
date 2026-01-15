//! Request Builder - Fluent API for Capability Requests
//!
//! Provides a fluent builder pattern for constructing capability requests.

use super::capability_types::*;

/// Builder for capability requests
pub struct CapabilityRequestBuilder {
    #[allow(dead_code)]
    capability: Option<CapabilityType>,
}

impl CapabilityRequestBuilder {
    /// Create a new request builder
    pub fn new() -> Self {
        Self { capability: None }
    }

    /// Request security capability
    pub fn security(self) -> SecurityRequestBuilder {
        SecurityRequestBuilder {
            features: vec![],
            min_trust_level: TrustLevel::Medium,
        }
    }

    /// Request storage capability
    pub fn storage(self) -> StorageRequestBuilder {
        StorageRequestBuilder {
            features: vec![],
            min_throughput_mbps: None,
        }
    }

    /// Request coordination capability
    pub fn coordination(self) -> CoordinationRequestBuilder {
        CoordinationRequestBuilder {
            features: vec![],
            max_latency_ms: None,
        }
    }

    /// Request intelligence capability
    pub fn intelligence(self) -> IntelligenceRequestBuilder {
        IntelligenceRequestBuilder {
            features: vec![],
            model_types: vec![],
        }
    }
}

impl Default for CapabilityRequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for security capability requests
pub struct SecurityRequestBuilder {
    features: Vec<SecurityFeature>,
    min_trust_level: TrustLevel,
}

impl SecurityRequestBuilder {
    /// Add encryption feature
    pub fn with_encryption(mut self) -> Self {
        self.features.push(SecurityFeature::Encryption);
        self
    }

    /// Add signing feature
    pub fn with_signing(mut self) -> Self {
        self.features.push(SecurityFeature::Signing);
        self
    }

    /// Add audit feature
    pub fn with_audit(mut self) -> Self {
        self.features.push(SecurityFeature::Audit);
        self
    }

    /// Set minimum trust level
    pub fn min_trust_level(mut self, level: TrustLevel) -> Self {
        self.min_trust_level = level;
        self
    }

    /// Build the capability type
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
    pub fn with_compression(mut self) -> Self {
        self.features.push(StorageFeature::Compression);
        self
    }

    /// Add encryption feature
    pub fn with_encryption(mut self) -> Self {
        self.features.push(StorageFeature::Encryption);
        self
    }

    /// Add versioning feature
    pub fn with_versioning(mut self) -> Self {
        self.features.push(StorageFeature::Versioning);
        self
    }

    /// Set minimum throughput
    pub fn min_throughput_mbps(mut self, throughput: u64) -> Self {
        self.min_throughput_mbps = Some(throughput);
        self
    }

    /// Build the capability type
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
    pub fn with_service_discovery(mut self) -> Self {
        self.features.push(CoordinationFeature::ServiceDiscovery);
        self
    }

    /// Add load balancing feature
    pub fn with_load_balancing(mut self) -> Self {
        self.features.push(CoordinationFeature::LoadBalancing);
        self
    }

    /// Set maximum latency
    pub fn max_latency_ms(mut self, latency: u64) -> Self {
        self.max_latency_ms = Some(latency);
        self
    }

    /// Build the capability type
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
    pub fn with_natural_language(mut self) -> Self {
        self.features.push(IntelligenceFeature::NaturalLanguage);
        self
    }

    /// Add code generation feature
    pub fn with_code_generation(mut self) -> Self {
        self.features.push(IntelligenceFeature::CodeGeneration);
        self
    }

    /// Add LLM model type
    pub fn with_llm(mut self) -> Self {
        self.model_types.push(ModelType::LLM);
        self
    }

    /// Build the capability type
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

        match capability {
            CapabilityType::Security { features, min_trust_level } => {
                assert_eq!(features.len(), 2);
                assert_eq!(min_trust_level, TrustLevel::High);
            }
            _ => panic!("Should be Security capability"),
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

        match capability {
            CapabilityType::Storage { features, min_throughput_mbps } => {
                assert_eq!(features.len(), 2);
                assert_eq!(min_throughput_mbps, Some(100));
            }
            _ => panic!("Should be Storage capability"),
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

        match capability {
            CapabilityType::Coordination { features, max_latency_ms } => {
                assert_eq!(features.len(), 2);
                assert_eq!(max_latency_ms, Some(10));
            }
            _ => panic!("Should be Coordination capability"),
        }
    }

    #[test]
    fn test_intelligence_builder() {
        let capability = CapabilityRequestBuilder::new()
            .intelligence()
            .with_natural_language()
            .with_llm()
            .build();

        match capability {
            CapabilityType::Intelligence { features, model_types } => {
                assert_eq!(features.len(), 1);
                assert_eq!(model_types.len(), 1);
            }
            _ => panic!("Should be Intelligence capability"),
        }
    }
}
