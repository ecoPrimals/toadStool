//! Performance management types
//!
//! Core types for performance optimization, metrics, and statistics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use toadstool::execution::RuntimeType;
use toadstool::resources::RuntimeMetrics;

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable runtime selection optimization
    pub enable_runtime_selection: bool,
    /// Enable performance profiling
    pub enable_profiling: bool,
    /// Enable resource prediction
    pub enable_prediction: bool,
    /// Enable optimization recommendations
    pub enable_recommendations: bool,
    /// Metrics collection interval in milliseconds
    pub metrics_interval_ms: u64,
    /// Historical data retention period in hours
    pub history_retention_hours: u64,
    /// Minimum samples for prediction
    pub min_prediction_samples: usize,
    /// Performance threshold percentile
    pub performance_threshold_percentile: f64,
    /// Resource utilization target percentage
    pub target_utilization_percent: f64,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_runtime_selection: true,
            enable_profiling: true,
            enable_prediction: true,
            enable_recommendations: true,
            metrics_interval_ms: 1000,
            history_retention_hours: 24,
            min_prediction_samples: 10,
            performance_threshold_percentile: 95.0,
            target_utilization_percent: 75.0,
        }
    }
}

/// Runtime selection strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeSelectionStrategy {
    /// Select fastest runtime based on historical performance
    FastestExecution,
    /// Select runtime with lowest resource usage
    LowestResourceUsage,
    /// Select runtime with best resource efficiency
    BestEfficiency,
    /// Load balance across available runtimes
    LoadBalance,
    /// Select based on workload characteristics
    WorkloadOptimized,
    /// Custom selection with weighted factors
    Custom { weights: SelectionWeights },
}

/// Selection weights for custom strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionWeights {
    pub execution_time: f64,
    pub memory_usage: f64,
    pub cpu_usage: f64,
    pub resource_availability: f64,
    pub historical_success_rate: f64,
}

impl Default for SelectionWeights {
    fn default() -> Self {
        Self {
            execution_time: 0.3,
            memory_usage: 0.2,
            cpu_usage: 0.2,
            resource_availability: 0.2,
            historical_success_rate: 0.1,
        }
    }
}

/// Performance metrics for a runtime execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Execution identifier
    pub execution_id: String,
    /// Runtime type used
    pub runtime_type: RuntimeType,
    /// Workload type
    pub workload_type: String,
    /// Execution start time
    pub start_time: DateTime<Utc>,
    /// Execution end time
    pub end_time: Option<DateTime<Utc>>,
    /// Total execution duration
    pub execution_duration: Option<Duration>,
    /// Resource metrics
    pub resource_metrics: RuntimeMetrics,
    /// Success/failure status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Performance score (0-100)
    pub performance_score: f64,
    /// Resource efficiency score (0-100)
    pub efficiency_score: f64,
}

/// Runtime performance statistics
#[derive(Debug, Clone)]
pub struct RuntimeStats {
    /// Runtime type
    pub runtime_type: RuntimeType,
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Average execution time
    pub avg_execution_time: Duration,
    /// P95 execution time
    pub p95_execution_time: Duration,
    /// Average memory usage
    pub avg_memory_usage: f64,
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Resource efficiency score
    pub efficiency_score: f64,
    /// Current load (0-100)
    pub current_load: f64,
}

/// Resource prediction
#[derive(Debug, Clone)]
pub struct ResourcePrediction {
    /// Prediction timestamp
    pub timestamp: DateTime<Utc>,
    /// Predicted execution time
    pub execution_time: Duration,
    /// Predicted memory usage in MB
    pub memory_mb: f64,
    /// Predicted CPU usage percentage
    pub cpu_percent: f64,
    /// Confidence level (0-100)
    pub confidence: f64,
    /// Prediction model used
    pub model_type: String,
}

/// Optimization recommendation
#[derive(Debug, Clone)]
pub struct OptimizationRecommendation {
    /// Recommendation ID
    pub id: String,
    /// Recommendation type
    pub recommendation_type: RecommendationType,
    /// Priority (1-10, 10 being highest)
    pub priority: u8,
    /// Expected performance improvement percentage
    pub expected_improvement: f64,
    /// Recommendation description
    pub description: String,
    /// Specific action items
    pub actions: Vec<String>,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Types of optimization recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// Switch to a different runtime
    RuntimeSwitch,
    /// Increase resource allocation
    ResourceIncrease,
    /// Decrease resource allocation
    ResourceDecrease,
    /// Enable performance features
    FeatureEnable,
    /// Adjust configuration
    ConfigurationAdjustment,
    /// Scale horizontally
    HorizontalScaling,
    /// Other optimization
    Other,
}
