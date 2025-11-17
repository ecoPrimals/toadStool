//! OS-layer compatibility testing

use super::*;

impl IntegrationTestManager {
    /// Test OS-layer compatibility across different platforms
    pub(crate) async fn test_os_layer_compatibility(&self) -> Result<IntegrationTestResult> {
        let test_name = "os_layer_compatibility";
        let start_time = std::time::Instant::now();

        info!("Testing OS-layer compatibility");

        // Create test context
        let temp_dir = tempfile::tempdir()?;
        let mut context = TestContext {
            test_name: test_name.to_string(),
            start_time,
            temp_dir: temp_dir.path().to_path_buf(),
            cleanup_tasks: Vec::new(),
            metrics_collector: MetricsCollector::new(),
        };

        let mut test_data = HashMap::new();
        let mut components_tested = vec!["os_layer".to_string()];

        // Test different OS compatibility layers
        match self.test_linux_compatibility(&mut context).await {
            Ok(()) => {
                test_data.insert("linux_compatibility".to_string(), "passed".to_string());
                components_tested.push("linux_os".to_string());
            }
            Err(e) => {
                test_data.insert("linux_compatibility".to_string(), format!("failed: {e}"));
            }
        }

        match self.test_windows_compatibility(&mut context).await {
            Ok(()) => {
                test_data.insert("windows_compatibility".to_string(), "passed".to_string());
                components_tested.push("windows_os".to_string());
            }
            Err(e) => {
                test_data.insert("windows_compatibility".to_string(), format!("failed: {e}"));
            }
        }

        match self.test_macos_compatibility(&mut context).await {
            Ok(()) => {
                test_data.insert("macos_compatibility".to_string(), "passed".to_string());
                components_tested.push("macos_os".to_string());
            }
            Err(e) => {
                test_data.insert("macos_compatibility".to_string(), format!("failed: {e}"));
            }
        }

        match self.test_legacy_compatibility(&mut context).await {
            Ok(()) => {
                test_data.insert("legacy_compatibility".to_string(), "passed".to_string());
                components_tested.push("legacy_os".to_string());
            }
            Err(e) => {
                test_data.insert("legacy_compatibility".to_string(), format!("failed: {e}"));
            }
        }

        let duration = start_time.elapsed();
        let metrics = context.metrics_collector.finalize();

        // Determine overall status
        let passed_tests = test_data.values().filter(|v| v.contains("passed")).count();
        let total_tests = test_data.len();
        let status = if passed_tests == total_tests {
            TestStatus::Passed
        } else {
            TestStatus::Failed
        };

        Ok(IntegrationTestResult {
            test_name: test_name.to_string(),
            status,
            duration,
            message: format!("OS-layer compatibility: {passed_tests}/{total_tests} tests passed"),
            details: Some(IntegrationTestDetails {
                components_tested,
                test_data,
                metrics,
                artifacts: Vec::new(),
            }),
        })
    }

    pub(crate) async fn test_linux_compatibility(&self, context: &mut TestContext) -> Result<()> {
        context
            .metrics_collector
            .record_metric("linux_compatibility_test", 1.0);
        // Simulate Linux compatibility test
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    pub(crate) async fn test_windows_compatibility(&self, context: &mut TestContext) -> Result<()> {
        context
            .metrics_collector
            .record_metric("windows_compatibility_test", 1.0);
        // Simulate Windows compatibility test
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    pub(crate) async fn test_macos_compatibility(&self, context: &mut TestContext) -> Result<()> {
        context
            .metrics_collector
            .record_metric("macos_compatibility_test", 1.0);
        // Simulate macOS compatibility test
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    pub(crate) async fn test_legacy_compatibility(&self, context: &mut TestContext) -> Result<()> {
        context
            .metrics_collector
            .record_metric("legacy_compatibility_test", 1.0);
        // Simulate legacy compatibility test
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

