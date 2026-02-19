//! Performance Management and Optimization for `ToadStool`
//!
//! This crate provides comprehensive performance management including:
//! - Runtime selection algorithms with intelligent workload routing
//! - Performance profiling and metrics collection
//! - Resource pool management and optimization
//! - Usage prediction and recommendation engines
//!
//! ## Architecture
//!
//! The crate is organized into focused modules:
//! - `types`: Core types, configuration, and data structures
//! - `optimizer`: Trait definition for performance optimization
//! - `scoring`: Performance and efficiency scoring algorithms
//! - `implementation`: Main optimizer implementation (to be extracted)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use toadstool_management_performance::{
//!     IntelligentPerformanceOptimizer,
//!     PerformanceConfig,
//!     RuntimeSelectionStrategy,
//! };
//!
//! let config = PerformanceConfig::default();
//! let strategy = RuntimeSelectionStrategy::FastestExecution;
//! let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);
//! ```

// Re-export main types and traits
pub use types::{
    OptimizationRecommendation, PerformanceConfig, PerformanceMetrics, RecommendationType,
    ResourcePrediction, RuntimeSelectionStrategy, RuntimeStats, SelectionWeights,
};

pub use optimizer::PerformanceOptimizer;

pub use scoring::{calculate_efficiency_score, calculate_performance_score};

// Public modules
pub mod optimizer;
pub mod scoring;
pub mod types;

// Implementation details (to be refactored into separate modules)
mod implementation {
    use std::collections::{HashMap, VecDeque};
    use std::sync::Arc;
    use std::time::Duration;

    use async_trait::async_trait;
    use chrono::Utc;
    use tokio::sync::RwLock;
    use tracing::{debug, info};

    use toadstool::error::{ToadStoolError, ToadStoolResult};
    use toadstool::execution::{ExecutionRequest, RuntimeType};
    use toadstool::workload::WorkloadSpec;

    use super::optimizer::PerformanceOptimizer;
    use super::scoring;
    use super::types::*;

    /// Intelligent performance optimizer implementation
    pub struct IntelligentPerformanceOptimizer {
        config: PerformanceConfig,
        metrics_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
        runtime_stats: Arc<RwLock<HashMap<RuntimeType, RuntimeStats>>>,
        _runtime_metrics: Arc<RwLock<HashMap<RuntimeType, PerformanceMetrics>>>,
        _baseline_measurements: Arc<RwLock<HashMap<String, BaselineMetrics>>>,
        _runtime_selector: Arc<RwLock<RuntimeSelector>>,
        #[allow(dead_code)]
        prediction_models: Arc<RwLock<HashMap<String, PredictionModel>>>,
        selection_strategy: RuntimeSelectionStrategy,
    }

    // Internal types (will be moved to separate modules)
    #[derive(Clone)]
    struct BaselineMetrics {
        _avg_execution_time: Duration,
        _avg_memory_mb: f64,
        _avg_cpu_percent: f64,
    }

    #[derive(Default)]
    struct RuntimeSelector {
        _last_selection: Option<RuntimeType>,
    }

    struct PredictionModel {
        _model_type: String,
        _confidence: f64,
    }

    impl IntelligentPerformanceOptimizer {
        /// Create new intelligent performance optimizer
        pub fn new(config: PerformanceConfig, strategy: RuntimeSelectionStrategy) -> Self {
            info!("Creating intelligent performance optimizer");

            Self {
                config,
                metrics_history: Arc::new(RwLock::new(VecDeque::new())),
                runtime_stats: Arc::new(RwLock::new(HashMap::new())),
                _runtime_metrics: Arc::new(RwLock::new(HashMap::new())),
                _baseline_measurements: Arc::new(RwLock::new(HashMap::new())),
                _runtime_selector: Arc::new(RwLock::new(RuntimeSelector::default())),
                prediction_models: Arc::new(RwLock::new(HashMap::new())),
                selection_strategy: strategy,
            }
        }

