//! Internal types for the intelligent performance optimizer.
//! Not part of the public API.

use std::time::Duration;

use toadstool::execution::RuntimeType;

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

/// Prediction model placeholder (for future ML integration)
pub(super) struct PredictionModel {
    pub(super) _model_type: String,
    pub(super) _confidence: f64,
}
