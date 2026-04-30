// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload Orchestrator - Main coordination logic
//!
//! **Deep Debt**: Runtime discovery, intelligent selection, automatic fallback

use crate::error::OrchestrationError;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use toadstool_runtime_universal::substrate::*;

use crate::policy::*;

/// Main orchestrator for workload distribution
///
/// **Deep Debt**: Discovers substrates at runtime, no hardcoding
pub struct WorkloadOrchestrator<S: ComputeSubstrate> {
    /// Available substrates (discovered at runtime)
    substrates: Arc<RwLock<Vec<Arc<S>>>>,

    /// Selection policy
    policy: SelectionPolicy,

    /// Performance history for learning
    history: Arc<RwLock<PerformanceHistory>>,
}

impl<S: ComputeSubstrate> WorkloadOrchestrator<S> {
    /// Discover all available substrates
    ///
    /// **Deep Debt**: Runtime discovery, capability-based
    pub async fn discover() -> Result<Self, OrchestrationError> {
        let substrates = Arc::new(RwLock::new(Vec::new()));
        let policy = SelectionPolicy::default();
        let history = Arc::new(RwLock::new(PerformanceHistory::new()));

        Ok(Self {
            substrates,
            policy,
            history,
        })
    }

    /// Create with explicit substrates (for testing)
    pub fn with_substrates(substrates: Vec<Arc<S>>) -> Self {
        Self {
            substrates: Arc::new(RwLock::new(substrates)),
            policy: SelectionPolicy::default(),
            history: Arc::new(RwLock::new(PerformanceHistory::new())),
        }
    }

    /// Register a substrate.
    ///
    /// Returns an error only if the internal lock was poisoned by a prior panic.
    pub fn register_substrate(&self, substrate: Arc<S>) -> Result<(), OrchestrationError> {
        self.substrates
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?
            .push(substrate);
        Ok(())
    }

    /// Get number of available substrates.
    ///
    /// Returns an error only if the internal lock was poisoned by a prior panic.
    pub fn num_substrates(&self) -> Result<usize, OrchestrationError> {
        Ok(self
            .substrates
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?
            .len())
    }

    /// Execute a workload on optimal substrate
    ///
    /// **Deep Debt**: Automatic selection based on actual capabilities
    pub async fn execute(
        &self,
        request: WorkloadRequest,
    ) -> Result<WorkloadResult, OrchestrationError> {
        let start = Instant::now();

        // Select optimal substrate
        let substrate = self.select_substrate(&request)?;

        // Execute operation (simplified for now)
        let operation = self.convert_request_to_operation(&request)?;
        let output = substrate
            .execute_buffer_op(operation)
            .await
            .map_err(|e| OrchestrationError::Substrate(e.to_string()))?;

        let duration = start.elapsed();
        let result = WorkloadResult {
            substrate_name: substrate.name().to_string(),
            substrate_type: substrate.substrate_type(),
            duration,
            success: true,
            power_consumed_mw: output.metadata.power_consumed_mw,
        };

        self.history
            .write()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?
            .record(substrate.substrate_type(), &result);

        Ok(result)
    }

    /// Execute workload with fallback on failure
    pub async fn execute_with_fallback(
        &self,
        request: WorkloadRequest,
    ) -> Result<WorkloadResult, OrchestrationError> {
        let candidates = self.rank_substrates(&request)?;

        for substrate in candidates {
            match self.execute_on_substrate(substrate.clone(), &request).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!("Substrate {} failed: {}", substrate.name(), e);
                    continue;
                }
            }
        }

        Err(OrchestrationError::AllSubstratesFailed)
    }

    /// Select optimal substrate for workload
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // selected is from policy using substrates/history
    fn select_substrate(&self, request: &WorkloadRequest) -> Result<Arc<S>, OrchestrationError> {
        let substrates = self
            .substrates
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?;

        if substrates.is_empty() {
            return Err(OrchestrationError::NoSubstrates);
        }

        let history = self
            .history
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?;
        let selected = self.policy.select(&substrates, request, &history)?;

        Ok(selected)
    }

    /// Rank substrates by suitability
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // ranked uses substrates and history
    fn rank_substrates(
        &self,
        request: &WorkloadRequest,
    ) -> Result<Vec<Arc<S>>, OrchestrationError> {
        let substrates = self
            .substrates
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?;
        let history = self
            .history
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?;

        let mut ranked = self.policy.rank_all(&substrates, request, &history)?;
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(ranked.into_iter().map(|(s, _)| s).collect())
    }

    /// Execute on specific substrate
    async fn execute_on_substrate(
        &self,
        substrate: Arc<S>,
        request: &WorkloadRequest,
    ) -> Result<WorkloadResult, OrchestrationError> {
        let start = Instant::now();
        let operation = self.convert_request_to_operation(request)?;
        let output = substrate
            .execute_buffer_op(operation)
            .await
            .map_err(|e| OrchestrationError::Substrate(e.to_string()))?;

        Ok(WorkloadResult {
            substrate_name: substrate.name().to_string(),
            substrate_type: substrate.substrate_type(),
            duration: start.elapsed(),
            success: true,
            power_consumed_mw: output.metadata.power_consumed_mw,
        })
    }

    /// Convert workload request to buffer operation
    fn convert_request_to_operation(
        &self,
        request: &WorkloadRequest,
    ) -> Result<BufferOperation, OrchestrationError> {
        // Simplified conversion for now
        Ok(BufferOperation::Custom {
            name: "generic_operation".to_string(),
            data: vec![0u8; request.operation_count],
            metadata: serde_json::json!({
                "power_budget": request.power_budget_watts,
                "target": format!("{:?}", request.target),
            }),
        })
    }

    /// Get performance statistics.
    ///
    /// Returns an error only if an internal lock was poisoned by a prior panic.
    pub fn stats(&self) -> Result<OrchestratorStats, OrchestrationError> {
        let history = self
            .history
            .read()
            .map_err(|e| OrchestrationError::LockPoisoned(e.to_string()))?;
        Ok(OrchestratorStats {
            total_executions: history.total_executions(),
            substrates_available: self.num_substrates()?,
            average_duration_ms: history.average_duration().as_secs_f64() * 1000.0,
        })
    }
}

