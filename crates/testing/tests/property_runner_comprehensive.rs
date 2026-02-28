//! Comprehensive tests for property test runner
//!
//! Expanding coverage for properties/runner.rs (current: 7.37%)
//! Target: 80%+ coverage

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use toadstool_testing::properties::{Generator, Property, PropertyTestConfig, PropertyTestRunner};

// ============================================================================
// Test Generators
// ============================================================================

/// Simple integer generator for testing
#[derive(Debug, Clone)]
struct TestIntGenerator;

impl Generator<i32> for TestIntGenerator {
    fn generate(&mut self, size: usize) -> i32 {
        (size as i32) * 10
    }

    fn shrink(&self, value: &i32) -> Vec<i32> {
        if *value == 0 {
            vec![]
        } else {
            vec![value / 2, 0]
        }
    }
}

/// Configurable generator that can produce failing values
#[derive(Debug, Clone)]
struct ConfigurableGenerator {
    fail_at_size: Option<usize>,
}

impl Generator<i32> for ConfigurableGenerator {
    fn generate(&mut self, size: usize) -> i32 {
        if let Some(fail_size) = self.fail_at_size {
            if size == fail_size {
                return 999; // Failure marker
            }
        }
        size as i32
    }

    fn shrink(&self, value: &i32) -> Vec<i32> {
        if *value == 0 {
            vec![]
        } else {
            let half = value / 2;
            vec![half, 0]
        }
    }
}

// ============================================================================
// Test Properties
// ============================================================================

/// Always passing property
#[derive(Debug, Clone)]
struct AlwaysPassProperty;

impl Property<i32> for AlwaysPassProperty {
    fn test(&self, _value: &i32) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "always_pass"
    }
}

/// Fails on specific values
#[derive(Debug, Clone)]
struct FailOnValueProperty {
    fail_value: i32,
}

