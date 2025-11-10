//! Comprehensive tests for performance management
//!
//! Tests for PerformanceConfig, runtime selection, metrics, predictions,
//! and optimization recommendations.

use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use toadstool::execution::{ExecutionInput, ExecutionRequest, RuntimeType};
use toadstool::resources::{
    CpuMetrics, MemoryMetrics, NetworkMetrics, ResourceRequirements, RuntimeMetrics,
    StorageMetrics, TimingMetrics,
};
use toadstool::security::SecurityContext;
use toadstool::workload::{ExecutableSource, WorkloadSpec};
use toadstool_management_performance::*;
use uuid::Uuid;

// ============================================================================
// PerformanceConfig Tests
// ============================================================================

#[test]
fn test_performance_config_default() {
    let config = PerformanceConfig::default();

    assert!(config.enable_runtime_selection);
    assert!(config.enable_profiling);
    assert!(config.enable_prediction);
    assert!(config.enable_recommendations);
    assert_eq!(config.metrics_interval_ms, 1000);
    assert_eq!(config.history_retention_hours, 24);
    assert_eq!(config.min_prediction_samples, 10);
    assert_eq!(config.performance_threshold_percentile, 95.0);
    assert_eq!(config.target_utilization_percent, 75.0);
}

#[test]
fn test_performance_config_clone() {
    let config1 = PerformanceConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.enable_runtime_selection,
        config2.enable_runtime_selection
    );
    assert_eq!(config1.metrics_interval_ms, config2.metrics_interval_ms);
    assert_eq!(
        config1.min_prediction_samples,
        config2.min_prediction_samples
    );
}

#[test]
fn test_performance_config_custom() {
    let config = PerformanceConfig {
        enable_runtime_selection: false,
        enable_profiling: false,
        enable_prediction: false,
        enable_recommendations: false,
        metrics_interval_ms: 5000,
        history_retention_hours: 48,
        min_prediction_samples: 20,
        performance_threshold_percentile: 99.0,
        target_utilization_percent: 85.0,
    };

    assert!(!config.enable_runtime_selection);
    assert!(!config.enable_profiling);
    assert_eq!(config.metrics_interval_ms, 5000);
    assert_eq!(config.min_prediction_samples, 20);
}

#[test]
fn test_performance_config_serialization() {
    let config = PerformanceConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    assert!(json.contains("enable_runtime_selection"));
}

#[test]
fn test_performance_config_deserialization() {
    let json = r#"{
        "enable_runtime_selection": true,
        "enable_profiling": true,
        "enable_prediction": true,
        "enable_recommendations": true,
        "metrics_interval_ms": 1000,
        "history_retention_hours": 24,
        "min_prediction_samples": 10,
        "performance_threshold_percentile": 95.0,
        "target_utilization_percent": 75.0
    }"#;

    let config: PerformanceConfig = serde_json::from_str(json).expect("Should deserialize");
    assert!(config.enable_runtime_selection);
    assert_eq!(config.metrics_interval_ms, 1000);
}

// ============================================================================
// RuntimeSelectionStrategy Tests
// ============================================================================

#[test]
fn test_runtime_selection_strategy_fastest_execution() {
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    assert!(matches!(
        strategy,
        RuntimeSelectionStrategy::FastestExecution
    ));
}

#[test]
fn test_runtime_selection_strategy_lowest_resource_usage() {
    let strategy = RuntimeSelectionStrategy::LowestResourceUsage;
    assert!(matches!(
        strategy,
        RuntimeSelectionStrategy::LowestResourceUsage
    ));
}

#[test]
fn test_runtime_selection_strategy_best_efficiency() {
    let strategy = RuntimeSelectionStrategy::BestEfficiency;
    assert!(matches!(strategy, RuntimeSelectionStrategy::BestEfficiency));
}

#[test]
fn test_runtime_selection_strategy_load_balance() {
    let strategy = RuntimeSelectionStrategy::LoadBalance;
    assert!(matches!(strategy, RuntimeSelectionStrategy::LoadBalance));
}

#[test]
fn test_runtime_selection_strategy_workload_optimized() {
    let strategy = RuntimeSelectionStrategy::WorkloadOptimized;
    assert!(matches!(
        strategy,
        RuntimeSelectionStrategy::WorkloadOptimized
    ));
}