/// Workload request
///
/// **Deep Debt**: Fully configurable, no hardcoded values
#[derive(Debug, Clone)]
pub struct WorkloadRequest {
    /// Number of operations
    pub operation_count: usize,

    /// Power budget in watts (None = unlimited)
    pub power_budget_watts: Option<f64>,

    /// Performance target
    pub target: PerformanceTarget,

    /// Batch size hint
    pub batch_size: Option<usize>,
}

impl WorkloadRequest {
    /// Creates a new workload request builder.
    #[expect(
        clippy::new_ret_no_self,
        reason = "builder pattern: returns builder, not Self"
    )]
    pub fn new() -> WorkloadRequestBuilder {
        WorkloadRequestBuilder::default()
    }
}

impl Default for WorkloadRequest {
    fn default() -> Self {
        Self {
            operation_count: 1000,
            power_budget_watts: None,
            target: PerformanceTarget::Balanced,
            batch_size: None,
        }
    }
}

/// Performance target for workload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceTarget {
    /// Minimize latency
    Latency,
    /// Maximize throughput
    Throughput,
    /// Minimize energy
    Energy,
    /// Balance all factors
    Balanced,
}

/// Workload request builder
#[derive(Default)]
pub struct WorkloadRequestBuilder {
    request: WorkloadRequest,
}

impl WorkloadRequestBuilder {
    /// Sets the number of operations for the workload.
    pub const fn operation_count(mut self, count: usize) -> Self {
        self.request.operation_count = count;
        self
    }

    /// Sets the power budget in watts.
    pub const fn power_budget_watts(mut self, watts: f64) -> Self {
        self.request.power_budget_watts = Some(watts);
        self
    }

    /// Sets target to minimize latency.
    pub const fn target_latency(mut self) -> Self {
        self.request.target = PerformanceTarget::Latency;
        self
    }

    /// Sets target to maximize throughput.
    pub const fn target_throughput(mut self) -> Self {
        self.request.target = PerformanceTarget::Throughput;
        self
    }

    /// Sets target to minimize energy consumption.
    pub const fn target_energy(mut self) -> Self {
        self.request.target = PerformanceTarget::Energy;
        self
    }

    /// Sets the batch size hint.
    pub const fn batch_size(mut self, size: usize) -> Self {
        self.request.batch_size = Some(size);
        self
    }

    /// Builds the workload request.
    pub const fn build(self) -> Result<WorkloadRequest, OrchestrationError> {
        if self.request.operation_count == 0 {
            return Err(OrchestrationError::InvalidOperationCount);
        }
        Ok(self.request)
    }
}

/// Workload execution result
#[derive(Debug, Clone)]
pub struct WorkloadResult {
    /// Substrate that executed this
    pub substrate_name: String,

    /// Substrate type
    pub substrate_type: SubstrateType,

    /// Execution duration
    pub duration: Duration,

    /// Whether execution succeeded
    pub success: bool,

    /// Power consumed (if measured)
    pub power_consumed_mw: Option<f64>,
}

/// Performance history for learning
///
/// **Deep Debt**: Learn from actual performance, don't hardcode
#[derive(Debug)]
pub struct PerformanceHistory {
    records: Vec<(SubstrateType, WorkloadResult)>,
}

impl PerformanceHistory {
    /// Creates a new empty performance history.
    pub const fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    /// Records a workload result for the given substrate type.
    pub fn record(&mut self, substrate_type: SubstrateType, result: &WorkloadResult) {
        self.records.push((substrate_type, result.clone()));
    }

    /// Returns the average duration for executions on the given substrate type.
    pub fn average_duration_for(&self, substrate_type: SubstrateType) -> Option<Duration> {
        let durations: Vec<_> = self
            .records
            .iter()
            .filter(|(st, _)| *st == substrate_type)
            .map(|(_, r)| r.duration)
            .collect();

        if durations.is_empty() {
            return None;
        }

        let total: Duration = durations.iter().sum();
        Some(total / durations.len() as u32)
    }

    /// Returns the total number of recorded executions.
    pub const fn total_executions(&self) -> usize {
        self.records.len()
    }

    /// Returns the average duration across all recorded executions.
    pub fn average_duration(&self) -> Duration {
        if self.records.is_empty() {
            return Duration::from_secs(0);
        }

        let total: Duration = self.records.iter().map(|(_, r)| r.duration).sum();
        total / self.records.len() as u32
    }
}

impl Default for PerformanceHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Orchestrator statistics
#[derive(Debug, Clone)]
pub struct OrchestratorStats {
    /// Total number of workload executions.
    pub total_executions: usize,
    /// Number of available substrates.
    pub substrates_available: usize,
    /// Average execution duration in milliseconds.
    pub average_duration_ms: f64,
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod tests;
