//! Internal types for the intelligent performance optimizer.
//! Not part of the public API.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use toadstool::execution::RuntimeType;

use crate::types::{PerformanceMetrics, ResourcePrediction};

/// Baseline metrics for a runtime (used in model updates)
#[derive(Clone)]
pub(super) struct BaselineMetrics {
    pub(super) _avg_execution_time: Duration,
    pub(super) _avg_memory_mb: f64,
    pub(super) _avg_cpu_percent: f64,
}

/// Runtime selector state (for future ML-based selection)
#[derive(Default)]
pub(super) struct RuntimeSelector {
    pub(super) _last_selection: Option<RuntimeType>,
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
    pub(super) fn new() -> Self {
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
        let exec_secs = metrics
            .execution_duration
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
        let mem_mb = metrics.resource_metrics.memory.used_bytes as f64 / 1024.0 / 1024.0;
        let cpu = metrics.resource_metrics.cpu.usage_percent;

        if self.sample_count == 0 {
            self.ema_execution_secs = exec_secs;
            self.ema_memory_mb = mem_mb;
            self.ema_cpu_percent = cpu;
        } else {
            self.ema_execution_secs =
                self.alpha * exec_secs + (1.0 - self.alpha) * self.ema_execution_secs;
            self.ema_memory_mb = self.alpha * mem_mb + (1.0 - self.alpha) * self.ema_memory_mb;
            self.ema_cpu_percent = self.alpha * cpu + (1.0 - self.alpha) * self.ema_cpu_percent;
        }
        self.sample_count += 1;
    }

    /// Reset and rebuild from metrics (avoids double-counting when update_model is called repeatedly).
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

    pub(super) fn sample_count(&self) -> usize {
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
    for m in history.iter() {
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
