// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intelligent performance optimizer implementation.
//!
//! Main optimizer struct and `PerformanceOptimizer` trait implementation.
//! Delegates to domain modules: selection, recommendations, statistics.

mod internal;
mod recommendations;
mod selection;
mod statistics;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use std::sync::RwLock;
use tracing::debug;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionRequest, RuntimeType};
use toadstool::workload::WorkloadSpec;

use crate::optimizer::PerformanceOptimizer;
use crate::scoring;
use crate::types::{
    OptimizationRecommendation, PerformanceConfig, PerformanceMetrics, ResourcePrediction,
    RuntimeSelectionStrategy, RuntimeStats,
};

use internal::{
    BaselineMetrics, PredictionModel, RuntimeSelector, update_prediction_models_from_history,
};
use recommendations::generate_recommendations;
use selection::select_runtime_by_strategy;
use statistics::{cleanup_old_metrics, update_model_from_history, update_runtime_stats};

/// Intelligent performance optimizer implementation.
pub struct IntelligentPerformanceOptimizer {
    config: PerformanceConfig,
    metrics_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
    runtime_stats: Arc<RwLock<HashMap<RuntimeType, RuntimeStats>>>,
    _runtime_metrics: Arc<RwLock<HashMap<RuntimeType, PerformanceMetrics>>>,
    baseline_measurements: Arc<RwLock<HashMap<String, BaselineMetrics>>>,
    _runtime_selector: Arc<RwLock<RuntimeSelector>>,
    prediction_models: Arc<RwLock<HashMap<String, PredictionModel>>>,
    selection_strategy: RuntimeSelectionStrategy,
}

impl IntelligentPerformanceOptimizer {
    /// Create new intelligent performance optimizer.
    pub fn new(config: PerformanceConfig, strategy: RuntimeSelectionStrategy) -> Self {
        tracing::info!("Creating intelligent performance optimizer");

        Self {
            config,
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            runtime_stats: Arc::new(RwLock::new(HashMap::new())),
            _runtime_metrics: Arc::new(RwLock::new(HashMap::new())),
            baseline_measurements: Arc::new(RwLock::new(HashMap::new())),
            _runtime_selector: Arc::new(RwLock::new(RuntimeSelector::default())),
            prediction_models: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy: strategy,
        }
    }
}

impl PerformanceOptimizer for IntelligentPerformanceOptimizer {
    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // guard must outlive select_runtime_by_strategy call
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType> {
        debug!("Selecting optimal runtime for execution request");

        if !self.config.enable_runtime_selection {
            return Ok(available_runtimes[0].clone());
        }

        let stats = self
            .runtime_stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let selected = select_runtime_by_strategy(
            &stats,
            &self.selection_strategy,
            request,
            available_runtimes,
        );
        Ok(selected)
    }

    async fn record_metrics(&self, mut metrics: PerformanceMetrics) -> ToadStoolResult<()> {
        debug!(
            "Recording performance metrics for execution: {}",
            metrics.execution_id
        );

        if let Some(duration) = metrics.execution_duration {
            metrics.performance_score =
                scoring::calculate_performance_score(&metrics.resource_metrics, duration);
            metrics.efficiency_score =
                scoring::calculate_efficiency_score(&metrics.resource_metrics, duration);
        }

        {
            let mut stats = self
                .runtime_stats
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            update_runtime_stats(&mut stats, &metrics);
        }

        {
            let mut history = self
                .metrics_history
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            history.push_back(metrics);
            cleanup_old_metrics(&mut history, self.config.history_retention_hours);
            drop(history);
        }

        Ok(())
    }

    async fn get_runtime_stats(&self, runtime_type: &RuntimeType) -> ToadStoolResult<RuntimeStats> {
        let stats = self
            .runtime_stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        stats.get(runtime_type).cloned().ok_or_else(|| {
            ToadStoolError::runtime(format!(
                "No statistics available for runtime: {runtime_type:?}"
            ))
        })
    }

    async fn predict_resources(
        &self,
        _workload: &WorkloadSpec,
    ) -> ToadStoolResult<ResourcePrediction> {
        const MIN_EXECUTION_TIME: Duration = Duration::from_secs(1);
        const DEFAULT_PREDICTION_EXECUTION_SECS: u64 = 10;

        let models = self
            .prediction_models
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, model)) = models.iter().max_by_key(|(_, m)| m.sample_count())
            && model.sample_count() > 0
        {
            return Ok(model.predict());
        }
        drop(models);

        let stats = self
            .runtime_stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((_, runtime_stats)) = stats.iter().next() {
            return Ok(ResourcePrediction {
                timestamp: SystemTime::now(),
                execution_time: runtime_stats.avg_execution_time.max(MIN_EXECUTION_TIME),
                memory_mb: runtime_stats.avg_memory_usage.max(1.0),
                cpu_percent: runtime_stats.avg_cpu_usage.clamp(0.0, 100.0),
                confidence: 50.0,
                model_type: "runtime_stats_fallback".to_string(),
            });
        }
        drop(stats);

        Ok(ResourcePrediction {
            timestamp: SystemTime::now(),
            execution_time: Duration::from_secs(DEFAULT_PREDICTION_EXECUTION_SECS),
            memory_mb: 256.0,
            cpu_percent: 50.0,
            confidence: 20.0,
            model_type: "default_no_data".to_string(),
        })
    }

    async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>> {
        let stats = self
            .runtime_stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Ok(generate_recommendations(&self.config, &stats))
    }

    async fn update_model(&self) -> ToadStoolResult<()> {
        let history = self
            .metrics_history
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .runtime_stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut baselines = self
            .baseline_measurements
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut models = self
            .prediction_models
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        update_model_from_history(
            &history,
            &mut stats,
            &mut baselines,
            self.config.min_prediction_samples,
        );
        update_prediction_models_from_history(
            &history,
            &mut models,
            self.config.min_prediction_samples,
        );

        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