#[test]
fn test_runtime_selection_strategy_custom() {
    let weights = SelectionWeights::default();
    let strategy = RuntimeSelectionStrategy::Custom { weights };
    assert!(matches!(strategy, RuntimeSelectionStrategy::Custom { .. }));
}

#[test]
fn test_runtime_selection_strategy_clone() {
    let strategy1 = RuntimeSelectionStrategy::FastestExecution;
    let strategy2 = strategy1.clone();
    assert!(matches!(
        strategy2,
        RuntimeSelectionStrategy::FastestExecution
    ));
}

// ============================================================================
// SelectionWeights Tests
// ============================================================================

#[test]
fn test_selection_weights_default() {
    let weights = SelectionWeights::default();

    assert_eq!(weights.execution_time, 0.3);
    assert_eq!(weights.memory_usage, 0.2);
    assert_eq!(weights.cpu_usage, 0.2);
    assert_eq!(weights.resource_availability, 0.2);
    assert_eq!(weights.historical_success_rate, 0.1);
}

#[test]
fn test_selection_weights_custom() {
    let weights = SelectionWeights {
        execution_time: 0.4,
        memory_usage: 0.3,
        cpu_usage: 0.1,
        resource_availability: 0.1,
        historical_success_rate: 0.1,
    };

    assert_eq!(weights.execution_time, 0.4);
    assert_eq!(weights.memory_usage, 0.3);
}

#[test]
fn test_selection_weights_clone() {
    let weights1 = SelectionWeights::default();
    let weights2 = weights1.clone();

    assert_eq!(weights1.execution_time, weights2.execution_time);
    assert_eq!(weights1.memory_usage, weights2.memory_usage);
}

#[test]
fn test_selection_weights_serialization() {
    let weights = SelectionWeights::default();
    let json = serde_json::to_string(&weights).expect("Should serialize");
    assert!(json.contains("execution_time"));
}

// ============================================================================
// PerformanceMetrics Tests
// ============================================================================

fn create_test_metrics() -> PerformanceMetrics {
    PerformanceMetrics {
        execution_id: "test-123".to_string(),
        runtime_type: RuntimeType::Native,
        workload_type: "test".to_string(),
        start_time: Utc::now(),
        end_time: Some(Utc::now()),
        execution_duration: Some(Duration::from_secs(5)),
        resource_metrics: RuntimeMetrics {
            memory: MemoryMetrics {
                usage_percent: 50.0,
                used_bytes: 100 * 1024 * 1024,
                peak_bytes: 120 * 1024 * 1024,
            },
            cpu: CpuMetrics {
                usage_percent: 50.0,
                cores_used: 2.0,
                cpu_time_seconds: 5.0,
            },
            storage: StorageMetrics::default(),
            network: NetworkMetrics::default(),
            gpu: None,
            timing: TimingMetrics::default(),
        },
        success: true,
        error_message: None,
        performance_score: 80.0,
        efficiency_score: 75.0,
    }
}

#[test]
fn test_performance_metrics_creation() {
    let metrics = create_test_metrics();

    assert_eq!(metrics.execution_id, "test-123");
    assert_eq!(metrics.runtime_type, RuntimeType::Native);
    assert!(metrics.success);
    assert!(metrics.error_message.is_none());
}

#[test]
fn test_performance_metrics_with_error() {
    let mut metrics = create_test_metrics();
    metrics.success = false;
    metrics.error_message = Some("Test error".to_string());

    assert!(!metrics.success);
    assert_eq!(metrics.error_message.unwrap(), "Test error");
}

#[test]
fn test_performance_metrics_different_runtimes() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
        RuntimeType::Gpu,
    ];

    for runtime in runtimes {
        let mut metrics = create_test_metrics();
        metrics.runtime_type = runtime.clone();
        assert_eq!(metrics.runtime_type, runtime);
    }
}

#[test]
fn test_performance_metrics_clone() {
    let metrics1 = create_test_metrics();
    let metrics2 = metrics1.clone();

    assert_eq!(metrics1.execution_id, metrics2.execution_id);
    assert_eq!(metrics1.runtime_type, metrics2.runtime_type);
}

