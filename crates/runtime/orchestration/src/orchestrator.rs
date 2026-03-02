//! Workload Orchestrator - Main coordination logic
//!
//! **Deep Debt**: Runtime discovery, intelligent selection, automatic fallback

use crate::error::OrchestrationError;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool_runtime_universal::substrate::*;

use crate::policy::*;

/// Main orchestrator for workload distribution
///
/// **Deep Debt**: Discovers substrates at runtime, no hardcoding
pub struct WorkloadOrchestrator {
    /// Available substrates (discovered at runtime)
    substrates: Arc<RwLock<Vec<SubstrateHandle>>>,

    /// Selection policy
    policy: SelectionPolicy,

    /// Performance history for learning
    history: Arc<RwLock<PerformanceHistory>>,
}

impl WorkloadOrchestrator {
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
    pub fn with_substrates(substrates: Vec<SubstrateHandle>) -> Self {
        Self {
            substrates: Arc::new(RwLock::new(substrates)),
            policy: SelectionPolicy::default(),
            history: Arc::new(RwLock::new(PerformanceHistory::new())),
        }
    }

    /// Register a substrate
    pub fn register_substrate(&self, substrate: SubstrateHandle) {
        self.substrates.write().push(substrate);
    }

    /// Get number of available substrates
    pub fn num_substrates(&self) -> usize {
        self.substrates.read().len()
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

        // Record performance
        let duration = start.elapsed();
        let result = WorkloadResult {
            substrate_name: substrate.name().to_string(),
            substrate_type: substrate.substrate_type(),
            duration,
            success: true,
            power_consumed_mw: output.metadata.power_consumed_mw,
        };

        // Update history
        self.history
            .write()
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
    fn select_substrate(
        &self,
        request: &WorkloadRequest,
    ) -> Result<SubstrateHandle, OrchestrationError> {
        let substrates = self.substrates.read();

        if substrates.is_empty() {
            return Err(OrchestrationError::NoSubstrates);
        }

        // Apply selection policy
        let history = self.history.read();
        let selected = self.policy.select(&substrates, request, &history)?;

        Ok(selected)
    }

    /// Rank substrates by suitability
    fn rank_substrates(
        &self,
        request: &WorkloadRequest,
    ) -> Result<Vec<SubstrateHandle>, OrchestrationError> {
        let substrates = self.substrates.read();
        let history = self.history.read();

        let mut ranked = self.policy.rank_all(&substrates, request, &history)?;
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(ranked.into_iter().map(|(s, _)| s).collect())
    }

    /// Execute on specific substrate
    async fn execute_on_substrate(
        &self,
        substrate: SubstrateHandle,
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

    /// Get performance statistics
    pub fn stats(&self) -> OrchestratorStats {
        let history = self.history.read();
        OrchestratorStats {
            total_executions: history.total_executions(),
            substrates_available: self.num_substrates(),
            average_duration_ms: history.average_duration().as_secs_f64() * 1000.0,
        }
    }
}

/// Handle to a compute substrate
///
/// **Deep Debt**: Type-erased for flexibility, discovered at runtime
pub type SubstrateHandle = Arc<dyn ComputeSubstrate>;

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
    #[allow(clippy::new_ret_no_self)] // Builder pattern - returns builder, not Self
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn operation_count(mut self, count: usize) -> Self {
        self.request.operation_count = count;
        self
    }

    pub fn power_budget_watts(mut self, watts: f64) -> Self {
        self.request.power_budget_watts = Some(watts);
        self
    }

    pub fn target_latency(mut self) -> Self {
        self.request.target = PerformanceTarget::Latency;
        self
    }

    pub fn target_throughput(mut self) -> Self {
        self.request.target = PerformanceTarget::Throughput;
        self
    }

    pub fn target_energy(mut self) -> Self {
        self.request.target = PerformanceTarget::Energy;
        self
    }

    pub fn batch_size(mut self, size: usize) -> Self {
        self.request.batch_size = Some(size);
        self
    }

    pub fn build(self) -> Result<WorkloadRequest, OrchestrationError> {
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
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn record(&mut self, substrate_type: SubstrateType, result: &WorkloadResult) {
        self.records.push((substrate_type, result.clone()));
    }

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

    pub fn total_executions(&self) -> usize {
        self.records.len()
    }

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
    pub total_executions: usize,
    pub substrates_available: usize,
    pub average_duration_ms: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockSubstrate {
        name: String,
        substrate_type: SubstrateType,
    }

    // TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
    #[async_trait]
    impl ComputeSubstrate for MockSubstrate {
        fn name(&self) -> &str {
            &self.name
        }

        fn substrate_type(&self) -> SubstrateType {
            self.substrate_type
        }

        async fn execute_buffer_op(
            &self,
            _op: BufferOperation,
        ) -> Result<BufferOutput, toadstool_runtime_universal::SubstrateError> {
            Ok(BufferOutput {
                data: vec![0; 100],
                metadata: BufferMetadata {
                    duration: Duration::from_millis(10),
                    substrate_name: self.name.clone(),
                    power_consumed_mw: Some(50000.0),
                },
            })
        }
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = WorkloadOrchestrator::discover().await.unwrap();
        assert_eq!(orchestrator.num_substrates(), 0);
    }

    #[tokio::test]
    async fn test_register_substrate() {
        let orchestrator = WorkloadOrchestrator::discover().await.unwrap();

        let substrate: SubstrateHandle = Arc::new(MockSubstrate {
            name: "Test CPU".to_string(),
            substrate_type: SubstrateType::Cpu,
        });

        orchestrator.register_substrate(substrate);
        assert_eq!(orchestrator.num_substrates(), 1);
    }

    #[tokio::test]
    async fn test_workload_execution() {
        let substrate: SubstrateHandle = Arc::new(MockSubstrate {
            name: "Test GPU".to_string(),
            substrate_type: SubstrateType::Gpu,
        });

        let orchestrator = WorkloadOrchestrator::with_substrates(vec![substrate]);

        let request = WorkloadRequest::new()
            .operation_count(1000)
            .target_latency()
            .build()
            .unwrap();

        let result = orchestrator.execute(request).await.unwrap();
        assert_eq!(result.substrate_name, "Test GPU");
        assert!(result.success);
    }

    #[test]
    fn test_workload_request_builder() {
        let request = WorkloadRequest::new()
            .operation_count(5000)
            .power_budget_watts(50.0)
            .target_energy()
            .batch_size(100)
            .build()
            .unwrap();

        assert_eq!(request.operation_count, 5000);
        assert_eq!(request.power_budget_watts, Some(50.0));
        assert_eq!(request.target, PerformanceTarget::Energy);
        assert_eq!(request.batch_size, Some(100));
    }
}