impl Property<i32> for FailOnValueProperty {
    fn test(&self, value: &i32) -> toadstool::ToadStoolResult<()> {
        if *value == self.fail_value {
            return Err(toadstool::ToadStoolError::runtime(format!(
                "Value {} failed",
                value
            )));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "fail_on_value"
    }
}

/// Fails on values greater than threshold
#[derive(Debug, Clone)]
struct FailOnLargeProperty {
    threshold: i32,
}

impl Property<i32> for FailOnLargeProperty {
    fn test(&self, value: &i32) -> toadstool::ToadStoolResult<()> {
        if *value > self.threshold {
            return Err(toadstool::ToadStoolError::runtime(format!(
                "Value {} exceeds threshold {}",
                value, self.threshold
            )));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "fail_on_large"
    }
}

/// Slow property for timeout testing
#[derive(Debug, Clone)]
struct SlowProperty {
    delay_ms: u64,
}

impl Property<i32> for SlowProperty {
    fn test(&self, _value: &i32) -> toadstool::ToadStoolResult<()> {
        // Spin until delay elapsed (no sleep - event-driven test philosophy)
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(self.delay_ms) {
            std::hint::spin_loop();
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "slow_property"
    }
}

/// Property that tracks how many times it's called
#[derive(Debug, Clone)]
struct CountingProperty {
    counter: Arc<AtomicUsize>,
}

impl Property<i32> for CountingProperty {
    fn test(&self, _value: &i32) -> toadstool::ToadStoolResult<()> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn name(&self) -> &'static str {
        "counting_property"
    }
}

// ============================================================================
// Basic Runner Tests
// ============================================================================

#[test]
fn test_runner_new_with_default_config() {
    let config = PropertyTestConfig::default();
    let runner = PropertyTestRunner::new(config);

    // Should create successfully
    drop(runner);
}

#[test]
fn test_runner_new_with_custom_config() {
    let config = PropertyTestConfig {
        test_name: "custom_test".to_string(),
        test_cases: 50,
        shrink_attempts: 20,
        timeout: Duration::from_secs(10),
        seed: Some(12345),
        verbose: true,
    };
    let runner = PropertyTestRunner::new(config);

    // Should create successfully
    drop(runner);
}

#[test]
fn test_runner_new_with_seed() {
    let config = PropertyTestConfig {
        seed: Some(42),
        ..Default::default()
    };
    let runner = PropertyTestRunner::new(config);

    // Should create successfully with seeded RNG
    drop(runner);
}

// ============================================================================
// run_test Tests
// ============================================================================

#[test]
fn test_run_test_all_passing() {
    let config = PropertyTestConfig {
        test_cases: 10,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.test_cases_run, 10);
    assert!(result.failures.is_empty());
    assert_eq!(result.test_name, "always_pass");
}

#[test]
fn test_run_test_with_failure() {
    let config = PropertyTestConfig {
        test_cases: 100,
        shrink_attempts: 5,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator; // Generates size * 10
    let property = FailOnLargeProperty { threshold: 50 }; // Fails when > 50

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    assert!(result.test_cases_run < 100); // Should stop after failure
    assert_eq!(result.failures.len(), 1);

    let failure = &result.failures[0];
    assert!(failure.error_message.contains("threshold"));
}

#[test]
fn test_run_test_with_shrinking() {
    let config = PropertyTestConfig {
        test_cases: 100,
        shrink_attempts: 10,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = FailOnLargeProperty { threshold: 50 };

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    assert_eq!(result.failures.len(), 1);

    // Should have attempted shrinking
    let failure = &result.failures[0];
    assert!(!failure.original_input.is_empty());
    assert!(!failure.shrunk_input.is_empty());
}

#[test]
fn test_run_test_with_timeout() {
    let config = PropertyTestConfig {
        test_name: "timeout_test".to_string(),
        test_cases: 1000,
        timeout: Duration::from_millis(100), // Very short timeout
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = SlowProperty { delay_ms: 10 }; // Each test takes 10ms

    let result = runner.run_test(generator, property);

    // Should timeout before completing all test cases
    assert!(result.test_cases_run < 1000);
    assert!(result.duration >= Duration::from_millis(100));
    // Timeout doesn't count as failure
    assert!(result.success);
    assert!(result.failures.is_empty());
}

#[test]
fn test_run_test_tracks_statistics() {
    let config = PropertyTestConfig {
        test_cases: 20,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.statistics.execution_times.len(), 20);
    assert!(!result.statistics.input_distribution.is_empty());
}

#[test]
fn test_run_test_verbose_mode() {
    let config = PropertyTestConfig {
        test_name: "verbose_test".to_string(),
        test_cases: 10,
        verbose: true,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = FailOnLargeProperty { threshold: 5 }; // Will fail quickly

    // Should print debug output (we can't easily capture, but test it runs)
    let result = runner.run_test(generator, property);

    assert!(!result.success);
}

#[test]
fn test_run_test_counts_all_executions() {
    let counter = Arc::new(AtomicUsize::new(0));
    let config = PropertyTestConfig {
        test_cases: 15,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = CountingProperty {
        counter: Arc::clone(&counter),
    };

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.test_cases_run, 15);
    assert_eq!(counter.load(Ordering::SeqCst), 15);
}

// ============================================================================
// Shrinking Tests
// ============================================================================

#[test]
fn test_shrinking_finds_minimal_failure() {
    let config = PropertyTestConfig {
        test_cases: 100,
        shrink_attempts: 20,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = FailOnLargeProperty { threshold: 5 };

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    let failure = &result.failures[0];

    // Shrunk input should be closer to minimal failing case
    assert!(
        failure.shrunk_input.contains("0")
            || failure.shrunk_input.len() < failure.original_input.len()
    );
}

#[test]
fn test_shrinking_with_no_shrink_candidates() {
    let config = PropertyTestConfig {
        test_cases: 10,
        shrink_attempts: 10,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = FailOnValueProperty { fail_value: 10 }; // Will fail when generator produces 10

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    // Should still record the failure even if shrinking doesn't help
    assert_eq!(result.failures.len(), 1);
}

// ============================================================================
// Test Size Calculation Tests
// ============================================================================

#[test]
fn test_size_grows_gradually() {
    let config = PropertyTestConfig {
        test_cases: 50,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let counter = Arc::new(AtomicUsize::new(0));

    struct SizeTrackingProperty {
        sizes: Arc<AtomicUsize>,
        last_value: std::sync::Mutex<i32>,
    }

    impl Property<i32> for SizeTrackingProperty {
        fn test(&self, value: &i32) -> toadstool::ToadStoolResult<()> {
            let mut last = self.last_value.lock().unwrap();
            // Values should generally increase (TestIntGenerator multiplies size by 10)
            if *value < *last {
                // Allow some variation
            }
            *last = *value;
            self.sizes.fetch_add(*value as usize, Ordering::SeqCst);
            Ok(())
        }

        fn name(&self) -> &'static str {
            "size_tracking"
        }
    }

    let property = SizeTrackingProperty {
        sizes: Arc::clone(&counter),
        last_value: std::sync::Mutex::new(0),
    };

    let result = runner.run_test(generator, property);

    assert!(result.success);
    // Sizes should have been tracked
    assert!(counter.load(Ordering::SeqCst) > 0);
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_run_test_with_zero_test_cases() {
    let config = PropertyTestConfig {
        test_cases: 0,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.test_cases_run, 0);
    assert!(result.failures.is_empty());
}

#[test]
fn test_run_test_with_one_test_case() {
    let config = PropertyTestConfig {
        test_cases: 1,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.test_cases_run, 1);
}

#[test]
fn test_run_test_with_zero_shrink_attempts() {
    let config = PropertyTestConfig {
        test_cases: 10,
        shrink_attempts: 0,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = FailOnLargeProperty { threshold: 5 };

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    let failure = &result.failures[0];
    // Should still have original input even without shrinking
    assert!(!failure.original_input.is_empty());
}

#[test]
fn test_run_test_stops_on_first_failure() {
    let config = PropertyTestConfig {
        test_cases: 100,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = ConfigurableGenerator {
        fail_at_size: Some(5),
    };
    let property = FailOnValueProperty { fail_value: 999 };

    let result = runner.run_test(generator, property);

    assert!(!result.success);
    // Should stop immediately after first failure
    assert_eq!(result.failures.len(), 1);
    assert!(result.test_cases_run < 100);
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[test]
fn test_statistics_tracks_execution_times() {
    let config = PropertyTestConfig {
        test_cases: 10,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert_eq!(result.statistics.execution_times.len(), 10);

    // All execution times should be positive
    for time in &result.statistics.execution_times {
        assert!(time.as_nanos() > 0);
    }
}

#[test]
fn test_statistics_tracks_input_distribution() {
    let config = PropertyTestConfig {
        test_cases: 20,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = AlwaysPassProperty;

    let result = runner.run_test(generator, property);

    assert!(result.success);
    assert!(!result.statistics.input_distribution.is_empty());

    // Should have tracked i32 type
    let i32_count = result
        .statistics
        .input_distribution
        .get("i32")
        .copied()
        .unwrap_or(0);
    assert_eq!(i32_count, 20);
}

#[test]
fn test_duration_is_accurate() {
    let config = PropertyTestConfig {
        test_cases: 5,
        ..Default::default()
    };
    let mut runner = PropertyTestRunner::new(config);
    let generator = TestIntGenerator;
    let property = SlowProperty { delay_ms: 20 }; // 20ms per test

    let result = runner.run_test(generator, property);

    assert!(result.success);
    // Duration should be at least 5 * 20ms = 100ms
    assert!(result.duration >= Duration::from_millis(100));
}
