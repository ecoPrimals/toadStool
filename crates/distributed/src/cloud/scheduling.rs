// SPDX-License-Identifier: AGPL-3.0-only
//! Hybrid cloud scheduling
//!
//! This module contains the hybrid cloud scheduler and related strategies.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toadstool::error::ToadStoolResult;

use crate::{UniversalJob, UniversalJobType};

/// Tracks per-provider cost models, accumulated usage, and budget alerts.
#[derive(Default)]
pub struct CloudCostTracker {
    cost_models: HashMap<String, super::types::CostModel>,
    usage_metrics: HashMap<String, f64>,
    alerts: Vec<super::types::CostAlert>,
}

impl CloudCostTracker {
    /// Creates an empty cost tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a cost model for a named provider (e.g. "aws", "gcp").
    pub fn set_cost_model(&mut self, provider: String, model: super::types::CostModel) {
        self.cost_models.insert(provider, model);
    }

    /// Record usage (accumulates) for a named metric key.
    pub fn record_usage(&mut self, key: String, amount: f64) {
        *self.usage_metrics.entry(key).or_default() += amount;
    }

    /// Returns current accumulated usage for a metric key.
    #[must_use]
    pub fn get_usage(&self, key: &str) -> f64 {
        self.usage_metrics.get(key).copied().unwrap_or(0.0)
    }

    /// Estimate cost for a provider given `core_hours` of compute.
    #[must_use]
    pub fn estimate_cost(&self, provider: &str, core_hours: f64) -> f64 {
        self.cost_models
            .get(provider)
            .map(|m| m.cpu_cost_per_core_hour * core_hours)
            .unwrap_or(0.0)
    }

    /// Return any active alerts.
    #[must_use]
    pub fn alerts(&self) -> &[super::types::CostAlert] {
        &self.alerts
    }

    /// Push a cost alert.
    pub fn add_alert(&mut self, alert: super::types::CostAlert) {
        self.alerts.push(alert);
    }
}

/// Tracks performance samples and baselines per provider.
#[derive(Default)]
pub struct CloudPerformanceTracker {
    performance_metrics: HashMap<String, super::types::PerformanceMetric>,
    baseline_metrics: HashMap<String, f64>,
}

impl CloudPerformanceTracker {
    /// Creates an empty performance tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a performance sample for a metric key.
    pub fn record_metric(&mut self, key: String, metric: super::types::PerformanceMetric) {
        self.performance_metrics.insert(key, metric);
    }

    /// Set the baseline value for a metric key.
    pub fn set_baseline(&mut self, key: String, value: f64) {
        self.baseline_metrics.insert(key, value);
    }

    /// Get the current metric value for a key.
    #[must_use]
    pub fn get_metric(&self, key: &str) -> Option<&super::types::PerformanceMetric> {
        self.performance_metrics.get(key)
    }

    /// Returns the ratio of current metric to baseline (>1.0 means above baseline).
    #[must_use]
    pub fn performance_ratio(&self, key: &str) -> Option<f64> {
        let current = self.performance_metrics.get(key)?;
        let baseline = self.baseline_metrics.get(key)?;
        if *baseline == 0.0 {
            return None;
        }
        Some(current.value / baseline)
    }
}

/// Hybrid cloud scheduler
pub struct HybridCloudScheduler {
    /// Selected scheduling strategy (cost, performance, compliance, etc.).
    pub(crate) _strategy: HybridSchedulingStrategy,
    /// Placeholder cost tracking state.
    pub(crate) _cost_tracker: CloudCostTracker,
    /// Placeholder performance tracking state.
    pub(crate) _performance_tracker: CloudPerformanceTracker,
    /// Default compliance requirements applied when constructing the scheduler.
    pub(crate) _compliance_requirements: super::types::ComplianceRequirements,
}

/// Hybrid scheduling strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HybridSchedulingStrategy {
    /// Cost-optimized scheduling
    CostOptimized,
    /// Performance-optimized scheduling
    PerformanceOptimized,
    /// Compliance-first scheduling
    ComplianceFirst,
    /// Weighted blend of cost, performance, and compliance objectives.
    Balanced {
        /// Relative weight for cost in the combined score.
        cost_weight: f64,
        /// Relative weight for performance.
        performance_weight: f64,
        /// Relative weight for compliance.
        compliance_weight: f64,
    },
    /// Prefer placement in or near the listed regions.
    GeographicAffinity {
        /// Region names to favor when choosing providers.
        preferred_regions: Vec<String>,
    },
    /// Cap end-to-end latency while restricting to certain regions.
    LatencySensitive {
        /// Maximum acceptable one-way or round-trip latency in milliseconds.
        max_latency_ms: u64,
        /// Regions considered for low-latency placement.
        target_regions: Vec<String>,
    },
    /// Prefer providers with higher renewable energy share.
    SustainabilityFocused {
        /// Weight 0.0-1.0 favoring renewable-powered infrastructure.
        renewable_energy_preference: f64,
    },
}