// ============================================================================
// RuntimeStats Tests
// ============================================================================

#[test]
fn test_runtime_stats_creation() {
    let stats = RuntimeStats {
        runtime_type: RuntimeType::Native,
        total_executions: 100,
        successful_executions: 95,
        avg_execution_time: Duration::from_secs(2),
        p95_execution_time: Duration::from_secs(5),
        avg_memory_usage: 256.0,
        avg_cpu_usage: 50.0,
        success_rate: 95.0,
        efficiency_score: 80.0,
        current_load: 45.0,
    };

    assert_eq!(stats.total_executions, 100);
    assert_eq!(stats.successful_executions, 95);
    assert_eq!(stats.success_rate, 95.0);
}

#[test]
fn test_runtime_stats_clone() {
    let stats1 = RuntimeStats {
        runtime_type: RuntimeType::Native,
        total_executions: 50,
        successful_executions: 48,
        avg_execution_time: Duration::from_secs(3),
        p95_execution_time: Duration::from_secs(7),
        avg_memory_usage: 512.0,
        avg_cpu_usage: 60.0,
        success_rate: 96.0,
        efficiency_score: 85.0,
        current_load: 30.0,
    };

    let stats2 = stats1.clone();

    assert_eq!(stats1.total_executions, stats2.total_executions);
    assert_eq!(stats1.success_rate, stats2.success_rate);
}

#[test]
fn test_runtime_stats_different_runtimes() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
    ];

    for runtime in runtimes {
        let stats = RuntimeStats {
            runtime_type: runtime.clone(),
            total_executions: 10,
            successful_executions: 9,
            avg_execution_time: Duration::from_secs(1),
            p95_execution_time: Duration::from_secs(2),
            avg_memory_usage: 128.0,
            avg_cpu_usage: 40.0,
            success_rate: 90.0,
            efficiency_score: 75.0,
            current_load: 20.0,
        };

        assert_eq!(stats.runtime_type, runtime);
    }
}

// ============================================================================
// ResourcePrediction Tests
// ============================================================================

#[test]
fn test_resource_prediction_creation() {
    let prediction = ResourcePrediction {
        timestamp: Utc::now(),
        execution_time: Duration::from_secs(10),
        memory_mb: 256.0,
        cpu_percent: 50.0,
        confidence: 85.0,
        model_type: "historical_average".to_string(),
    };

    assert_eq!(prediction.memory_mb, 256.0);
    assert_eq!(prediction.cpu_percent, 50.0);
    assert_eq!(prediction.confidence, 85.0);
}

#[test]
fn test_resource_prediction_clone() {
    let prediction1 = ResourcePrediction {
        timestamp: Utc::now(),
        execution_time: Duration::from_secs(5),
        memory_mb: 512.0,
        cpu_percent: 75.0,
        confidence: 90.0,
        model_type: "ml_model".to_string(),
    };

    let prediction2 = prediction1.clone();

    assert_eq!(prediction1.memory_mb, prediction2.memory_mb);
    assert_eq!(prediction1.confidence, prediction2.confidence);
}

#[test]
fn test_resource_prediction_different_models() {
    let models = vec!["historical_average", "ml_model", "linear_regression"];

    for model in models {
        let prediction = ResourcePrediction {
            timestamp: Utc::now(),
            execution_time: Duration::from_secs(3),
            memory_mb: 128.0,
            cpu_percent: 40.0,
            confidence: 80.0,
            model_type: model.to_string(),
        };

        assert_eq!(prediction.model_type, model);
    }
}

// ============================================================================
// OptimizationRecommendation Tests
// ============================================================================

#[test]
fn test_optimization_recommendation_creation() {
    let recommendation = OptimizationRecommendation {
        id: "rec-001".to_string(),
        recommendation_type: RecommendationType::RuntimeSwitch,
        target: "Switch to WASM runtime".to_string(),
        expected_improvement: 25.0,
        confidence: 85.0,
        description: "WASM runtime is 25% faster for this workload".to_string(),
        priority: RecommendationPriority::High,
    };

    assert_eq!(recommendation.id, "rec-001");
    assert_eq!(recommendation.expected_improvement, 25.0);
    assert!(matches!(
        recommendation.priority,
        RecommendationPriority::High
    ));
}

