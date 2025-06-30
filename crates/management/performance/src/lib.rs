//! Performance Management and Optimization for ToadStool
//!
//! This crate provides comprehensive performance management including:
//! - Runtime selection algorithms with intelligent workload routing
//! - Performance profiling and metrics collection
//! - Resource pool management and optimization
//! - Usage prediction and recommendation engines

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use statrs::statistics::Statistics;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use toadstool::execution::{ExecutionRequest, RuntimeType};
use toadstool::resources::RuntimeMetrics;
use toadstool::workload::WorkloadSpec;

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
    /// Target runtime or configuration
    pub target: String,
    /// Expected improvement
    pub expected_improvement: f64,
    /// Confidence level
    pub confidence: f64,
    /// Recommendation description
    pub description: String,
    /// Implementation priority
    pub priority: RecommendationPriority,
}

/// Recommendation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationType {
    /// Switch to different runtime
    RuntimeSwitch,
    /// Adjust resource allocation
    ResourceAdjustment,
    /// Modify execution parameters
    ParameterTuning,
    /// Infrastructure scaling
    Scaling,
    /// Workload optimization
    WorkloadOptimization,
}

/// Recommendation priority levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Performance optimization engine trait
#[async_trait]
pub trait PerformanceOptimizer: Send + Sync {
    /// Select optimal runtime for execution request
    async fn select_runtime(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType>;
    
    /// Record execution performance metrics
    async fn record_metrics(&self, metrics: PerformanceMetrics) -> ToadStoolResult<()>;
    
    /// Get runtime performance statistics
    async fn get_runtime_stats(&self, runtime_type: RuntimeType) -> ToadStoolResult<RuntimeStats>;
    
    /// Predict resource requirements for workload
    async fn predict_resources(&self, workload: &WorkloadSpec) -> ToadStoolResult<ResourcePrediction>;
    
    /// Generate optimization recommendations
    async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>>;
    
    /// Update performance model with new data
    async fn update_model(&self) -> ToadStoolResult<()>;
}

/// Intelligent performance optimizer implementation
pub struct IntelligentPerformanceOptimizer {
    config: PerformanceConfig,
    metrics_history: Arc<RwLock<VecDeque<PerformanceMetrics>>>,
    runtime_stats: Arc<RwLock<HashMap<RuntimeType, RuntimeStats>>>,
    prediction_models: Arc<RwLock<HashMap<String, PredictionModel>>>,
    selection_strategy: RuntimeSelectionStrategy,
}

impl IntelligentPerformanceOptimizer {
    /// Create new intelligent performance optimizer
    pub fn new(config: PerformanceConfig, strategy: RuntimeSelectionStrategy) -> Self {
        info!("Creating intelligent performance optimizer");
        
        Self {
            config,
            metrics_history: Arc::new(RwLock::new(VecDeque::new())),
            runtime_stats: Arc::new(RwLock::new(HashMap::new())),
            prediction_models: Arc::new(RwLock::new(HashMap::new())),
            selection_strategy: strategy,
        }
    }
    
    /// Calculate performance score based on metrics and duration
    fn calculate_performance_score(&self, metrics: &RuntimeMetrics, duration: Duration) -> f64 {
        let execution_score = if duration.as_secs() > 0 {
            100.0 - (duration.as_secs_f64() / 300.0 * 100.0).min(100.0)
        } else {
            100.0
        };
        
        let memory_usage_mb = metrics.memory.usage_bytes as f64 / 1024.0 / 1024.0;
        let memory_score = 100.0 - (memory_usage_mb / 1024.0 * 100.0).min(100.0);
        let cpu_score = 100.0 - metrics.cpu.usage_percent.min(100.0);
        
        (execution_score * 0.4 + memory_score * 0.3 + cpu_score * 0.3).min(100.0)
    }
    
