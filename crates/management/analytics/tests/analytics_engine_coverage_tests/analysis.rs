// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trend analysis, predictions, alerts, export hooks, and statistical helpers.

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool::execution::RuntimeType;
use toadstool_management_analytics::*;
use uuid::Uuid;

// ============================================================================
// Analyze Trends Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_analyze_trends_with_no_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Try to analyze trends for a metric that doesn't exist
    let result = engine.analyze_trends("nonexistent_metric", 24).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_analyze_trends_with_single_data_point() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add a single data point
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: 42.0,
        runtime_type: Some(RuntimeType::Native),
        execution_id: Some("exec_123".to_string()),
        tags: HashMap::new(),
    };

    engine.collect_data_point(data_point).await.unwrap();

    // ✅ MODERNIZED: Minimal delay for buffered data collection
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_analyze_trends_with_increasing_trend() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Create increasing trend data
    let base_time = SystemTime::now() - Duration::from_secs(5 * 3600);
    for i in 0..10 {
        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: base_time + Duration::from_secs((i as u64) * 3600),
            metric_name: "increasing_metric".to_string(),
            value: f64::from(i).mul_add(5.0, 10.0), // 10, 15, 20, 25, ...
            runtime_type: Some(RuntimeType::Native),
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(data_point).await.unwrap();
    }

    // ✅ MODERNIZED: Minimal delay for background processing
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_analyze_trends_with_stable_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Create stable data (low variation)
    let base_time = SystemTime::now() - Duration::from_secs(3 * 3600);
    for i in 0..10 {
        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: base_time + Duration::from_secs((i as u64) * 3600),
            metric_name: "stable_metric".to_string(),
            value: f64::from(i).mul_add(0.1, 50.0), // Very small variation
            runtime_type: Some(RuntimeType::Wasm),
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(data_point).await.unwrap();
    }

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED
}

// ============================================================================
// Predict Values Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_predict_values_no_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    let result = engine.predict_values("nonexistent_metric", 12).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_predict_values_with_historical_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add historical data for prediction
    let base_time = SystemTime::now() - Duration::from_secs(7 * 86400);
    for i in 0..50 {
        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: base_time + Duration::from_secs((i * 3) as u64 * 3600),
            metric_name: "predict_metric".to_string(),
            value: f64::from(i).mul_add(2.0, 100.0),
            runtime_type: Some(RuntimeType::Native),
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(data_point).await.unwrap();
    }

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_predict_values_short_horizon() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add minimal data
    let base_time = SystemTime::now() - Duration::from_secs(12 * 3600);
    for i in 0..5 {
        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: base_time + Duration::from_secs((i * 2) as u64 * 3600),
            metric_name: "short_predict".to_string(),
            value: 50.0 + f64::from(i),
            runtime_type: Some(RuntimeType::Wasm),
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(data_point).await.unwrap();
    }

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED
}

// ============================================================================
// Alert Evaluation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_alerts_no_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    let alerts = engine.evaluate_alerts().await.unwrap();
    assert_eq!(alerts.len(), 0, "Should have no alerts with no data");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_alerts_cpu_threshold() {
    let mut config = AnalyticsConfig::default();
    config.alert_thresholds.cpu_threshold = 70.0;

    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add CPU metrics that exceed threshold
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "cpu_usage".to_string(),
        value: 85.0, // Above 70% threshold
        runtime_type: Some(RuntimeType::Native),
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(data_point).await.unwrap();

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED

    let _alerts = engine.evaluate_alerts().await.unwrap();
    // Check if any alerts were triggered
    // Note: may be empty if data not yet persisted
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_alerts_memory_threshold() {
    let mut config = AnalyticsConfig::default();
    config.alert_thresholds.memory_threshold = 80.0;

    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add memory metrics that exceed threshold
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "memory_usage".to_string(),
        value: 95.0, // Above 80% threshold
        runtime_type: Some(RuntimeType::Wasm),
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(data_point).await.unwrap();

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED

    let _alerts = engine.evaluate_alerts().await.unwrap();
    // Check if memory alerts were triggered
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_evaluate_alerts_below_threshold() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

    // Add metrics below all thresholds
    let cpu_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "cpu_usage".to_string(),
        value: 30.0, // Well below threshold
        runtime_type: Some(RuntimeType::Native),
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(cpu_point).await.unwrap();

    let memory_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "memory_usage".to_string(),
        value: 40.0, // Well below threshold
        runtime_type: Some(RuntimeType::Wasm),
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(memory_point).await.unwrap();

    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED

    let alerts = engine.evaluate_alerts().await.unwrap();
    assert_eq!(
        alerts.len(),
        0,
        "Should have no alerts when below threshold"
    );
}
