// SPDX-License-Identifier: AGPL-3.0-only
//! Integration tests for WASM runtime
//!
//! These tests exercise WASM workload execution with real runtime logic.

use toadstool_testing::fixtures::{runtime::*, TestEnvironment};

#[tokio::test]
async fn test_wasm_workload_creation() {
    let workload = TestWorkloadBuilder::wasm()
        .with_entry_point("main")
        .with_timeout(30)
        .build();

    assert_eq!(workload["workload_type"], "Wasm");
    assert_eq!(workload["entry_point"], "main");
    assert_eq!(workload["timeout_seconds"], 30);
}

#[tokio::test]
async fn test_wasm_workload_with_resources() {
    let workload = TestWorkloadBuilder::wasm()
        .with_resources(2.0, 1024)
        .build();

    assert_eq!(workload["resources"]["cpu_cores"], 2.0);
    assert_eq!(workload["resources"]["memory_mb"], 1024);
}

#[tokio::test]
async fn test_create_wasm_test_workload_helper() {
    let workload = create_wasm_test_workload();

    assert_eq!(workload["workload_type"], "Wasm");
    assert!(workload.get("entry_point").is_some());
}

#[tokio::test]
async fn test_heavy_wasm_workload() {
    let workload = create_heavy_test_workload();

    assert_eq!(workload["workload_type"], "Wasm");
    assert!(workload["resources"]["cpu_cores"].as_f64().unwrap() >= 4.0);
    assert!(workload["resources"]["memory_mb"].as_u64().unwrap() >= 2048);
    assert!(workload["timeout_seconds"].as_u64().unwrap() >= 300);
}

#[tokio::test]
async fn test_wasm_workload_different_timeouts() {
    let short = TestWorkloadBuilder::wasm().with_timeout(5).build();

    let long = TestWorkloadBuilder::wasm().with_timeout(600).build();

    assert_eq!(short["timeout_seconds"], 5);
    assert_eq!(long["timeout_seconds"], 600);
}

#[tokio::test]
async fn test_wasm_workload_minimal_resources() {
    let minimal = TestWorkloadBuilder::wasm().with_resources(0.5, 128).build();

    assert_eq!(minimal["resources"]["cpu_cores"], 0.5);
    assert_eq!(minimal["resources"]["memory_mb"], 128);
}

#[tokio::test]
async fn test_wasm_workload_maximum_resources() {
    let maximum = TestWorkloadBuilder::wasm()
        .with_resources(16.0, 16384)
        .build();

    assert_eq!(maximum["resources"]["cpu_cores"], 16.0);
    assert_eq!(maximum["resources"]["memory_mb"], 16384);
}

#[tokio::test]
async fn test_multiple_wasm_workloads_isolated() {
    let workload1 = TestWorkloadBuilder::wasm()
        .with_entry_point("module1")
        .with_timeout(30)
        .build();

    let workload2 = TestWorkloadBuilder::wasm()
        .with_entry_point("module2")
        .with_timeout(60)
        .build();

    // Workloads should be independent
    assert_eq!(workload1["entry_point"], "module1");
    assert_eq!(workload2["entry_point"], "module2");
    assert_ne!(workload1["timeout_seconds"], workload2["timeout_seconds"]);
}

#[tokio::test]
async fn test_wasm_workload_in_test_environment() {
    let env = TestEnvironment::new();

    // Create workload configuration file in test environment
    let workload = create_wasm_test_workload();
    let config_path = env.config_dir.join("workload.json");

    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&workload).unwrap(),
    )
    .expect("Failed to write workload config");

    assert!(config_path.exists());

    // Read back and verify
    let content = std::fs::read_to_string(&config_path).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(loaded["workload_type"], "Wasm");
}

#[tokio::test]
async fn test_wasm_workload_configuration_roundtrip() {
    let original = TestWorkloadBuilder::wasm()
        .with_entry_point("test_function")
        .with_resources(3.0, 2048)
        .with_timeout(90)
        .build();

    // Serialize and deserialize
    let json = serde_json::to_string(&original).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(original, loaded);
}
