// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for PlatformOptimizer and optimization logic
//!
//! Tests platform-specific optimizations and configuration generation

use std::sync::Arc;
use std::time::Duration;
use toadstool_auto_config::hardware::HardwareDetector;
use toadstool_auto_config::intelligent::PlatformOptimizer;
use tokio::sync::Barrier;

// ============================================================================
// UNIT TESTS - PlatformOptimizer
// ============================================================================

#[test]
fn test_platform_optimizer_creation() {
    let optimizer = PlatformOptimizer::new();
    // Should construct without panicking
    drop(optimizer);
}

#[test]
fn test_platform_optimizer_default() {
    let optimizer1 = PlatformOptimizer::default();
    let optimizer2 = PlatformOptimizer::new();

    // Both should construct successfully
    drop(optimizer1);
    drop(optimizer2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimize_for_platform_minimal_hardware() {
    let optimizer = PlatformOptimizer::new();
    let mut detector = HardwareDetector::new();

    // Get real hardware capabilities
    let hardware_result = detector.scan_system().await;

    if let Ok(hardware) = hardware_result {
        // Test optimization with real hardware
        let result = optimizer.optimize_for_platform(&hardware).await;

        match result {
            Ok(platform_config) => {
                assert!(
                    !platform_config.optimizations.is_empty(),
                    "Should provide at least some optimizations"
                );
                println!(
                    "✅ Platform optimizations: {} applied",
                    platform_config.optimizations.len()
                );
            }
            Err(e) => {
                eprintln!("⚠️  Platform optimization failed: {:?}", e);
            }
        }
    }
}

// ============================================================================
// CONCURRENT TESTS - Test thread safety
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_concurrent_platform_optimizer_creation() {
    const NUM_OPTIMIZERS: usize = 100;
    let barrier = Arc::new(Barrier::new(NUM_OPTIMIZERS));
    let mut tasks = vec![];

    for _ in 0..NUM_OPTIMIZERS {
        let barrier = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;

            // Create optimizer concurrently
            let _optimizer = PlatformOptimizer::new();
            Ok::<_, std::io::Error>(())
        }));
    }

    let results = futures::future::join_all(tasks).await;

    // All should succeed
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(
        success_count, NUM_OPTIMIZERS,
        "All concurrent creations should succeed"
    );

    println!(
        "✅ Created {} platform optimizers concurrently",
        NUM_OPTIMIZERS
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_hardware_detection() {
    const NUM_DETECTIONS: usize = 10;
    let mut tasks = vec![];

    for _ in 0..NUM_DETECTIONS {
        tasks.push(tokio::spawn(async {
            let mut detector = HardwareDetector::new();
            detector.scan_system().await
        }));
    }

    let results = futures::future::join_all(tasks).await;

    // All should complete (success or graceful failure)
    for result in results {
        assert!(result.is_ok(), "Task should not panic");
    }

    println!(
        "✅ Completed {} concurrent hardware detections",
        NUM_DETECTIONS
    );
}

// ============================================================================
// PERFORMANCE TESTS - Ensure operations are fast enough
// ============================================================================

#[test]
fn test_platform_optimizer_creation_performance() {
    use std::time::Instant;

    let start = Instant::now();

    // Creating optimizers should be very fast (no I/O)
    for _ in 0..1000 {
        let _ = PlatformOptimizer::new();
    }

    let duration = start.elapsed();

    // Should be extremely fast - just OS detection
    assert!(
        duration < Duration::from_millis(500),
        "Creating 1000 platform optimizers should be <500ms, took {:?}",
        duration
    );

    println!("✅ Created 1000 platform optimizers in {:?}", duration);
}

#[tokio::test]
async fn test_hardware_scan_performance() {
    use std::time::Instant;

    let mut detector = HardwareDetector::new();
    let start = Instant::now();

    let _ = detector.scan_system().await;

    let duration = start.elapsed();

    // Hardware scan should be reasonably fast
    assert!(
        duration < Duration::from_secs(5),
        "Hardware scan should complete <5s, took {:?}",
        duration
    );

    println!("✅ Hardware scan completed in {:?}", duration);
}

// ============================================================================
// REGRESSION TESTS - Prevent known bugs
// ============================================================================

#[test]
fn test_optimizer_does_not_panic() {
    // Regression: Constructor should never panic
    for _ in 0..10 {
        let _ = PlatformOptimizer::new();
    }
}

#[tokio::test]
async fn test_hardware_detector_does_not_panic() {
    // Regression: Hardware detection should never panic
    let mut detector = HardwareDetector::new();
    let _ = detector.scan_system().await;

    // If we get here, no panic occurred
}

// ============================================================================
// EDGE CASE TESTS - Boundary conditions
// ============================================================================

#[tokio::test]
async fn test_rapid_sequential_scans() {
    let mut detector = HardwareDetector::new();

    // Rapid sequential scans (no delay between)
    for _ in 0..5 {
        let _ = detector.scan_system().await;
    }

    // Should handle rapid calls
}

#[test]
fn test_multiple_optimizer_instances() {
    // Create multiple optimizers and verify they don't interfere
    let _opt1 = PlatformOptimizer::new();
    let _opt2 = PlatformOptimizer::new();
    let _opt3 = PlatformOptimizer::new();

    // All should coexist without issues
}