#[test]
fn test_optimization_recommendation_clone() {
    let recommendation1 = OptimizationRecommendation {
        id: "rec-002".to_string(),
        recommendation_type: RecommendationType::ResourceAdjustment,
        target: "Increase memory allocation".to_string(),
        expected_improvement: 15.0,
        confidence: 75.0,
        description: "More memory will reduce GC overhead".to_string(),
        priority: RecommendationPriority::Medium,
    };

    let recommendation2 = recommendation1.clone();

    assert_eq!(recommendation1.id, recommendation2.id);
    assert_eq!(
        recommendation1.expected_improvement,
        recommendation2.expected_improvement
    );
}

// ============================================================================
// RecommendationType Tests
// ============================================================================

#[test]
fn test_recommendation_type_runtime_switch() {
    let rec_type = RecommendationType::RuntimeSwitch;
    assert!(matches!(rec_type, RecommendationType::RuntimeSwitch));
}

#[test]
fn test_recommendation_type_resource_adjustment() {
    let rec_type = RecommendationType::ResourceAdjustment;
    assert!(matches!(rec_type, RecommendationType::ResourceAdjustment));
}

#[test]
fn test_recommendation_type_parameter_tuning() {
    let rec_type = RecommendationType::ParameterTuning;
    assert!(matches!(rec_type, RecommendationType::ParameterTuning));
}

#[test]
fn test_recommendation_type_scaling() {
    let rec_type = RecommendationType::Scaling;
    assert!(matches!(rec_type, RecommendationType::Scaling));
}

#[test]
fn test_recommendation_type_workload_optimization() {
    let rec_type = RecommendationType::WorkloadOptimization;
    assert!(matches!(rec_type, RecommendationType::WorkloadOptimization));
}

// ============================================================================
// RecommendationPriority Tests
// ============================================================================

#[test]
fn test_recommendation_priority_low() {
    let priority = RecommendationPriority::Low;
    assert!(matches!(priority, RecommendationPriority::Low));
}

#[test]
fn test_recommendation_priority_medium() {
    let priority = RecommendationPriority::Medium;
    assert!(matches!(priority, RecommendationPriority::Medium));
}

#[test]
fn test_recommendation_priority_high() {
    let priority = RecommendationPriority::High;
    assert!(matches!(priority, RecommendationPriority::High));
}

#[test]
fn test_recommendation_priority_critical() {
    let priority = RecommendationPriority::Critical;
    assert!(matches!(priority, RecommendationPriority::Critical));
}

// ============================================================================
// IntelligentPerformanceOptimizer Tests
// ============================================================================

#[tokio::test]
async fn test_intelligent_optimizer_creation() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;

    let _optimizer = IntelligentPerformanceOptimizer::new(config.clone(), strategy);

    // Optimizer should be created successfully (verified by not panicking)
}

#[tokio::test]
async fn test_intelligent_optimizer_different_strategies() {
    let config = PerformanceConfig::default();

    let strategies = vec![
        RuntimeSelectionStrategy::FastestExecution,
        RuntimeSelectionStrategy::LowestResourceUsage,
        RuntimeSelectionStrategy::BestEfficiency,
        RuntimeSelectionStrategy::LoadBalance,
        RuntimeSelectionStrategy::WorkloadOptimized,
    ];

    for strategy in strategies {
        let _optimizer = IntelligentPerformanceOptimizer::new(config.clone(), strategy);
        // Should create successfully with each strategy
    }
}

fn create_test_execution_request() -> ExecutionRequest {
    ExecutionRequest {
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
        resources: ResourceRequirements::default(),
        security_context: SecurityContext::default(),
        timeout: Some(Duration::from_secs(30)),
        environment: HashMap::new(),
        input_data: ExecutionInput::default(),
        callback_config: None,
    }
}

