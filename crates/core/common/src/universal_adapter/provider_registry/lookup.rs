// SPDX-License-Identifier: AGPL-3.0-only

use super::super::capability_types::{CapabilityInfo, CapabilityType, HealthStatus};
use super::{ProviderRegistry, RegisteredProvider};
use crate::ToadStoolResult;

impl ProviderRegistry {
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

    /// Get provider count
    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}
