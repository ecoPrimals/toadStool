// SPDX-License-Identifier: AGPL-3.0-only
//! Constructor and basic methods for IntegrationTestManager

use super::*;

impl IntegrationTestManager {
    /// Create a new integration test manager
    #[must_use]
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            _config: config,
            results: Arc::new(RwLock::new(Vec::new())),
            _active_tests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get test results
    pub async fn get_results(&self) -> Vec<IntegrationTestResult> {
        self.results.read().await.clone()
    }

    /// Generate test report
    pub async fn generate_report(&self) -> String {
        let results = self.get_results().await;
        let total_tests = results.len();
        let passed_tests = results
            .iter()
            .filter(|r| r.status == TestStatus::Passed)
            .count();
        let failed_tests = results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count();
        let skipped_tests = results
            .iter()
            .filter(|r| r.status == TestStatus::Skipped)
            .count();

        format!(
            "Integration Test Report\n\
             ======================\n\
             Total Tests: {}\n\
             Passed: {}\n\
             Failed: {}\n\
            Skipped: {}\n\
            Success Rate: {:.2}%\n\n\
            Test Details:\n\
            {}",
            total_tests,
            passed_tests,
            failed_tests,
            skipped_tests,
            (passed_tests as f64 / total_tests as f64) * 100.0,
            results
                .iter()
                .map(|r| format!("- {}: {:?} ({})", r.test_name, r.status, r.message))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Run comprehensive integration tests for OS-layer, biomeOS, and security features
    pub async fn run_comprehensive_tests(&self) -> Result<Vec<IntegrationTestResult>> {
        info!("Starting comprehensive integration tests");

        let mut results = Vec::new();

        // Test 1: OS-layer compatibility across platforms
        results.push(self.test_os_layer_compatibility().await?);

        // Test 2: biomeOS integration with multiple workloads
        results.push(self.test_biomeos_integration().await?);

        // Test 3: Security sandboxing with resource limits
        results.push(self.test_security_sandboxing().await?);

        // Test 4: Cross-component integration
        results.push(self.test_cross_component_integration().await?);

        // Test 5: Performance under load
        results.push(self.test_performance_under_load().await?);

        // Test 6: Large biome performance and scalability
        results.push(self.test_large_biome_performance().await?);

        // Test 7: Error handling and recovery
        results.push(self.test_error_handling_recovery().await?);

        // Test 8: Concurrent execution safety
        results.push(self.test_concurrent_execution_safety().await?);

        // Test 9: Resource cleanup and lifecycle management
        results.push(self.test_resource_cleanup_lifecycle().await?);

        // Store results
        {
            let mut stored_results = self.results.write().await;
            stored_results.extend(results.clone());
        }

        info!(
            "Comprehensive integration tests completed: {} results",
            results.len()
        );
        Ok(results)
    }
}

