// SPDX-License-Identifier: AGPL-3.0-or-later
//! Internal types for the intelligent performance optimizer.
//! Not part of the public API.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use toadstool::execution::RuntimeType;

use crate::types::{PerformanceMetrics, ResourcePrediction};

/// Baseline metrics for a runtime (used in model updates)
#[allow(dead_code, reason = "reserved for model updates; used in tests")]
#[expect(clippy::struct_field_names, reason = "field names match domain")]
#[derive(Clone)]
pub(super) struct BaselineMetrics {
    pub(super) avg_execution_time: Duration,
    pub(super) avg_memory_mb: f64,
    pub(super) avg_cpu_percent: f64,
}

/// Runtime selector state (for future ML-based selection)
#[derive(Default)]
pub(super) struct RuntimeSelector {
    #[allow(dead_code, reason = "reserved for ML-based selection")]
    pub(super) last_selection: Option<RuntimeType>,
}

/// Exponential moving average prediction model.
///
/// Uses EMA for execution time, memory, and CPU to smooth historical data
/// and produce stable predictions. Confidence increases with sample count.
pub(super) struct PredictionModel {
    /// EMA of execution time in seconds
    ema_execution_secs: f64,
    /// EMA of memory usage in MB
    ema_memory_mb: f64,
    /// EMA of CPU usage percent
    ema_cpu_percent: f64,
    /// Smoothing factor (0 < alpha <= 1). Higher = more weight on recent samples.
    alpha: f64,
    /// Number of samples used to build the model
    sample_count: usize,
}

impl PredictionModel {
    /// Create a new model with default values (used when no history yet).
    pub(super) const fn new() -> Self {
        Self {
            ema_execution_secs: 10.0,
            ema_memory_mb: 256.0,
            ema_cpu_percent: 50.0,
            alpha: 0.2,
            sample_count: 0,
        }
    }

    /// Update model with a new observation (call once per metric).
    pub(super) fn update(&mut self, metrics: &PerformanceMetrics) {
        let exec_secs = metrics.execution_duration.map_or(0.0, |d| d.as_secs_f64());
        let mem_mb = metrics.resource_metrics.memory.used_bytes as f64 / 1024.0 / 1024.0;
        let cpu = metrics.resource_metrics.cpu.usage_percent;

        if self.sample_count == 0 {
            self.ema_execution_secs = exec_secs;
            self.ema_memory_mb = mem_mb;
            self.ema_cpu_percent = cpu;
        } else {
            self.ema_execution_secs = self
                .alpha
                .mul_add(exec_secs, (1.0 - self.alpha) * self.ema_execution_secs);
            self.ema_memory_mb = self
                .alpha
                .mul_add(mem_mb, (1.0 - self.alpha) * self.ema_memory_mb);
            self.ema_cpu_percent = self
                .alpha
                .mul_add(cpu, (1.0 - self.alpha) * self.ema_cpu_percent);
        }
        self.sample_count += 1;
    }

    /// Reset and rebuild from metrics (avoids double-counting when `update_model` is called repeatedly).
    pub(super) fn rebuild_from_metrics(&mut self, metrics: &[&PerformanceMetrics]) {
        *self = Self::new();
        for m in metrics {
            self.update(m);
        }
    }

    /// Produce a resource prediction. Confidence scales with sample count (capped at 95%).
    pub(super) fn predict(&self) -> ResourcePrediction {
        let confidence = (self.sample_count as f64 * 10.0).clamp(20.0, 95.0);
        ResourcePrediction {
            timestamp: SystemTime::now(),
            execution_time: Duration::from_secs_f64(self.ema_execution_secs.max(0.001)),
            memory_mb: self.ema_memory_mb.max(0.0),
            cpu_percent: self.ema_cpu_percent.clamp(0.0, 100.0),
            confidence,
            model_type: "exponential_moving_average".to_string(),
        }
    }

    pub(super) const fn sample_count(&self) -> usize {
        self.sample_count
    }
}

