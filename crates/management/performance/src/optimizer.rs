//! Performance optimizer trait and core interfaces

use async_trait::async_trait;

use toadstool::error::ToadStoolResult;
use toadstool::execution::{ExecutionRequest, RuntimeType};
use toadstool::workload::WorkloadSpec;

use crate::types::{
    OptimizationRecommendation, PerformanceMetrics, ResourcePrediction, RuntimeStats,
};

/// Performance optimization engine trait
///
/// This trait defines the interface for intelligent performance optimization,
/// including runtime selection, metrics collection, resource prediction,
/// and recommendation generation.
#[async_trait]
pub trait PerformanceOptimizer: Send + Sync {
    /// Select optimal runtime for execution request
    ///
    /// Analyzes the execution request and available runtimes to select
    /// the best runtime based on historical performance data and current
    /// system state.
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType>;

    /// Record execution performance metrics
    ///
    /// Stores performance metrics for completed executions to build
    /// historical data for future optimization decisions.
    async fn record_metrics(&self, metrics: PerformanceMetrics) -> ToadStoolResult<()>;

    /// Get runtime performance statistics
    ///
    /// Returns aggregated statistics for a specific runtime type,
    /// including success rates, average execution times, and resource usage.
    async fn get_runtime_stats(&self, runtime_type: RuntimeType) -> ToadStoolResult<RuntimeStats>;

    /// Predict resource requirements for workload
    ///
    /// Uses machine learning models and historical data to predict
    /// the resource requirements for a given workload.
    async fn predict_resources(
        &self,
        workload: &WorkloadSpec,
    ) -> ToadStoolResult<ResourcePrediction>;

    /// Generate optimization recommendations
    ///
    /// Analyzes system performance and generates actionable recommendations
    /// for improving performance and resource utilization.
    async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>>;

    /// Update performance model with new data
    ///
    /// Retrains or updates the internal performance models with
    /// accumulated metrics data.
    async fn update_model(&self) -> ToadStoolResult<()>;
}