        /// Cleanup old metrics based on retention policy
        async fn cleanup_old_metrics(&self) {
            let retention_duration =
                Duration::from_secs(self.config.history_retention_hours * 3600);
            let cutoff_time =
                Utc::now() - chrono::Duration::from_std(retention_duration).unwrap_or_default();

            let mut history = self.metrics_history.write().await;
            while let Some(front) = history.front() {
                if front.start_time < cutoff_time {
                    history.pop_front();
                } else {
                    break;
                }
            }
        }

        /// Update runtime statistics
        async fn update_runtime_stats(&self, metrics: &PerformanceMetrics) {
            let mut stats = self.runtime_stats.write().await;

            let runtime_stats = stats
                .entry(metrics.runtime_type.clone())
                .or_insert_with(|| RuntimeStats {
                    runtime_type: metrics.runtime_type.clone(),
                    total_executions: 0,
                    successful_executions: 0,
                    avg_execution_time: Duration::ZERO,
                    p95_execution_time: Duration::ZERO,
                    avg_memory_usage: 0.0,
                    avg_cpu_usage: 0.0,
                    success_rate: 0.0,
                    efficiency_score: 0.0,
                    current_load: 0.0,
                });

            runtime_stats.total_executions += 1;
            if metrics.success {
                runtime_stats.successful_executions += 1;
            }

            runtime_stats.success_rate = if runtime_stats.total_executions > 0 {
                (runtime_stats.successful_executions as f64 / runtime_stats.total_executions as f64)
                    * 100.0
            } else {
                0.0
            };

            if let Some(duration) = metrics.execution_duration {
                runtime_stats.avg_execution_time = Duration::from_secs_f64(
                    (runtime_stats.avg_execution_time.as_secs_f64()
                        * (runtime_stats.total_executions - 1) as f64
                        + duration.as_secs_f64())
                        / runtime_stats.total_executions as f64,
                );
            }

            runtime_stats.avg_memory_usage = (runtime_stats.avg_memory_usage
                * (runtime_stats.total_executions - 1) as f64
                + metrics.resource_metrics.memory.used_bytes as f64 / 1024.0 / 1024.0)
                / runtime_stats.total_executions as f64;

            runtime_stats.avg_cpu_usage = (runtime_stats.avg_cpu_usage
                * (runtime_stats.total_executions - 1) as f64
                + metrics.resource_metrics.cpu.usage_percent)
                / runtime_stats.total_executions as f64;

            runtime_stats.efficiency_score = metrics.efficiency_score;
        }

        /// Select runtime by configured strategy
        async fn select_runtime_by_strategy(
            &self,
            request: &ExecutionRequest,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            match &self.selection_strategy {
                RuntimeSelectionStrategy::FastestExecution => {
                    self.select_fastest_runtime(available_runtimes).await
                }
                RuntimeSelectionStrategy::LowestResourceUsage => {
                    self.select_lowest_resource_runtime(available_runtimes)
                        .await
                }
                RuntimeSelectionStrategy::BestEfficiency => {
                    self.select_most_efficient_runtime(available_runtimes).await
                }
                RuntimeSelectionStrategy::LoadBalance => {
                    self.select_least_loaded_runtime(available_runtimes).await
                }
                RuntimeSelectionStrategy::WorkloadOptimized => {
                    self.select_workload_optimized_runtime(request, available_runtimes)
                        .await
                }
                RuntimeSelectionStrategy::Custom { weights } => {
                    self.select_custom_weighted_runtime(available_runtimes, weights)
                        .await
                }
            }
        }

        /// Select fastest runtime based on historical data
        async fn select_fastest_runtime(
            &self,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            let stats = self.runtime_stats.read().await;

            let fastest = available_runtimes
                .iter()
                .filter_map(|rt| {
                    stats
                        .get(rt)
                        .filter(|s| s.total_executions > 0)
                        .map(|s| (rt, s.avg_execution_time))
                })
                .min_by_key(|(_, time)| *time)
                .map(|(rt, _)| rt.clone());

            Ok(fastest.unwrap_or_else(|| available_runtimes[0].clone()))
        }

