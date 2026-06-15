// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Compute Scheduler
//!
//! Matches workloads to compute resources based on capabilities

use crate::compute_dispatch::UniversalComputeResourceDispatch;
use crate::universal::{ComputeRequirements, UniversalComputeResource};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;

/// Scheduling policy for resource selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    /// Select resource with best performance
    Performance,

    /// Select resource with best energy efficiency
    Efficiency,

    /// Balance load across available resources
    LoadBalance,

    /// Select resource with best capability match
    CapabilityMatch,

    /// Select resource with lowest latency
    LowLatency,
}

/// Performance history entry
#[derive(Debug, Clone)]
#[expect(dead_code, reason = "reserved for future performance metrics")]
struct PerformanceRecord {
    resource_id: String,
    workload_signature: String,
    execution_time: Duration,
    timestamp: Instant,
}

/// Universal compute scheduler
pub struct UniversalComputeScheduler {
    /// Available compute resources (GPU, CPU, TPU, etc.)
    resources: Arc<RwLock<Vec<Arc<UniversalComputeResourceDispatch>>>>,

    /// Scheduling policy
    policy: SchedulingPolicy,

    /// Performance history for learning
    history: Arc<RwLock<Vec<PerformanceRecord>>>,

    /// Resource utilization cache
    utilization_cache: Arc<RwLock<HashMap<String, (f32, Instant)>>>,
}

impl UniversalComputeScheduler {
    /// Create new scheduler with policy
    pub fn new(policy: SchedulingPolicy) -> Self {
        Self {
            resources: Arc::new(RwLock::new(Vec::new())),
            policy,
            history: Arc::new(RwLock::new(Vec::new())),
            utilization_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a compute resource (GPU, CPU, TPU, etc.)
    pub async fn register_resource(&self, resource: Arc<UniversalComputeResourceDispatch>) {
        let mut resources = self.resources.write().await;
        tracing::info!(
            "Registered compute resource: {} ({})",
            resource.resource_id(),
            resource.capabilities().resource_type
        );
        resources.push(resource);
    }

    /// Get list of all registered resources (as descriptive strings)
    pub async fn list_resources(&self) -> Vec<String> {
        let resources = self.resources.read().await;
        resources
            .iter()
            .map(|r| format!("{} ({})", r.resource_id(), r.capabilities().resource_type))
            .collect()
    }

    /// Get all registered compute resource objects
    pub async fn get_resources(&self) -> Vec<Arc<UniversalComputeResourceDispatch>> {
        let resources = self.resources.read().await;
        resources.clone()
    }

    /// Select best resource for workload based on policy
    ///
    /// # Errors
    ///
    /// Returns when no suitable resource is registered or ranking fails.
    pub async fn select_resource(
        &self,
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<Arc<UniversalComputeResourceDispatch>> {
        // Clone capable resources before await to avoid holding lock across .await
        let capable: Vec<Arc<UniversalComputeResourceDispatch>> = {
            let resources = self.resources.read().await;
            if resources.is_empty() {
                return Err(ToadStoolError::runtime("No compute resources registered"));
            }
            resources
                .iter()
                .filter(|r| r.can_execute(requirements))
                .map(Arc::clone)
                .collect()
        };

        if capable.is_empty() {
            return Err(ToadStoolError::runtime(
                "No compute resource has required capabilities",
            ));
        }

        // 2. Rank by policy (lock released before await)
        let best = self.rank_resources_owned(&capable, requirements).await?;

        tracing::info!(
            "Selected resource: {} for workload (policy: {:?})",
            best.resource_id(),
            self.policy
        );

        Ok(best)
    }

    /// Rank resources according to scheduling policy (owned Vec to avoid lock across await)
    async fn rank_resources_owned(
        &self,
        resources: &[Arc<UniversalComputeResourceDispatch>],
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<Arc<UniversalComputeResourceDispatch>> {
        let refs: Vec<&Arc<UniversalComputeResourceDispatch>> = resources.iter().collect();
        match self.policy {
            SchedulingPolicy::Performance => self
                .select_by_performance(&refs, requirements)
                .await
                .map(Arc::clone),
            SchedulingPolicy::Efficiency => self
                .select_by_efficiency(&refs, requirements)
                .await
                .map(Arc::clone),
            SchedulingPolicy::LoadBalance => {
                self.select_by_load_balance(&refs).await.map(Arc::clone)
            }
            SchedulingPolicy::CapabilityMatch => self
                .select_by_capability_match(&refs, requirements)
                .map(Arc::clone),
            SchedulingPolicy::LowLatency => {
                self.select_by_latency(&refs, requirements).map(Arc::clone)
            }
        }
    }

    /// Select resource with best estimated performance
    async fn select_by_performance<'a>(
        &self,
        resources: &'a [&'a Arc<UniversalComputeResourceDispatch>],
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<&'a Arc<UniversalComputeResourceDispatch>> {
        // Check history first
        let workload_sig = self.workload_signature(requirements);
        let history = self.history.read().await;

        // If we have performance data, use it
        let best_from_history = history
            .iter()
            .filter(|r| r.workload_signature == workload_sig)
            .min_by_key(|r| r.execution_time)
            .map(|r| (r.resource_id.clone(), r.execution_time));
        drop(history);

        if let Some((resource_id, _execution_time)) = best_from_history {
            // Find resource with this ID
            if let Some(resource) = resources.iter().find(|r| r.resource_id() == resource_id) {
                return Ok(resource);
            }
        }

        // No history, estimate based on capabilities
        let mut scored: Vec<_> = resources
            .iter()
            .map(|r| {
                let estimated_time = r.estimate_execution_time(requirements);
                (r, estimated_time)
            })
            .collect();

        scored.sort_by_key(|(_, time)| *time);

        Ok(scored[0].0)
    }

    /// Select resource with best energy efficiency
    async fn select_by_efficiency<'a>(
        &self,
        resources: &'a [&'a Arc<UniversalComputeResourceDispatch>],
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<&'a Arc<UniversalComputeResourceDispatch>> {
        let mut scored: Vec<_> = resources
            .iter()
            .map(|r| {
                let caps = r.capabilities();
                let estimated_time = r.estimate_execution_time(requirements);
                let energy_joules = caps.performance.power_watts * estimated_time.as_secs_f32();
                (r, energy_joules)
            })
            .collect();

        scored.sort_by(|(_, e1), (_, e2)| e1.partial_cmp(e2).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored[0].0)
    }

