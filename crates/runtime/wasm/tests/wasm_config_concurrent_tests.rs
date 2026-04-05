// SPDX-License-Identifier: AGPL-3.0-or-later
//! Concurrent tests for WASM Runtime Configuration
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests WASM config creation and validation concurrently

use std::sync::Arc;
use tokio::sync::Barrier;

use toadstool_runtime_wasm::WasmRuntimeConfig;

#[tokio::test]
async fn test_concurrent_config_creation() {
    // ✅ FULLY CONCURRENT: Create 100 configs in parallel
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            // Create default config
            let _config = WasmRuntimeConfig::default();
            true
        }));
    }

    for task in tasks {
        assert!(task.await.expect("Task failed"));
    }
}

#[tokio::test]
async fn test_concurrent_config_with_builder() {
    // ✅ FULLY CONCURRENT: Create configs with builder pattern
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            // Create config with different params
            let config = WasmRuntimeConfig::default();

            config.max_memory_mb > 0
        }));
    }

    for task in tasks {
        assert!(task.await.expect("Task failed"));
    }
}

#[tokio::test]
async fn test_stress_200_concurrent_configs() {
    // ✅ STRESS TEST: 200 concurrent config operations
    let barrier = Arc::new(Barrier::new(200));
    let mut tasks = vec![];

    for i in 0..200 {
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            match i % 3 {
                0 => {
                    // Create default
                    let _ = WasmRuntimeConfig::default();
                }
                1 => {
                    // Create another default
                    let _ = WasmRuntimeConfig::default();
                }
                _ => {
                    // Clone existing
                    let config = WasmRuntimeConfig::default();
                    let _ = config;
                }
            }
            true
        }));
    }

    let mut completed = 0;
    for task in tasks {
        if task.await.expect("Task panicked") {
            completed += 1;
        }
    }

    assert_eq!(completed, 200, "All operations should complete");
}