        /// Select runtime with lowest resource usage
        async fn select_lowest_resource_runtime(
            &self,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            let stats = self.runtime_stats.read().await;

            let lowest = available_runtimes
                .iter()
                .filter_map(|rt| {
                    stats.get(rt).filter(|s| s.total_executions > 0).map(|s| {
                        let resource_score = s.avg_memory_usage + s.avg_cpu_usage;
                        (rt, resource_score)
                    })
                })
                .min_by(|(_, score1), (_, score2)| {
                    score1
                        .partial_cmp(score2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(rt, _)| rt.clone());

            Ok(lowest.unwrap_or_else(|| available_runtimes[0].clone()))
        }

        /// Select most efficient runtime
        async fn select_most_efficient_runtime(
            &self,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            let stats = self.runtime_stats.read().await;

            let most_efficient = available_runtimes
                .iter()
                .filter_map(|rt| {
                    stats
                        .get(rt)
                        .filter(|s| s.total_executions > 0)
                        .map(|s| (rt, s.efficiency_score))
                })
                .max_by(|(_, score1), (_, score2)| {
                    score1
                        .partial_cmp(score2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(rt, _)| rt.clone());

            Ok(most_efficient.unwrap_or_else(|| available_runtimes[0].clone()))
        }

        /// Select least loaded runtime
        async fn select_least_loaded_runtime(
            &self,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            let stats = self.runtime_stats.read().await;

            let least_loaded = available_runtimes
                .iter()
                .filter_map(|rt| stats.get(rt).map(|s| (rt, s.current_load)))
                .min_by(|(_, load1), (_, load2)| {
                    load1
                        .partial_cmp(load2)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(rt, _)| rt.clone());

            Ok(least_loaded.unwrap_or_else(|| available_runtimes[0].clone()))
        }

        /// Select runtime optimized for workload type
        async fn select_workload_optimized_runtime(
            &self,
            request: &ExecutionRequest,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            match &request.workload {
                WorkloadSpec::Native { .. } => {
                    if available_runtimes.contains(&RuntimeType::Native) {
                        Ok(RuntimeType::Native)
                    } else {
                        self.select_fastest_runtime(available_runtimes).await
                    }
                }
                WorkloadSpec::Wasm { .. } => {
                    if available_runtimes.contains(&RuntimeType::Wasm) {
                        Ok(RuntimeType::Wasm)
                    } else {
                        self.select_fastest_runtime(available_runtimes).await
                    }
                }
                WorkloadSpec::Container { .. } => {
                    if available_runtimes.contains(&RuntimeType::Container) {
                        Ok(RuntimeType::Container)
                    } else {
                        self.select_fastest_runtime(available_runtimes).await
                    }
                }
                WorkloadSpec::Gpu { .. } => {
                    if available_runtimes.contains(&RuntimeType::Gpu) {
                        Ok(RuntimeType::Gpu)
                    } else {
                        self.select_fastest_runtime(available_runtimes).await
                    }
                }
                WorkloadSpec::Python { .. } => {
                    if available_runtimes.contains(&RuntimeType::Python) {
                        Ok(RuntimeType::Python)
                    } else {
                        self.select_fastest_runtime(available_runtimes).await
                    }
                }
                _ => self.select_fastest_runtime(available_runtimes).await,
            }
        }

        /// Select runtime using custom weights
        async fn select_custom_weighted_runtime(
            &self,
            available_runtimes: &[RuntimeType],
            weights: &SelectionWeights,
        ) -> ToadStoolResult<RuntimeType> {
            let stats = self.runtime_stats.read().await;

            let mut best_runtime = available_runtimes[0].clone();
            let mut best_score = f64::MIN;

            for runtime in available_runtimes {
                if let Some(runtime_stats) = stats.get(runtime) {
                    if runtime_stats.total_executions == 0 {
                        continue;
                    }

                    let weighted_score = scoring::calculate_weighted_score(
                        runtime_stats.avg_execution_time,
                        runtime_stats.avg_memory_usage,
                        runtime_stats.avg_cpu_usage,
                        runtime_stats.current_load,
                        runtime_stats.success_rate,
                        weights,
                    );

                    if weighted_score > best_score {
                        best_score = weighted_score;
                        best_runtime = runtime.clone();
                    }
                }
            }

            Ok(best_runtime)
        }
    }

    #[async_trait]
    impl PerformanceOptimizer for IntelligentPerformanceOptimizer {
        async fn select_runtime(
            &self,
            request: &ExecutionRequest,
            available_runtimes: &[RuntimeType],
        ) -> ToadStoolResult<RuntimeType> {
            debug!("Selecting optimal runtime for execution request");

            if !self.config.enable_runtime_selection {
                return Ok(available_runtimes[0].clone());
            }

            self.select_runtime_by_strategy(request, available_runtimes)
                .await
        }

        async fn record_metrics(&self, mut metrics: PerformanceMetrics) -> ToadStoolResult<()> {
            debug!(
                "Recording performance metrics for execution: {}",
                metrics.execution_id
            );

            // Calculate scores using scoring module
            if let Some(duration) = metrics.execution_duration {
                metrics.performance_score =
                    scoring::calculate_performance_score(&metrics.resource_metrics, duration);
                metrics.efficiency_score =
                    scoring::calculate_efficiency_score(&metrics.resource_metrics, duration);
            }

            // Update runtime statistics
            self.update_runtime_stats(&metrics).await;

            // Store metrics
            {
                let mut history = self.metrics_history.write().await;
                history.push_back(metrics);
            }

            // Cleanup old metrics
            self.cleanup_old_metrics().await;

            Ok(())
        }

        async fn get_runtime_stats(
            &self,
            runtime_type: RuntimeType,
        ) -> ToadStoolResult<RuntimeStats> {
            let stats = self.runtime_stats.read().await;
            stats.get(&runtime_type).cloned().ok_or_else(|| {
                ToadStoolError::runtime(format!(
                    "No statistics available for runtime: {runtime_type:?}"
                ))
            })
        }

        async fn predict_resources(
            &self,
            _workload: &WorkloadSpec,
        ) -> ToadStoolResult<ResourcePrediction> {
            // Simplified prediction for now
            Ok(ResourcePrediction {
                timestamp: Utc::now(),
                execution_time: Duration::from_secs(10),
                memory_mb: 256.0,
                cpu_percent: 50.0,
                confidence: 70.0,
                model_type: "historical_average".to_string(),
            })
        }

        async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>> {
            // Placeholder for recommendations
            Ok(vec![])
        }

        async fn update_model(&self) -> ToadStoolResult<()> {
            // Placeholder for model updates
            Ok(())
        }
    }
}

// Re-export implementation
pub use implementation::IntelligentPerformanceOptimizer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_optimizer_fastest() {
        let config = PerformanceConfig::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::FastestExecution,
        );
        // Just ensure construction succeeds
        let _ = opt;
    }

    #[test]
    fn test_create_optimizer_load_balance() {
        let config = PerformanceConfig::default();
        let opt =
            IntelligentPerformanceOptimizer::new(config, RuntimeSelectionStrategy::LoadBalance);
        let _ = opt;
    }

    #[test]
    fn test_create_optimizer_custom_weights() {
        let config = PerformanceConfig::default();
        let weights = SelectionWeights::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::Custom { weights },
        );
        let _ = opt;
    }

    #[tokio::test]
    async fn test_predict_resources_returns_ok() {
        use std::path::PathBuf;
        use toadstool::workload::{ExecutableSource, WorkloadSpec};

        let config = PerformanceConfig::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::FastestExecution,
        );
        let workload = WorkloadSpec::Native {
            executable: ExecutableSource::File {
                path: PathBuf::from("/usr/bin/true"),
            },
            args: None,
            working_dir: None,
            env_vars: Default::default(),
            user: None,
        };
        let pred = opt.predict_resources(&workload).await.unwrap();
        assert!(pred.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_get_recommendations_returns_empty() {
        let config = PerformanceConfig::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::FastestExecution,
        );
        let recs = opt.get_recommendations().await.unwrap();
        assert!(recs.is_empty());
    }

    #[tokio::test]
    async fn test_update_model_returns_ok() {
        let config = PerformanceConfig::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::FastestExecution,
        );
        opt.update_model().await.unwrap();
    }
}