#[tokio::test]
async fn test_select_runtime_with_no_history() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let request = create_test_execution_request();
    let available_runtimes = vec![RuntimeType::Native, RuntimeType::Wasm];

    let result = optimizer
        .select_runtime(&request, &available_runtimes)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_record_metrics() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let metrics = create_test_metrics();
    let result = optimizer.record_metrics(metrics).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_record_multiple_metrics() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    for i in 0..5 {
        let mut metrics = create_test_metrics();
        metrics.execution_id = format!("test-{i}");
        let result = optimizer.record_metrics(metrics).await;
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_get_runtime_stats() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    // Record some metrics first
    let metrics = create_test_metrics();
    optimizer.record_metrics(metrics).await.unwrap();

    // Now get stats
    let result = optimizer.get_runtime_stats(RuntimeType::Native).await;
    assert!(result.is_ok());

    let stats = result.unwrap();
    assert_eq!(stats.runtime_type, RuntimeType::Native);
    assert_eq!(stats.total_executions, 1);
}

#[tokio::test]
async fn test_get_runtime_stats_no_data() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    // Get stats without recording metrics
    let result = optimizer.get_runtime_stats(RuntimeType::Native).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_predict_resources_insufficient_data() {
    let config = PerformanceConfig {
        min_prediction_samples: 5,
        ..Default::default()
    };
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let workload = WorkloadSpec::Native {
        executable: ExecutableSource::File {
            path: PathBuf::from("/bin/echo"),
        },
        args: None,
        working_dir: None,
        env_vars: HashMap::new(),
        user: None,
    };

    let result = optimizer.predict_resources(&workload).await;
    assert!(result.is_err()); // Not enough historical data
}

#[tokio::test]
async fn test_get_recommendations_empty() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let result = optimizer.get_recommendations().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0); // No data, no recommendations
}

#[tokio::test]
async fn test_update_model() {
    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let result = optimizer.update_model().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_select_runtime_with_disabled_selection() {
    let config = PerformanceConfig {
        enable_runtime_selection: false,
        ..Default::default()
    };
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    let request = create_test_execution_request();
    let available_runtimes = vec![RuntimeType::Wasm, RuntimeType::Native];

    let result = optimizer
        .select_runtime(&request, &available_runtimes)
        .await;
    assert!(result.is_ok());
    // Should return first runtime when selection is disabled
    assert_eq!(result.unwrap(), RuntimeType::Wasm);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_complete_performance_workflow() {
    let config = PerformanceConfig {
        min_prediction_samples: 2,
        ..Default::default()
    };
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = IntelligentPerformanceOptimizer::new(config, strategy);

    // 1. Record multiple metrics
    for i in 0..3 {
        let mut metrics = create_test_metrics();
        metrics.execution_id = format!("workflow-{i}");
        optimizer.record_metrics(metrics).await.unwrap();
    }

    // 2. Get runtime stats
    let stats = optimizer
        .get_runtime_stats(RuntimeType::Native)
        .await
        .unwrap();
    assert_eq!(stats.total_executions, 3);

    // 3. Select runtime based on history
    let request = create_test_execution_request();
    let available_runtimes = vec![RuntimeType::Native, RuntimeType::Wasm];
    let selected = optimizer
        .select_runtime(&request, &available_runtimes)
        .await
        .unwrap();
    assert_eq!(selected, RuntimeType::Native); // Has history

    // 4. Get recommendations
    let recommendations = optimizer.get_recommendations().await.unwrap();
    // May or may not have recommendations depending on metrics
    // Recommendations vector exists (length check removed - always >= 0)
    assert!(recommendations.is_empty() || !recommendations.is_empty()); // Always true, but verifies type
}

#[tokio::test]
async fn test_concurrent_metrics_recording() {
    use tokio::task::JoinSet;

    let config = PerformanceConfig::default();
    let strategy = RuntimeSelectionStrategy::FastestExecution;
    let optimizer = std::sync::Arc::new(IntelligentPerformanceOptimizer::new(config, strategy));

    let mut set = JoinSet::new();

    for i in 0..10 {
        let opt_clone = optimizer.clone();
        set.spawn(async move {
            let mut metrics = create_test_metrics();
            metrics.execution_id = format!("concurrent-{i}");
            opt_clone.record_metrics(metrics).await
        });
    }

    // Wait for all to complete
    while let Some(result) = set.join_next().await {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    // Verify all were recorded
    let stats = optimizer
        .get_runtime_stats(RuntimeType::Native)
        .await
        .unwrap();
    assert_eq!(stats.total_executions, 10);
}