    /// Select least utilized resource
    async fn select_by_load_balance<'a>(
        &self,
        resources: &'a [&'a Arc<UniversalComputeResourceDispatch>],
    ) -> ToadStoolResult<&'a Arc<UniversalComputeResourceDispatch>> {
        // Get utilization for each resource
        let mut utilizations = Vec::new();
        for resource in resources {
            let utilization = self.get_cached_utilization(resource).await;
            utilizations.push((resource, utilization));
        }

        // Sort by utilization (ascending)
        utilizations
            .sort_by(|(_, u1), (_, u2)| u1.partial_cmp(u2).unwrap_or(std::cmp::Ordering::Equal));

        Ok(utilizations[0].0)
    }

    /// Select resource with best capability match score
    fn select_by_capability_match<'a>(
        &self,
        resources: &'a [&'a Arc<UniversalComputeResourceDispatch>],
        requirements: &ComputeRequirements,
    ) -> ToadStoolResult<&'a Arc<UniversalComputeResourceDispatch>> {
        let mut scored: Vec<_> = resources
            .iter()
            .map(|r| {
                let score = r.score_workload(requirements);
                (r, score)
            })
            .collect();

        scored.sort_by(|(_, s1), (_, s2)| s2.partial_cmp(s1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored[0].0)
    }

    /// Select resource with lowest startup latency
    fn select_by_latency<'a>(
        &self,
        resources: &'a [&'a Arc<UniversalComputeResourceDispatch>],
        _requirements: &ComputeRequirements,
    ) -> ToadStoolResult<&'a Arc<UniversalComputeResourceDispatch>> {
        let mut scored: Vec<_> = resources
            .iter()
            .map(|r| {
                let latency = r.capabilities().performance.startup_latency_us;
                (r, latency)
            })
            .collect();

        scored.sort_by_key(|(_, lat)| *lat);

        Ok(scored[0].0)
    }

    /// Get cached utilization or query resource
    async fn get_cached_utilization(
        &self,
        resource: &Arc<UniversalComputeResourceDispatch>,
    ) -> f32 {
        let resource_id = resource.resource_id().to_string();
        // Check cache first, release lock before await
        {
            let cache = self.utilization_cache.read().await;
            const UTILIZATION_CACHE_TTL: Duration = Duration::from_secs(1);
            if let Some((utilization, timestamp)) = cache.get(&resource_id)
                && timestamp.elapsed() < UTILIZATION_CACHE_TTL
            {
                return *utilization;
            }
        }

        // Query resource (lock released - do NOT hold across await)
        let utilization = resource.utilization().await;

        // Update cache
        let mut cache = self.utilization_cache.write().await;
        cache.insert(resource_id, (utilization, Instant::now()));
        utilization
    }

    /// Record execution performance for future decisions
    pub async fn record_performance(
        &self,
        resource_id: &str,
        requirements: &ComputeRequirements,
        execution_time: Duration,
    ) {
        let record = PerformanceRecord {
            resource_id: resource_id.to_string(),
            workload_signature: self.workload_signature(requirements),
            execution_time,
            timestamp: Instant::now(),
        };

        let mut history = self.history.write().await;
        history.push(record);

        // Keep history reasonable size (last 1000 records)
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    /// Generate signature for workload (for history matching)
    fn workload_signature(&self, requirements: &ComputeRequirements) -> String {
        format!(
            "threads:{}_mem:{}_ops:{}",
            requirements.min_parallel_threads,
            requirements.memory_bytes,
            requirements.operations.len()
        )
    }
}

