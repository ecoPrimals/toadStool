// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_integration_test_manager() {
    let config = IntegrationTestConfig::default();
    let manager = IntegrationTestManager::new(config.clone());

    // Test manager creation and config values
    assert_eq!(config.max_concurrent_tests, 10);
    assert_eq!(config.default_timeout, Duration::from_mins(5));

    // Test initial state
    let results = manager.get_results().await;
    assert!(results.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_comprehensive_integration_tests() {
    let config = IntegrationTestConfig {
        max_concurrent_tests: 5,
        default_timeout: Duration::from_mins(1),
        collect_metrics: true,
        save_artifacts: false,
        artifact_dir: std::path::PathBuf::from("./test_artifacts"),
        cleanup_on_success: true,
        cleanup_on_failure: false,
    };

    let manager = IntegrationTestManager::new(config);

    // Run comprehensive tests
    let results = manager
        .run_comprehensive_tests()
        .await
        .expect("Comprehensive test run should succeed");

    // Verify results
    assert_eq!(results.len(), 9); // Updated to 9 for the new test
    assert!(
        results
            .iter()
            .any(|r| r.test_name == "os_layer_compatibility")
    );
    assert!(results.iter().any(|r| r.test_name == "biomeos_integration"));
    assert!(results.iter().any(|r| r.test_name == "security_sandboxing"));
    assert!(
        results
            .iter()
            .any(|r| r.test_name == "cross_component_integration")
    );
    assert!(
        results
            .iter()
            .any(|r| r.test_name == "performance_under_load")
    );
    assert!(
        results
            .iter()
            .any(|r| r.test_name == "large_biome_performance")
    );

    // Check that all tests passed
    let passed_count = results
        .iter()
        .filter(|r| r.status == TestStatus::Passed)
        .count();
    assert_eq!(passed_count, 9); // Updated to 9 for the new test
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_collector() {
    let mut collector = MetricsCollector::new();

    // Record some metrics
    collector.record_metric("test_metric", 42.0);
    collector.record_metric("another_metric", std::f64::consts::PI);

    // Finalize and check
    let metrics = collector.finalize();
    assert_eq!(metrics.custom_metrics.get("test_metric"), Some(&42.0));
    assert_eq!(
        metrics.custom_metrics.get("another_metric"),
        Some(&std::f64::consts::PI)
    );
    assert!(metrics.custom_metrics.contains_key("duration_ms"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_report_generation() {
    let config = IntegrationTestConfig::default();
    let manager = IntegrationTestManager::new(config);

    // Add some test results
    {
        let mut results = manager.results.write().unwrap_or_else(|e| e.into_inner());
        results.push(IntegrationTestResult {
            test_name: "test1".to_string(),
            status: TestStatus::Passed,
            duration: Duration::from_millis(100),
            message: "Test passed".to_string(),
            details: None,
        });
        results.push(IntegrationTestResult {
            test_name: "test2".to_string(),
            status: TestStatus::Failed,
            duration: Duration::from_millis(200),
            message: "Test failed".to_string(),
            details: None,
        });
    }

    // Generate report
    let report = manager.generate_report().await;
    assert!(report.contains("Total Tests: 2"));
    assert!(report.contains("Passed: 1"));
    assert!(report.contains("Failed: 1"));
    assert!(report.contains("Success Rate: 50.00%"));
}
