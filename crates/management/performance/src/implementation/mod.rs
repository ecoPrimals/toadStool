// SPDX-License-Identifier: AGPL-3.0-only
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

use tokio::sync::RwLock;
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
    #[allow(clippy::significant_drop_tightening)] // guard must outlive select_runtime_by_strategy call
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType> {
        debug!("Selecting optimal runtime for execution request");

        if !self.config.enable_runtime_selection {
            return Ok(available_runtimes[0].clone());
        }

        let stats = self.runtime_stats.read().await;
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
            let mut stats = self.runtime_stats.write().await;
            update_runtime_stats(&mut stats, &metrics);
        }

        {
            let mut history = self.metrics_history.write().await;
            history.push_back(metrics);
            cleanup_old_metrics(&mut history, self.config.history_retention_hours);
            drop(history);
        }

        Ok(())
    }

    async fn get_runtime_stats(&self, runtime_type: &RuntimeType) -> ToadStoolResult<RuntimeStats> {
        let stats = self.runtime_stats.read().await;
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
        let models = self.prediction_models.read().await;
        if let Some((_, model)) = models.iter().max_by_key(|(_, m)| m.sample_count())
            && model.sample_count() > 0
        {
            return Ok(model.predict());
        }
        drop(models);

        let stats = self.runtime_stats.read().await;
        if let Some((_, runtime_stats)) = stats.iter().next() {
            return Ok(ResourcePrediction {
                timestamp: SystemTime::now(),
                execution_time: runtime_stats.avg_execution_time.max(Duration::from_secs(1)),
                memory_mb: runtime_stats.avg_memory_usage.max(1.0),
                cpu_percent: runtime_stats.avg_cpu_usage.clamp(0.0, 100.0),
                confidence: 50.0,
                model_type: "runtime_stats_fallback".to_string(),
            });
        }
        drop(stats);

        Ok(ResourcePrediction {
            timestamp: SystemTime::now(),
            execution_time: Duration::from_secs(10),
            memory_mb: 256.0,
            cpu_percent: 50.0,
            confidence: 20.0,
            model_type: "default_no_data".to_string(),
        })
    }

    async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>> {
        let stats = self.runtime_stats.read().await;
        Ok(generate_recommendations(&self.config, &stats))
    }

    async fn update_model(&self) -> ToadStoolResult<()> {
        let history = self.metrics_history.read().await;
        let mut stats = self.runtime_stats.write().await;
        let mut baselines = self.baseline_measurements.write().await;
        let mut models = self.prediction_models.write().await;

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
mod tests {
    use super::statistics::{cleanup_old_metrics, update_model_from_history, update_runtime_stats};
    use super::*;
    use crate::SelectionWeights;
    use std::collections::VecDeque;
    use std::time::Duration;
    use toadstool::resources::{CpuMetrics, MemoryMetrics, RuntimeMetrics, StorageMetrics};
    use toadstool::resources::{NetworkMetrics, TimingMetrics};

    fn make_performance_metrics(
        execution_id: &str,
        runtime_type: &RuntimeType,
        start_time: std::time::SystemTime,
        execution_duration_secs: f64,
        memory_bytes: u64,
        cpu_percent: f64,
        success: bool,
    ) -> PerformanceMetrics {
        let duration = Duration::from_secs_f64(execution_duration_secs);
        PerformanceMetrics {
            execution_id: execution_id.to_string(),
            runtime_type: runtime_type.clone(),
            workload_type: "test".to_string(),
            start_time,
            end_time: Some(start_time + duration),
            execution_duration: Some(duration),
            resource_metrics: RuntimeMetrics {
                cpu: CpuMetrics {
                    usage_percent: cpu_percent,
                    cores_used: 1.0,
                    cpu_time_seconds: execution_duration_secs,
                },
                memory: MemoryMetrics {
                    usage_percent: 0.0,
                    used_bytes: memory_bytes,
                    peak_bytes: memory_bytes,
                },
                storage: StorageMetrics::default(),
                network: NetworkMetrics::default(),
                gpu: None,
                timing: TimingMetrics {
                    start_time,
                    end_time: Some(start_time + duration),
                    duration,
                },
            },
            success,
            error_message: None,
            performance_score: 80.0,
            efficiency_score: 75.0,
        }
    }

    #[test]
    fn test_cleanup_old_metrics_removes_old_entries() {
        let now = std::time::SystemTime::now();
        let old_time = now - Duration::from_secs(25 * 3600); // 25 hours ago
        let recent_time = now - Duration::from_secs(3600); // 1 hour ago

        let mut history = VecDeque::new();
        history.push_back(make_performance_metrics(
            "old1",
            &RuntimeType::Native,
            old_time,
            1.0,
            100 * 1024 * 1024,
            50.0,
            true,
        ));
        history.push_back(make_performance_metrics(
            "old2",
            &RuntimeType::Native,
            old_time,
            2.0,
            200 * 1024 * 1024,
            60.0,
            true,
        ));
        history.push_back(make_performance_metrics(
            "recent",
            &RuntimeType::Native,
            recent_time,
            0.5,
            50 * 1024 * 1024,
            30.0,
            true,
        ));

        cleanup_old_metrics(&mut history, 24);

        assert_eq!(history.len(), 1);
        assert_eq!(history.front().unwrap().execution_id, "recent");
    }

    #[test]
    fn test_cleanup_old_metrics_empty_history() {
        let mut history: VecDeque<PerformanceMetrics> = VecDeque::new();
        cleanup_old_metrics(&mut history, 24);
        assert!(history.is_empty());
    }

    #[test]
    fn test_cleanup_old_metrics_all_recent() {
        let now = std::time::SystemTime::now();
        let recent = now - Duration::from_secs(100);

        let mut history = VecDeque::new();
        history.push_back(make_performance_metrics(
            "r1",
            &RuntimeType::Native,
            recent,
            1.0,
            100 * 1024 * 1024,
            50.0,
            true,
        ));
        cleanup_old_metrics(&mut history, 24);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_update_runtime_stats_first_metric() {
        let mut stats = HashMap::new();
        let metrics = make_performance_metrics(
            "exec1",
            &RuntimeType::Wasm,
            std::time::SystemTime::now(),
            2.5,
            128 * 1024 * 1024,
            40.0,
            true,
        );

        update_runtime_stats(&mut stats, &metrics);

        let rs = stats
            .get(&RuntimeType::Wasm)
            .expect("should have Wasm stats");
        assert_eq!(rs.total_executions, 1);
        assert_eq!(rs.successful_executions, 1);
        assert!((rs.success_rate - 100.0).abs() < 1e-9);
        assert!((rs.avg_execution_time.as_secs_f64() - 2.5).abs() < 1e-6);
        assert!((rs.avg_memory_usage - 128.0).abs() < 1e-6);
        assert!((rs.avg_cpu_usage - 40.0).abs() < 1e-6);
    }

    #[test]
    fn test_update_runtime_stats_accumulates() {
        let mut stats = HashMap::new();
        let now = std::time::SystemTime::now();

        let m1 = make_performance_metrics(
            "e1",
            &RuntimeType::Native,
            now,
            1.0,
            100 * 1024 * 1024,
            30.0,
            true,
        );
        let m2 = make_performance_metrics(
            "e2",
            &RuntimeType::Native,
            now,
            3.0,
            200 * 1024 * 1024,
            70.0,
            false,
        );

        update_runtime_stats(&mut stats, &m1);
        update_runtime_stats(&mut stats, &m2);

        let rs = stats
            .get(&RuntimeType::Native)
            .expect("should have Native stats");
        assert_eq!(rs.total_executions, 2);
        assert_eq!(rs.successful_executions, 1);
        assert!((rs.success_rate - 50.0).abs() < 1e-9);
        assert!((rs.avg_execution_time.as_secs_f64() - 2.0).abs() < 1e-6); // (1+3)/2
        assert!((rs.avg_memory_usage - 150.0).abs() < 1e-6); // (100+200)/2
        assert!((rs.avg_cpu_usage - 50.0).abs() < 1e-6); // (30+70)/2
    }

    #[test]
    fn test_update_runtime_stats_no_duration() {
        let mut stats = HashMap::new();
        let mut metrics = make_performance_metrics(
            "e1",
            &RuntimeType::Native,
            std::time::SystemTime::now(),
            1.0,
            100 * 1024 * 1024,
            50.0,
            true,
        );
        metrics.execution_duration = None;

        update_runtime_stats(&mut stats, &metrics);

        let rs = stats.get(&RuntimeType::Native).expect("should have stats");
        assert_eq!(rs.total_executions, 1);
        assert_eq!(rs.avg_execution_time, Duration::ZERO);
    }

    #[test]
    fn test_update_model_from_history_insufficient_samples() {
        let mut stats = HashMap::new();
        let mut baselines = HashMap::new();
        let history: VecDeque<PerformanceMetrics> = VecDeque::new();
        update_model_from_history(&history, &mut stats, &mut baselines, 10);
        assert!(stats.is_empty());
        assert!(baselines.is_empty());
    }

    #[test]
    fn test_update_model_from_history_updates_p95_and_baselines() {
        let now = std::time::SystemTime::now();
        let mut history = VecDeque::new();
        for i in 0..12 {
            let dur = (f64::from(i) + 1.0) * 0.5; // 0.5, 1.0, 1.5, ..., 6.0
            history.push_back(make_performance_metrics(
                &format!("e{i}"),
                &RuntimeType::Native,
                now,
                dur,
                (100 + i as u64) * 1024 * 1024,
                30.0 + f64::from(i),
                true,
            ));
        }

        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            RuntimeStats {
                runtime_type: RuntimeType::Native,
                total_executions: 12,
                successful_executions: 12,
                avg_execution_time: Duration::from_secs(3),
                p95_execution_time: Duration::ZERO,
                avg_memory_usage: 150.0,
                avg_cpu_usage: 35.0,
                success_rate: 100.0,
                efficiency_score: 80.0,
                current_load: 0.0,
            },
        );
        let mut baselines = HashMap::new();

        update_model_from_history(&history, &mut stats, &mut baselines, 10);

        let rs = stats.get(&RuntimeType::Native).expect("should have stats");
        assert!(rs.p95_execution_time > Duration::ZERO);

        let bl = baselines.get("Native").expect("should have baseline");
        assert!(bl.avg_execution_time > Duration::ZERO);
        assert!(bl.avg_memory_mb > 0.0);
        assert!(bl.avg_cpu_percent > 0.0);
    }

    #[test]
    fn test_update_model_from_history_multiple_runtimes() {
        let now = std::time::SystemTime::now();
        let mut history = VecDeque::new();
        for i in 0..10 {
            history.push_back(make_performance_metrics(
                &format!("native_{i}"),
                &RuntimeType::Native,
                now,
                1.0 + f64::from(i) * 0.1,
                100 * 1024 * 1024,
                40.0,
                true,
            ));
        }
        for i in 0..10 {
            history.push_back(make_performance_metrics(
                &format!("wasm_{i}"),
                &RuntimeType::Wasm,
                now,
                0.5 + f64::from(i) * 0.05,
                64 * 1024 * 1024,
                25.0,
                true,
            ));
        }

        let mut stats = HashMap::new();
        stats.insert(
            RuntimeType::Native,
            RuntimeStats {
                runtime_type: RuntimeType::Native,
                total_executions: 10,
                successful_executions: 10,
                avg_execution_time: Duration::from_secs(1),
                p95_execution_time: Duration::ZERO,
                avg_memory_usage: 100.0,
                avg_cpu_usage: 40.0,
                success_rate: 100.0,
                efficiency_score: 80.0,
                current_load: 0.0,
            },
        );
        stats.insert(
            RuntimeType::Wasm,
            RuntimeStats {
                runtime_type: RuntimeType::Wasm,
                total_executions: 10,
                successful_executions: 10,
                avg_execution_time: Duration::from_millis(500),
                p95_execution_time: Duration::ZERO,
                avg_memory_usage: 64.0,
                avg_cpu_usage: 25.0,
                success_rate: 100.0,
                efficiency_score: 90.0,
                current_load: 0.0,
            },
        );
        let mut baselines = HashMap::new();

        update_model_from_history(&history, &mut stats, &mut baselines, 10);

        assert!(baselines.contains_key("Native"));
        assert!(baselines.contains_key("Wasm"));
        assert!(stats.get(&RuntimeType::Native).unwrap().p95_execution_time > Duration::ZERO);
        assert!(stats.get(&RuntimeType::Wasm).unwrap().p95_execution_time > Duration::ZERO);
    }

    #[test]
    fn test_create_optimizer_fastest() {
        let config = PerformanceConfig::default();
        let opt = IntelligentPerformanceOptimizer::new(
            config,
            RuntimeSelectionStrategy::FastestExecution,
        );
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
        use toadstool::workload::ExecutableSource;

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
            env_vars: std::collections::HashMap::default(),
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
