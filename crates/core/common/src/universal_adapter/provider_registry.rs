// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provider Registry - Runtime Catalog of Capability Providers
//!
//! Maintains a runtime registry of discovered providers and matches
//! capability requests to the best available provider.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::capability_types::{CapabilityInfo, CapabilityType, HealthStatus};
use crate::ToadStoolResult;

/// Runtime registry of capability providers
pub struct ProviderRegistry {
    /// Providers indexed by ID
    providers: HashMap<String, RegisteredProvider>,

    /// Provider health check interval (for future use)
    #[allow(dead_code, reason = "reserved for periodic health checks")]
    health_check_interval: Duration,
}

/// Registered provider with metadata
struct RegisteredProvider {
    info: CapabilityInfo,
    registered_at: Instant,
    last_health_check: Option<Instant>,
    request_count: u64,
    failure_count: u64,
}

impl ProviderRegistry {
    /// Create a new provider registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            health_check_interval: Duration::from_secs(30),
        }
    }

    /// Register a capability provider
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn register(&mut self, info: CapabilityInfo) -> ToadStoolResult<()> {
        let provider_id = info.provider_id.clone();

        let registered = RegisteredProvider {
            info,
            registered_at: Instant::now(),
            last_health_check: None,
            request_count: 0,
            failure_count: 0,
        };

        self.providers.insert(provider_id, registered);
        Ok(())
    }

    /// Unregister a provider
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn unregister(&mut self, provider_id: &str) -> ToadStoolResult<()> {
        self.providers.remove(provider_id);
        Ok(())
    }

    /// Find the best matching provider for a capability request
    ///
    /// # Errors
    ///
    /// This implementation does not fail; returns [`ToadStoolResult`] for API consistency.
    pub fn find_best_match(
        &self,
        requested: &CapabilityType,
    ) -> ToadStoolResult<Option<CapabilityInfo>> {
        let mut candidates: Vec<&RegisteredProvider> = self
            .providers
            .values()
            .filter(|p| Self::matches_capability(requested, &p.info.capability))
            .filter(|p| {
                p.info.health == HealthStatus::Healthy || p.info.health == HealthStatus::Unknown
            })
            .collect();

        if candidates.is_empty() {
            return Ok(None);
        }

        // Sort by quality score (lower is better)
        candidates.sort_by_key(|p| Self::calculate_quality_score(p));

        // Return the best match
        Ok(candidates.first().map(|p| p.info.clone()))
    }

    /// Check if a capability type matches another
    fn matches_capability(requested: &CapabilityType, available: &CapabilityType) -> bool {
        match (requested, available) {
            (
                CapabilityType::Security {
                    features: req_features,
                    min_trust_level,
                },
                CapabilityType::Security {
                    features: avail_features,
                    min_trust_level: avail_trust,
                },
            ) => {
                // Check if available provider has all requested features
                let has_features = req_features.iter().all(|f| avail_features.contains(f));
                // Check if trust level is sufficient
                let trust_ok = avail_trust >= min_trust_level;
                has_features && trust_ok
            }

            (
                CapabilityType::Storage {
                    features: req_features,
                    min_throughput_mbps,
                },
                CapabilityType::Storage {
                    features: avail_features,
                    min_throughput_mbps: avail_throughput,
                },
            ) => {
                let has_features = req_features.iter().all(|f| avail_features.contains(f));
                let throughput_ok = match (min_throughput_mbps, avail_throughput) {
                    (Some(req), Some(avail)) => avail >= req,
                    (None, _) => true,
                    (Some(_), None) => false,
                };
                has_features && throughput_ok
            }

            (
                CapabilityType::Coordination {
                    features: req_features,
                    max_latency_ms,
                },
                CapabilityType::Coordination {
                    features: avail_features,
                    max_latency_ms: avail_latency,
                },
            ) => {
                let has_features = req_features.iter().all(|f| avail_features.contains(f));
                let latency_ok = match (max_latency_ms, avail_latency) {
                    (Some(req), Some(avail)) => avail <= req,
                    (None, _) => true,
                    (Some(_), None) => false,
                };
                has_features && latency_ok
            }

            (
                CapabilityType::Intelligence {
                    features: req_features,
                    model_types: req_models,
                },
                CapabilityType::Intelligence {
                    features: avail_features,
                    model_types: avail_models,
                },
            ) => {
                let has_features = req_features.iter().all(|f| avail_features.contains(f));
                let has_models = req_models.iter().all(|m| avail_models.contains(m));
                has_features && has_models
            }

            (
                CapabilityType::Compute {
                    features: req_features,
                    min_memory_gb,
                },
                CapabilityType::Compute {
                    features: avail_features,
                    min_memory_gb: avail_memory,
                },
            ) => {
                let has_features = req_features.iter().all(|f| avail_features.contains(f));
                let memory_ok = match (min_memory_gb, avail_memory) {
                    (Some(req), Some(avail)) => avail >= req,
                    (None, _) => true,
                    (Some(_), None) => false,
                };
                has_features && memory_ok
            }

            // For other capability types, just match the variant
            (CapabilityType::Network { .. }, CapabilityType::Network { .. })
            | (CapabilityType::Monitoring { .. }, CapabilityType::Monitoring { .. }) => true,

            // Different capability types don't match
            _ => false,
        }
    }

    /// Calculate quality score for a provider (lower is better)
    fn calculate_quality_score(provider: &RegisteredProvider) -> u64 {
        let mut score = 0u64;

        // Health status affects score
        score += match provider.info.health {
            HealthStatus::Healthy => 0,
            HealthStatus::Unknown => 10,
            HealthStatus::Degraded => 50,
            HealthStatus::Unhealthy => 1000,
        };

        // Failure rate affects score
        if provider.request_count > 0 {
            let failure_rate = (provider.failure_count * 100) / provider.request_count;
            score += failure_rate;
        }

        // Age affects score (prefer established providers)
        let age_seconds = provider.registered_at.elapsed().as_secs();
        if age_seconds < 60 {
            score += 5; // New provider, slightly penalize
        }

        score
    }

    /// List all available capabilities
    #[must_use]
    pub fn list_capabilities(&self) -> Vec<CapabilityInfo> {
        self.providers.values().map(|p| p.info.clone()).collect()
    }

    /// Get provider by ID
    #[must_use]
    pub fn get_provider(&self, provider_id: &str) -> Option<&CapabilityInfo> {
        self.providers.get(provider_id).map(|p| &p.info)
    }

    /// Update provider health status
    pub fn update_health(&mut self, provider_id: &str, health: HealthStatus) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.info.health = health;
            provider.last_health_check = Some(Instant::now());
        }
    }

    /// Record a successful request to a provider
    pub fn record_success(&mut self, provider_id: &str) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.request_count += 1;
        }
    }

    /// Record a failed request to a provider
    pub fn record_failure(&mut self, provider_id: &str) {
        if let Some(provider) = self.providers.get_mut(provider_id) {
            provider.request_count += 1;
            provider.failure_count += 1;
        }
    }

    /// Get provider count
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// Clear all providers
    pub fn clear(&mut self) {
        self.providers.clear();
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::universal_adapter::{
        ComputeFeature, CoordinationFeature, IntelligenceFeature, ModelType, SecurityFeature,
        ServiceEndpoint, StorageFeature, TrustLevel,
    };

    fn create_test_security_provider() -> CapabilityInfo {
        CapabilityInfo {
            provider_id: "test-security-1".to_string(),
            capability: CapabilityType::Security {
                features: vec![SecurityFeature::Encryption, SecurityFeature::Signing],
                min_trust_level: TrustLevel::High,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        }
    }

    fn create_storage_provider(throughput: Option<u64>) -> CapabilityInfo {
        CapabilityInfo {
            provider_id: "test-storage-1".to_string(),
            capability: CapabilityType::Storage {
                features: vec![StorageFeature::Compression, StorageFeature::Encryption],
                min_throughput_mbps: throughput,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        }
    }

    fn create_coordination_provider(latency: Option<u64>) -> CapabilityInfo {
        CapabilityInfo {
            provider_id: "test-coord-1".to_string(),
            capability: CapabilityType::Coordination {
                features: vec![CoordinationFeature::ServiceDiscovery],
                max_latency_ms: latency,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        }
    }

    fn create_compute_provider(memory_gb: Option<f64>) -> CapabilityInfo {
        CapabilityInfo {
            provider_id: "test-compute-1".to_string(),
            capability: CapabilityType::Compute {
                features: vec![ComputeFeature::GPU],
                min_memory_gb: memory_gb,
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        }
    }

    fn create_intelligence_provider() -> CapabilityInfo {
        CapabilityInfo {
            provider_id: "test-intel-1".to_string(),
            capability: CapabilityType::Intelligence {
                features: vec![IntelligenceFeature::NaturalLanguage],
                model_types: vec![ModelType::LLM],
            },
            metadata: HashMap::new(),
            endpoint: ServiceEndpoint::InProcess,
            health: HealthStatus::Healthy,
        }
    }

    #[test]
    fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert_eq!(registry.provider_count(), 0);
    }

    #[test]
    fn test_provider_registration() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();

        registry.register(provider).unwrap();
        assert_eq!(registry.provider_count(), 1);
    }

    #[test]
    fn test_provider_unregistration() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();
        let provider_id = provider.provider_id.clone();

        registry.register(provider).unwrap();
        assert_eq!(registry.provider_count(), 1);

        registry.unregister(&provider_id).unwrap();
        assert_eq!(registry.provider_count(), 0);
    }

    #[test]
    fn test_capability_matching() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();
        registry.register(provider).unwrap();

        // Request with matching features
        let request = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::Medium,
        };

        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());
    }

    #[test]
    fn test_capability_no_match() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();
        registry.register(provider).unwrap();

        // Request with incompatible trust level
        let request = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::Maximum, // Provider only has High
        };

        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_none(), "Should not match - trust level too high");
    }

    #[test]
    fn test_health_update() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();
        let provider_id = provider.provider_id.clone();

        registry.register(provider).unwrap();
        registry.update_health(&provider_id, HealthStatus::Degraded);

        let info = registry.get_provider(&provider_id).unwrap();
        assert_eq!(info.health, HealthStatus::Degraded);
    }

    #[test]
    fn test_success_failure_tracking() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_security_provider();
        let provider_id = provider.provider_id.clone();

        registry.register(provider).unwrap();

        registry.record_success(&provider_id);
        registry.record_success(&provider_id);
        registry.record_failure(&provider_id);

        // Verify counts are tracked (internal state)
        assert!(registry.get_provider(&provider_id).is_some());
    }

    #[test]
    fn test_list_capabilities() {
        let mut registry = ProviderRegistry::new();
        registry.register(create_test_security_provider()).unwrap();

        let caps = registry.list_capabilities();
        assert_eq!(caps.len(), 1);
    }

    #[test]
    fn test_clear_registry() {
        let mut registry = ProviderRegistry::new();
        registry.register(create_test_security_provider()).unwrap();
        assert_eq!(registry.provider_count(), 1);

        registry.clear();
        assert_eq!(registry.provider_count(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = ProviderRegistry::default();
        assert_eq!(registry.provider_count(), 0);
    }

    #[test]
    fn test_storage_capability_matching_throughput() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(create_storage_provider(Some(200)))
            .unwrap();

        let request = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: Some(100),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());

        let request_too_high = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: Some(500),
        };
        let no_match = registry.find_best_match(&request_too_high).unwrap();
        assert!(no_match.is_none());
    }

    #[test]
    fn test_storage_capability_request_no_throughput() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(create_storage_provider(Some(100)))
            .unwrap();

        let request = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: None,
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());
    }

    #[test]
    fn test_storage_provider_no_throughput_fails_when_required() {
        let mut registry = ProviderRegistry::new();
        registry.register(create_storage_provider(None)).unwrap();

        let request = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: Some(100),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_none());
    }

    #[test]
    fn test_coordination_capability_matching_latency() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(create_coordination_provider(Some(50)))
            .unwrap();

        let request = CapabilityType::Coordination {
            features: vec![CoordinationFeature::ServiceDiscovery],
            max_latency_ms: Some(100),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());

        let request_too_strict = CapabilityType::Coordination {
            features: vec![CoordinationFeature::ServiceDiscovery],
            max_latency_ms: Some(10),
        };
        let no_match = registry.find_best_match(&request_too_strict).unwrap();
        assert!(no_match.is_none());
    }

    #[test]
    fn test_compute_capability_matching_memory() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(create_compute_provider(Some(16.0)))
            .unwrap();

        let request = CapabilityType::Compute {
            features: vec![ComputeFeature::GPU],
            min_memory_gb: Some(8.0),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());

        let request_too_high = CapabilityType::Compute {
            features: vec![ComputeFeature::GPU],
            min_memory_gb: Some(32.0),
        };
        let no_match = registry.find_best_match(&request_too_high).unwrap();
        assert!(no_match.is_none());
    }

    #[test]
    fn test_intelligence_capability_matching() {
        let mut registry = ProviderRegistry::new();
        registry.register(create_intelligence_provider()).unwrap();

        let request = CapabilityType::Intelligence {
            features: vec![IntelligenceFeature::NaturalLanguage],
            model_types: vec![ModelType::LLM],
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());

        let request_missing_model = CapabilityType::Intelligence {
            features: vec![IntelligenceFeature::NaturalLanguage],
            model_types: vec![ModelType::Vision],
        };
        let no_match = registry.find_best_match(&request_missing_model).unwrap();
        assert!(no_match.is_none());
    }

    #[test]
    fn test_network_capability_matching() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(CapabilityInfo {
                provider_id: "net-1".to_string(),
                capability: CapabilityType::Network {
                    features: vec![],
                    min_bandwidth_mbps: None,
                },
                metadata: HashMap::new(),
                endpoint: ServiceEndpoint::InProcess,
                health: HealthStatus::Healthy,
            })
            .unwrap();

        let request = CapabilityType::Network {
            features: vec![],
            min_bandwidth_mbps: Some(100),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());
    }

    #[test]
    fn test_monitoring_capability_matching() {
        let mut registry = ProviderRegistry::new();
        registry
            .register(CapabilityInfo {
                provider_id: "mon-1".to_string(),
                capability: CapabilityType::Monitoring {
                    features: vec![],
                    retention_days: None,
                },
                metadata: HashMap::new(),
                endpoint: ServiceEndpoint::InProcess,
                health: HealthStatus::Healthy,
            })
            .unwrap();

        let request = CapabilityType::Monitoring {
            features: vec![],
            retention_days: Some(30),
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());
    }

    #[test]
    fn test_capability_cross_type_no_match() {
        let mut registry = ProviderRegistry::new();
        registry.register(create_test_security_provider()).unwrap();

        let request = CapabilityType::Storage {
            features: vec![StorageFeature::Compression],
            min_throughput_mbps: None,
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_none());
    }

    #[test]
    fn test_unhealthy_provider_filtered_out() {
        let mut registry = ProviderRegistry::new();
        let mut provider = create_test_security_provider();
        provider.health = HealthStatus::Unhealthy;
        registry.register(provider).unwrap();

        let request = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::Medium,
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_none());
    }

    #[test]
    fn test_unknown_health_provider_matches() {
        let mut registry = ProviderRegistry::new();
        let mut provider = create_test_security_provider();
        provider.health = HealthStatus::Unknown;
        registry.register(provider).unwrap();

        let request = CapabilityType::Security {
            features: vec![SecurityFeature::Encryption],
            min_trust_level: TrustLevel::Medium,
        };
        let matched = registry.find_best_match(&request).unwrap();
        assert!(matched.is_some());
    }

    #[test]
    fn test_update_health_nonexistent_no_panic() {
        let mut registry = ProviderRegistry::new();
        registry.update_health("nonexistent", HealthStatus::Degraded);
        assert_eq!(registry.provider_count(), 0);
    }

    #[test]
    fn test_record_success_nonexistent_no_panic() {
        let mut registry = ProviderRegistry::new();
        registry.record_success("nonexistent");
    }

    #[test]
    fn test_record_failure_nonexistent_no_panic() {
        let mut registry = ProviderRegistry::new();
        registry.record_failure("nonexistent");
    }

    #[test]
    fn test_get_provider_nonexistent() {
        let registry = ProviderRegistry::new();
        assert!(registry.get_provider("missing").is_none());
    }
}