impl HybridCloudScheduler {
    /// Builds a scheduler with the given strategy and default compliance requirements.
    pub async fn new(strategy: HybridSchedulingStrategy) -> ToadStoolResult<Self> {
        let cost_tracker = CloudCostTracker::new();
        let performance_tracker = CloudPerformanceTracker::new();

        Ok(Self {
            _strategy: strategy,
            _cost_tracker: cost_tracker,
            _performance_tracker: performance_tracker,
            _compliance_requirements: super::types::ComplianceRequirements {
                certifications: vec![
                    super::types::ComplianceCertification::SOC2,
                    super::types::ComplianceCertification::ISO27001,
                ],
                regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
                data_sovereignty: vec![
                    super::types::DataSovereigntyRequirement {
                        data_type: "general".to_string(),
                        allowed_regions: vec!["US".to_string()],
                        encryption_required: true,
                    },
                    super::types::DataSovereigntyRequirement {
                        data_type: "general".to_string(),
                        allowed_regions: vec!["EU".to_string()],
                        encryption_required: true,
                    },
                ],
            },
        })
    }

    /// Returns heuristic per-provider performance scores for the given job type.
    pub async fn get_performance_estimates(
        &self,
        job: &UniversalJob,
    ) -> ToadStoolResult<HashMap<String, f64>> {
        let mut estimates = HashMap::new();

        // Calculate performance estimates based on job characteristics
        let complexity_factor = match job.job_type {
            Some(UniversalJobType::ComputeIntensive) => 1.0,
            Some(UniversalJobType::MemoryIntensive) => 0.8,
            Some(UniversalJobType::NetworkIntensive) => 0.6,
            Some(UniversalJobType::StorageIntensive) => 0.7,
            _ => 0.5,
        };

        estimates.insert("aws".to_string(), 100.0 * complexity_factor);
        estimates.insert("azure".to_string(), 95.0 * complexity_factor);
        estimates.insert("gcp".to_string(), 90.0 * complexity_factor);

        Ok(estimates)
    }

    /// Selects zero, one, or all available providers depending on list size and strategy.
    pub async fn select_providers(
        &self,
        _job: &UniversalJob,
        available_providers: &[String],
    ) -> ToadStoolResult<Vec<String>> {
        if available_providers.is_empty() {
            return Ok(vec![]);
        }
        if available_providers.len() >= 2 {
            return Ok(available_providers.to_vec());
        }
        Ok(vec![available_providers[0].clone()])
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
    };
    use std::time::SystemTime;
    use toadstool::ExecutionRequest;
    use uuid::Uuid;

    fn make_test_job(job_type: Option<UniversalJobType>) -> UniversalJob {
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type,
            execution_request: ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_cost_optimized() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_performance_optimized() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::PerformanceOptimized)
            .await
            .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_compliance_first() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::ComplianceFirst)
            .await
            .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_balanced() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::Balanced {
            cost_weight: 0.5,
            performance_weight: 0.3,
            compliance_weight: 0.2,
        })
        .await
        .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_geographic_affinity() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::GeographicAffinity {
            preferred_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
        })
        .await
        .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_latency_sensitive() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::LatencySensitive {
            max_latency_ms: 50,
            target_regions: vec!["us-east-1".to_string()],
        })
        .await
        .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_hybrid_cloud_scheduler_new_sustainability_focused() {
        let scheduler =
            HybridCloudScheduler::new(HybridSchedulingStrategy::SustainabilityFocused {
                renewable_energy_preference: 0.8,
            })
            .await
            .unwrap();
        let _ = scheduler;
    }

    #[tokio::test]
    async fn test_get_performance_estimates_compute_intensive() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
        let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
        assert!(estimates.contains_key("aws"));
        assert!(estimates.contains_key("azure"));
        assert!(estimates.contains_key("gcp"));
        assert_eq!(estimates.get("aws"), Some(&100.0));
    }

    #[tokio::test]
    async fn test_get_performance_estimates_memory_intensive() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::MemoryIntensive));
        let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
        assert_eq!(estimates.get("aws"), Some(&80.0));
    }

    #[tokio::test]
    async fn test_get_performance_estimates_network_intensive() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::NetworkIntensive));
        let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
        assert_eq!(estimates.get("aws"), Some(&60.0));
    }

    #[tokio::test]
    async fn test_get_performance_estimates_storage_intensive() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::StorageIntensive));
        let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
        assert_eq!(estimates.get("aws"), Some(&70.0));
    }

    #[tokio::test]
    async fn test_get_performance_estimates_default_job_type() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(None);
        let estimates = scheduler.get_performance_estimates(&job).await.unwrap();
        assert_eq!(estimates.get("aws"), Some(&50.0));
    }

    #[tokio::test]
    async fn test_select_providers_returns_all_when_multiple_registered() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
        let providers = scheduler
            .select_providers(&job, &["aws".to_string(), "gcp".to_string()])
            .await
            .unwrap();
        assert_eq!(providers, vec!["aws".to_string(), "gcp".to_string()]);
    }

    #[tokio::test]
    async fn test_select_providers_empty_when_none_registered() {
        let scheduler = HybridCloudScheduler::new(HybridSchedulingStrategy::CostOptimized)
            .await
            .unwrap();
        let job = make_test_job(Some(UniversalJobType::ComputeIntensive));
        let providers = scheduler.select_providers(&job, &[]).await.unwrap();
        assert!(providers.is_empty());
    }

    #[tokio::test]
    async fn test_cloud_cost_tracker_new() {
        let tracker = CloudCostTracker::new();
        let _ = tracker;
    }

    #[tokio::test]
    async fn test_cloud_performance_tracker_new() {
        let tracker = CloudPerformanceTracker::new();
        let _ = tracker;
    }
}