impl Default for UniversalComputeScheduler {
    fn default() -> Self {
        Self::new(SchedulingPolicy::CapabilityMatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_resource::CpuComputeResource;
    use std::time::Duration;

    #[tokio::test]
    async fn test_scheduler_creation() {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);
        assert_eq!(scheduler.policy, SchedulingPolicy::Performance);
    }

    #[tokio::test]
    async fn test_no_resources() {
        let scheduler = UniversalComputeScheduler::default();
        let requirements = ComputeRequirements::default();

        let result = scheduler.select_resource(&requirements).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_scheduler_default() {
        let scheduler = UniversalComputeScheduler::default();
        let list = scheduler.list_resources().await;
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_scheduler_register_and_list() {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);
        let cpu = CpuComputeResource::new().expect("CPU resource");
        scheduler
            .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
            .await;

        let list = scheduler.list_resources().await;
        assert!(!list.is_empty());
        assert!(list[0].contains("CPU"));
    }

    #[tokio::test]
    async fn test_scheduler_select_with_cpu() {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);
        let cpu = CpuComputeResource::new().expect("CPU resource");
        scheduler
            .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
            .await;

        let requirements = ComputeRequirements::default();
        let resource = scheduler.select_resource(&requirements).await;
        assert!(resource.is_ok());
    }

    #[tokio::test]
    async fn test_scheduler_all_policies() {
        for policy in [
            SchedulingPolicy::Performance,
            SchedulingPolicy::Efficiency,
            SchedulingPolicy::LoadBalance,
            SchedulingPolicy::CapabilityMatch,
            SchedulingPolicy::LowLatency,
        ] {
            let scheduler = UniversalComputeScheduler::new(policy);
            let cpu = CpuComputeResource::new().expect("CPU resource");
            scheduler
                .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
                .await;

            let requirements = ComputeRequirements::default();
            let result = scheduler.select_resource(&requirements).await;
            assert!(result.is_ok(), "policy {policy:?} should select resource");
        }
    }

    #[tokio::test]
    async fn test_scheduler_record_performance() {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);
        let cpu = CpuComputeResource::new().expect("CPU resource");
        let resource_id = cpu.resource_id().to_string();
        scheduler
            .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
            .await;

        scheduler
            .record_performance(
                &resource_id,
                &ComputeRequirements::default(),
                Duration::from_millis(100),
            )
            .await;

        let requirements = ComputeRequirements::default();
        let _ = scheduler.select_resource(&requirements).await;
    }

    #[tokio::test]
    async fn test_scheduler_get_resources() {
        let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);
        let cpu = CpuComputeResource::new().expect("CPU resource");
        scheduler
            .register_resource(Arc::new(UniversalComputeResourceDispatch::Cpu(cpu)))
            .await;

        let resources = scheduler.get_resources().await;
        assert_eq!(resources.len(), 1);
    }
}