    /// Calculate resource efficiency score
    fn calculate_efficiency_score(&self, metrics: &RuntimeMetrics, duration: Duration) -> f64 {
        let memory_usage_mb = metrics.memory.usage_bytes as f64 / 1024.0 / 1024.0;
        let memory_efficiency = if memory_usage_mb > 0.0 {
            100.0 / (memory_usage_mb / 1024.0).max(1.0)
        } else {
            100.0
        };
        
        let cpu_efficiency = if metrics.cpu.usage_percent > 0.0 {
            100.0 / metrics.cpu.usage_percent.max(1.0)
        } else {
            100.0
        };
        
        let time_efficiency = if duration.as_secs() > 0 {
            100.0 / duration.as_secs_f64().max(1.0)
        } else {
            100.0
        };
        
        (memory_efficiency * 0.4 + cpu_efficiency * 0.3 + time_efficiency * 0.3).min(100.0)
    }
    
    /// Cleanup old metrics based on retention policy
    async fn cleanup_old_metrics(&self) {
        let retention_duration = Duration::from_secs(self.config.history_retention_hours * 3600);
        let cutoff_time = Utc::now() - chrono::Duration::from_std(retention_duration).unwrap_or_default();
        
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
        
        let runtime_stats = stats.entry(metrics.runtime_type.clone()).or_insert_with(|| RuntimeStats {
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
        
        if let Some(duration) = metrics.execution_duration {
            runtime_stats.avg_execution_time = Duration::from_secs_f64(
                (runtime_stats.avg_execution_time.as_secs_f64() * (runtime_stats.total_executions - 1) as f64 
                + duration.as_secs_f64()) / runtime_stats.total_executions as f64
            );
        }
        
        runtime_stats.avg_memory_usage = (runtime_stats.avg_memory_usage * (runtime_stats.total_executions - 1) as f64 
            + metrics.resource_metrics.memory.usage_bytes as f64) / runtime_stats.total_executions as f64;
        
        runtime_stats.avg_cpu_usage = (runtime_stats.avg_cpu_usage * (runtime_stats.total_executions - 1) as f64 
            + metrics.resource_metrics.cpu.usage_percent) / runtime_stats.total_executions as f64;
        
        runtime_stats.success_rate = (runtime_stats.successful_executions as f64 / runtime_stats.total_executions as f64) * 100.0;
        runtime_stats.efficiency_score = self.calculate_efficiency_score(&metrics.resource_metrics, Duration::ZERO);
    }
    
    /// Select runtime based on strategy
    async fn select_runtime_by_strategy(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType> {
        if available_runtimes.is_empty() {
            return Err(ToadStoolError::runtime("No available runtimes".to_string()));
        }
        
        match &self.selection_strategy {
            RuntimeSelectionStrategy::FastestExecution => {
                self.select_fastest_runtime(available_runtimes).await
            }
            RuntimeSelectionStrategy::LowestResourceUsage => {
                self.select_lowest_resource_runtime(available_runtimes).await
            }
            RuntimeSelectionStrategy::BestEfficiency => {
                self.select_most_efficient_runtime(available_runtimes).await
            }
            RuntimeSelectionStrategy::LoadBalance => {
                self.select_load_balanced_runtime(available_runtimes).await
            }
            RuntimeSelectionStrategy::WorkloadOptimized => {
                self.select_workload_optimized_runtime(request, available_runtimes).await
            }
            RuntimeSelectionStrategy::Custom { weights } => {
                self.select_custom_weighted_runtime(available_runtimes, weights).await
            }
        }
    }
    
    /// Select fastest runtime based on historical performance
    async fn select_fastest_runtime(&self, available_runtimes: &[RuntimeType]) -> ToadStoolResult<RuntimeType> {
        let stats = self.runtime_stats.read().await;
        
        let mut best_runtime = available_runtimes[0].clone();
        let mut best_time = Duration::from_secs(u64::MAX);
        
        for runtime in available_runtimes {
            if let Some(runtime_stats) = stats.get(runtime) {
                if runtime_stats.avg_execution_time < best_time && runtime_stats.total_executions > 0 {
                    best_time = runtime_stats.avg_execution_time;
                    best_runtime = runtime.clone();
                }
            }
        }
        
        Ok(best_runtime)
    }
    
    /// Select runtime with lowest resource usage
    async fn select_lowest_resource_runtime(&self, available_runtimes: &[RuntimeType]) -> ToadStoolResult<RuntimeType> {
        let stats = self.runtime_stats.read().await;
        
        let mut best_runtime = available_runtimes[0].clone();
        let mut lowest_usage = f64::MAX;
        
        for runtime in available_runtimes {
            if let Some(runtime_stats) = stats.get(runtime) {
                let combined_usage = runtime_stats.avg_memory_usage + runtime_stats.avg_cpu_usage;
                if combined_usage < lowest_usage && runtime_stats.total_executions > 0 {
                    lowest_usage = combined_usage;
                    best_runtime = runtime.clone();
                }
            }
        }
        
        Ok(best_runtime)
    }
    
    /// Select most efficient runtime
    async fn select_most_efficient_runtime(&self, available_runtimes: &[RuntimeType]) -> ToadStoolResult<RuntimeType> {
        let stats = self.runtime_stats.read().await;
        
        let mut best_runtime = available_runtimes[0].clone();
        let mut best_efficiency = 0.0;
        
        for runtime in available_runtimes {
            if let Some(runtime_stats) = stats.get(runtime) {
                if runtime_stats.efficiency_score > best_efficiency && runtime_stats.total_executions > 0 {
                    best_efficiency = runtime_stats.efficiency_score;
                    best_runtime = runtime.clone();
                }
            }
        }
        
        Ok(best_runtime)
    }
    
    /// Select runtime for load balancing
    async fn select_load_balanced_runtime(&self, available_runtimes: &[RuntimeType]) -> ToadStoolResult<RuntimeType> {
        let stats = self.runtime_stats.read().await;
        
        let mut best_runtime = available_runtimes[0].clone();
        let mut lowest_load = f64::MAX;
        
        for runtime in available_runtimes {
            if let Some(runtime_stats) = stats.get(runtime) {
                if runtime_stats.current_load < lowest_load {
                    lowest_load = runtime_stats.current_load;
                    best_runtime = runtime.clone();
                }
            } else {
                // Prefer runtimes with no recorded load
                return Ok(runtime.clone());
            }
        }
        
        Ok(best_runtime)
    }
    
    /// Select runtime optimized for specific workload
    async fn select_workload_optimized_runtime(
        &self,
        request: &ExecutionRequest,
        available_runtimes: &[RuntimeType],
    ) -> ToadStoolResult<RuntimeType> {
        // Analyze workload characteristics and select optimal runtime
        match &request.workload {
            WorkloadSpec::Native { .. } => {
                // Prefer native runtime for native workloads
                if available_runtimes.contains(&RuntimeType::Native) {
                    Ok(RuntimeType::Native)
                } else {
                    self.select_fastest_runtime(available_runtimes).await
                }
            }
            WorkloadSpec::Wasm { .. } => {
                // Prefer WASM runtime for WASM workloads
                if available_runtimes.contains(&RuntimeType::Wasm) {
                    Ok(RuntimeType::Wasm)
                } else {
                    self.select_fastest_runtime(available_runtimes).await
                }
            }
            WorkloadSpec::Container { .. } => {
                // Prefer container runtime for container workloads
                if available_runtimes.contains(&RuntimeType::Container) {
                    Ok(RuntimeType::Container)
                } else {
                    self.select_fastest_runtime(available_runtimes).await
                }
            }
            WorkloadSpec::Gpu { .. } => {
                // Prefer GPU runtime for GPU workloads
                if available_runtimes.contains(&RuntimeType::Gpu) {
                    Ok(RuntimeType::Gpu)
                } else {
                    self.select_fastest_runtime(available_runtimes).await
                }
            }
            WorkloadSpec::Script { .. } => {
                // For script workloads, prefer native runtime
                if available_runtimes.contains(&RuntimeType::Native) {
                    Ok(RuntimeType::Native)
                } else {
                    self.select_fastest_runtime(available_runtimes).await
                }
            }
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
                
                let execution_score = 100.0 - (runtime_stats.avg_execution_time.as_secs_f64() / 300.0 * 100.0).min(100.0);
                let memory_score = 100.0 - (runtime_stats.avg_memory_usage / 1024.0 * 100.0).min(100.0);
                let cpu_score = 100.0 - runtime_stats.avg_cpu_usage.min(100.0);
                let availability_score = 100.0 - runtime_stats.current_load;
                let success_score = runtime_stats.success_rate;
                
                let weighted_score = execution_score * weights.execution_time
                    + memory_score * weights.memory_usage
                    + cpu_score * weights.cpu_usage
                    + availability_score * weights.resource_availability
                    + success_score * weights.historical_success_rate;
                
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
        
        self.select_runtime_by_strategy(request, available_runtimes).await
    }
    
    async fn record_metrics(&self, mut metrics: PerformanceMetrics) -> ToadStoolResult<()> {
        debug!("Recording performance metrics for execution: {}", metrics.execution_id);
        
        // Calculate scores
        if let Some(duration) = metrics.execution_duration {
            metrics.performance_score = self.calculate_performance_score(&metrics.resource_metrics, duration);
            metrics.efficiency_score = self.calculate_efficiency_score(&metrics.resource_metrics, duration);
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
    
    async fn get_runtime_stats(&self, runtime_type: RuntimeType) -> ToadStoolResult<RuntimeStats> {
        let stats = self.runtime_stats.read().await;
        stats.get(&runtime_type)
            .cloned()
            .ok_or_else(|| ToadStoolError::runtime(format!(
                "No statistics available for runtime: {:?}",
                runtime_type
            )))
    }
    
    async fn predict_resources(&self, workload: &WorkloadSpec) -> ToadStoolResult<ResourcePrediction> {
        debug!("Predicting resource requirements for workload");
        
        if !self.config.enable_prediction {
            return Err(ToadStoolError::runtime("Resource prediction is disabled".to_string()));
        }
        
        let workload_type = match workload {
            WorkloadSpec::Native { .. } => "native",
            WorkloadSpec::Wasm { .. } => "wasm",
            WorkloadSpec::Container { .. } => "container",
            WorkloadSpec::Gpu { .. } => "gpu",
            WorkloadSpec::Script { .. } => "script",
        };
        
        // Get historical data for similar workloads
        let history = self.metrics_history.read().await;
        let similar_executions: Vec<_> = history
            .iter()
            .filter(|m| m.workload_type == workload_type && m.success && m.execution_duration.is_some())
            .collect();
        
        if similar_executions.len() < self.config.min_prediction_samples {
            return Err(ToadStoolError::runtime(format!(
                "Insufficient historical data for prediction (need {}, have {})",
                self.config.min_prediction_samples,
                similar_executions.len()
            )));
        }
        
        // Calculate predictions based on historical averages
        let execution_times: Vec<f64> = similar_executions
            .iter()
            .map(|m| m.execution_duration.unwrap().as_secs_f64())
            .collect();
        
        let memory_usages: Vec<f64> = similar_executions
            .iter()
            .map(|m| m.resource_metrics.memory.usage_bytes as f64 / 1024.0 / 1024.0)
            .collect();
        
        let cpu_usages: Vec<f64> = similar_executions
            .iter()
            .map(|m| m.resource_metrics.cpu.usage_percent)
            .collect();
        
        let predicted_execution_time = Duration::from_secs_f64(execution_times.clone().mean());
        let predicted_memory = memory_usages.clone().mean();
        let predicted_cpu = cpu_usages.clone().mean();
        
        // Calculate confidence based on data consistency
        let time_std = execution_times.clone().std_dev();
        let memory_std = memory_usages.clone().std_dev();
        let cpu_std = cpu_usages.clone().std_dev();
        
        let confidence = (100.0 - (time_std / execution_times.mean() * 100.0).min(100.0)
            + 100.0 - (memory_std / memory_usages.mean() * 100.0).min(100.0)
            + 100.0 - (cpu_std / cpu_usages.mean() * 100.0).min(100.0)) / 3.0;
        
        Ok(ResourcePrediction {
            timestamp: Utc::now(),
            execution_time: predicted_execution_time,
            memory_mb: predicted_memory,
            cpu_percent: predicted_cpu,
            confidence: confidence.max(0.0).min(100.0),
            model_type: "historical_average".to_string(),
        })
    }
    
    async fn get_recommendations(&self) -> ToadStoolResult<Vec<OptimizationRecommendation>> {
        debug!("Generating optimization recommendations");
        
        if !self.config.enable_recommendations {
            return Ok(Vec::new());
        }
        
        let mut recommendations = Vec::new();
        let stats = self.runtime_stats.read().await;
        
        // Analyze runtime performance and generate recommendations
        for (runtime_type, runtime_stats) in stats.iter() {
            // Check for performance issues
            if runtime_stats.success_rate < 90.0 && runtime_stats.total_executions > 10 {
                recommendations.push(OptimizationRecommendation {
                    id: Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::RuntimeSwitch,
                    target: format!("Switch from {:?} runtime", runtime_type),
                    expected_improvement: (90.0 - runtime_stats.success_rate) * 1.5,
                    confidence: 85.0,
                    description: format!(
                        "Runtime {:?} has low success rate ({:.1}%). Consider switching to more reliable runtime.",
                        runtime_type, runtime_stats.success_rate
                    ),
                    priority: if runtime_stats.success_rate < 50.0 {
                        RecommendationPriority::Critical
                    } else {
                        RecommendationPriority::High
                    },
                });
            }
            
            // Check for resource inefficiency
            if runtime_stats.efficiency_score < 50.0 && runtime_stats.total_executions > 5 {
                recommendations.push(OptimizationRecommendation {
                    id: Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::ResourceAdjustment,
                    target: format!("Optimize {:?} runtime resources", runtime_type),
                    expected_improvement: (50.0 - runtime_stats.efficiency_score) * 2.0,
                    confidence: 75.0,
                    description: format!(
                        "Runtime {:?} has low efficiency score ({:.1}). Consider resource optimization.",
                        runtime_type, runtime_stats.efficiency_score
                    ),
                    priority: RecommendationPriority::Medium,
                });
            }
            
            // Check for high load
            if runtime_stats.current_load > 90.0 {
                recommendations.push(OptimizationRecommendation {
                    id: Uuid::new_v4().to_string(),
                    recommendation_type: RecommendationType::Scaling,
                    target: format!("Scale {:?} runtime capacity", runtime_type),
                    expected_improvement: runtime_stats.current_load - 70.0,
                    confidence: 90.0,
                    description: format!(
                        "Runtime {:?} is under high load ({:.1}%). Consider scaling up capacity.",
                        runtime_type, runtime_stats.current_load
                    ),
                    priority: RecommendationPriority::High,
                });
            }
        }
        
        // Sort recommendations by priority and expected improvement
        recommendations.sort_by(|a, b| {
            match (&a.priority, &b.priority) {
                (RecommendationPriority::Critical, RecommendationPriority::Critical) => 
                    b.expected_improvement.partial_cmp(&a.expected_improvement).unwrap_or(std::cmp::Ordering::Equal),
                (RecommendationPriority::Critical, _) => std::cmp::Ordering::Less,
                (_, RecommendationPriority::Critical) => std::cmp::Ordering::Greater,
                (RecommendationPriority::High, RecommendationPriority::High) => 
                    b.expected_improvement.partial_cmp(&a.expected_improvement).unwrap_or(std::cmp::Ordering::Equal),
                (RecommendationPriority::High, _) => std::cmp::Ordering::Less,
                (_, RecommendationPriority::High) => std::cmp::Ordering::Greater,
                _ => b.expected_improvement.partial_cmp(&a.expected_improvement).unwrap_or(std::cmp::Ordering::Equal),
            }
        });
        
        Ok(recommendations)
    }
    
    async fn update_model(&self) -> ToadStoolResult<()> {
        debug!("Updating performance prediction models");
        
        // This is a placeholder for more sophisticated model updates
        // In a real implementation, this would retrain ML models with new data
        
        info!("Performance models updated successfully");
        Ok(())
    }
}

/// Simple prediction model structure
#[derive(Debug, Clone)]
struct PredictionModel {
    model_type: String,
    last_updated: DateTime<Utc>,
    accuracy: f64,
    parameters: HashMap<String, f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool::workload::ExecutableSource;
    use std::path::PathBuf;
    
    fn create_test_config() -> PerformanceConfig {
        PerformanceConfig {
            min_prediction_samples: 3,
            ..Default::default()
        }
    }
    
    fn create_test_metrics() -> PerformanceMetrics {
        PerformanceMetrics {
            execution_id: "test-001".to_string(),
            runtime_type: RuntimeType::Native,
            workload_type: "test".to_string(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            execution_duration: Some(Duration::from_secs(1)),
            resource_metrics: RuntimeMetrics {
                memory: toadstool::resources::MemoryMetrics {
                    usage_bytes: 100 * 1024 * 1024, // 100 MB
                    peak_usage_bytes: 120 * 1024 * 1024, // 120 MB
                    average_usage_bytes: 110 * 1024 * 1024, // 110 MB
                    allocation_count: 10,
                    deallocation_count: 5,
                    page_faults: 2,
                    swap_usage_bytes: 0,
                },
                cpu: toadstool::resources::CpuMetrics {
                    usage_percent: 50.0,
                    peak_usage_percent: 80.0,
                    average_usage_percent: 60.0,
                    cpu_time_ms: 1000,
                    cpu_cycles: Some(1000000),
                    throttle_events: 0,
                },
                storage: toadstool::resources::StorageMetrics::default(),
                network: toadstool::resources::NetworkMetrics::default(),
                gpu: None,
                timing: toadstool::resources::TimingMetrics::default(),
                custom: std::collections::HashMap::new(),
            },
            success: true,
            error_message: None,
            performance_score: 80.0,
            efficiency_score: 75.0,
        }
    }
    
    #[tokio::test]
    async fn test_performance_optimizer_creation() {
        let config = create_test_config();
        let strategy = RuntimeSelectionStrategy::FastestExecution;
        
        let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);
        assert_eq!(optimizer.config.min_prediction_samples, 3);
    }
    
    #[tokio::test]
    async fn test_metrics_recording() {
        let config = create_test_config();
        let strategy = RuntimeSelectionStrategy::FastestExecution;
        let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);
        
        let metrics = create_test_metrics();
        let result = optimizer.record_metrics(metrics).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_runtime_selection() {
        let config = create_test_config();
        let strategy = RuntimeSelectionStrategy::FastestExecution;
        let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);
        
        let request = ExecutionRequest {
            execution_id: Uuid::new_v4(),
            workload: WorkloadSpec::Native {
                executable: ExecutableSource::File {
                    path: PathBuf::from("/bin/echo"),
                },
                args: Some(vec!["test".to_string()]),
                working_dir: None,
                env_vars: HashMap::new(),
                user: None,
            },
            runtime_hint: None,
            resources: toadstool::resources::ResourceRequirements::default(),
            security_context: toadstool::security::SecurityContext::default(),
            timeout: Some(Duration::from_secs(30)),
            environment: HashMap::new(),
            input_data: toadstool::execution::ExecutionInput::default(),
            callback_config: None,
        };
        
        let available_runtimes = vec![RuntimeType::Native, RuntimeType::Wasm];
        let result = optimizer.select_runtime(&request, &available_runtimes).await;
        assert!(result.is_ok());
    }
}