impl Default for PredictionModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Rebuild prediction models from metrics history (replaces existing to avoid double-counting).
pub(super) fn update_prediction_models_from_history(
    history: &std::collections::VecDeque<PerformanceMetrics>,
    models: &mut HashMap<String, PredictionModel>,
    min_samples: usize,
) {
    if history.len() < min_samples {
        return;
    }
    let mut by_runtime: HashMap<String, Vec<&PerformanceMetrics>> = HashMap::new();
    for m in history {
        by_runtime
            .entry(format!("{:?}", m.runtime_type))
            .or_default()
            .push(m);
    }
    for (key, runtime_metrics) in by_runtime {
        if runtime_metrics.len() >= min_samples {
            let model = models.entry(key).or_default();
            model.rebuild_from_metrics(&runtime_metrics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use toadstool::execution::RuntimeType;
    use toadstool::resources::{CpuMetrics, MemoryMetrics, RuntimeMetrics, StorageMetrics};
    use toadstool::resources::{NetworkMetrics, TimingMetrics};

    fn make_metrics(
        runtime_type: RuntimeType,
        exec_secs: f64,
        memory_bytes: u64,
        cpu_percent: f64,
    ) -> PerformanceMetrics {
        let start = std::time::SystemTime::now();
        let duration = Duration::from_secs_f64(exec_secs);
        PerformanceMetrics {
            execution_id: format!("exec-{}", uuid::Uuid::new_v4()),
            runtime_type,
            workload_type: "test".to_string(),
            start_time: start,
            end_time: Some(start + duration),
            execution_duration: Some(duration),
            resource_metrics: RuntimeMetrics {
                cpu: CpuMetrics {
                    usage_percent: cpu_percent,
                    cores_used: 1.0,
                    cpu_time_seconds: exec_secs,
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
                    start_time: start,
                    end_time: Some(start + duration),
                    duration,
                },
            },
            success: true,
            error_message: None,
            performance_score: 80.0,
            efficiency_score: 75.0,
        }
    }

    #[test]
    fn test_prediction_model_new() {
        let model = PredictionModel::new();
        assert_eq!(model.sample_count(), 0);
    }

    #[test]
    fn test_prediction_model_default() {
        let model = PredictionModel::default();
        assert_eq!(model.sample_count(), 0);
    }

    #[test]
    fn test_prediction_model_update_and_predict() {
        let mut model = PredictionModel::new();
        let metrics = make_metrics(RuntimeType::Native, 2.0, 256 * 1024 * 1024, 50.0);
        model.update(&metrics);
        assert_eq!(model.sample_count(), 1);
        let pred = model.predict();
        assert!(pred.confidence >= 20.0);
        assert!(pred.execution_time.as_secs_f64() > 0.0);
    }

    #[test]
    fn test_prediction_model_rebuild_from_metrics() {
        let mut model = PredictionModel::new();
        let m1 = make_metrics(RuntimeType::Native, 1.0, 100 * 1024 * 1024, 30.0);
        let m2 = make_metrics(RuntimeType::Native, 3.0, 200 * 1024 * 1024, 70.0);
        model.rebuild_from_metrics(&[&m1, &m2]);
        assert_eq!(model.sample_count(), 2);
    }

    #[test]
    fn test_runtime_selector_default() {
        let selector = RuntimeSelector::default();
        assert!(selector.last_selection.is_none());
    }

    #[test]
    fn test_update_prediction_models_from_history_insufficient() {
        let mut history: VecDeque<PerformanceMetrics> = VecDeque::new();
        history.push_back(make_metrics(
            RuntimeType::Native,
            1.0,
            100 * 1024 * 1024,
            50.0,
        ));
        let mut models = HashMap::new();
        update_prediction_models_from_history(&history, &mut models, 10);
        assert!(models.is_empty());
    }

    #[test]
    fn test_update_prediction_models_from_history_sufficient() {
        let mut history: VecDeque<PerformanceMetrics> = VecDeque::new();
        for i in 0..15 {
            history.push_back(make_metrics(
                RuntimeType::Native,
                1.0 + f64::from(i) * 0.1,
                100 * 1024 * 1024,
                50.0,
            ));
        }
        let mut models = HashMap::new();
        update_prediction_models_from_history(&history, &mut models, 10);
        assert!(!models.is_empty());
    }
}
