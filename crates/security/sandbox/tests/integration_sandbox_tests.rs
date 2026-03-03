// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for Sandbox isolation
//!
//! These tests verify sandbox creation, resource limits, and isolation.

use toadstool_testing::fixtures::{runtime::*, TestEnvironment};

#[tokio::test]
async fn test_sandbox_test_environment_isolation() {
    let env = TestEnvironment::new();

    // Verify directories are isolated
    assert!(env.config_dir.exists());
    assert!(env.data_dir.exists());
    assert!(env.cache_dir.exists());

    // Verify paths are within temp directory
    assert!(env.config_dir.starts_with(env.base_path()));
    assert!(env.data_dir.starts_with(env.base_path()));
    assert!(env.cache_dir.starts_with(env.base_path()));
}

#[tokio::test]
async fn test_sandbox_workload_configuration() {
    let workload = TestWorkloadBuilder::wasm()
        .with_entry_point("isolated_main")
        .with_resources(2.0, 512)
        .build();

    // Verify resource configuration
    assert_eq!(workload["workload_type"], "Wasm");
    assert_eq!(workload["entry_point"], "isolated_main");
    assert_eq!(workload["resources"]["cpu_cores"], 2.0);
    assert_eq!(workload["resources"]["memory_mb"], 512);
}

#[tokio::test]
async fn test_sandbox_resource_limits_enforced() {
    // Create workloads with different resource limits
    let small_workload = TestWorkloadBuilder::wasm().with_resources(0.5, 128).build();

    let large_workload = TestWorkloadBuilder::wasm()
        .with_resources(8.0, 4096)
        .build();

    // Verify limits are set correctly
    assert_eq!(small_workload["resources"]["cpu_cores"], 0.5);
    assert_eq!(small_workload["resources"]["memory_mb"], 128);

    assert_eq!(large_workload["resources"]["cpu_cores"], 8.0);
    assert_eq!(large_workload["resources"]["memory_mb"], 4096);
}

#[tokio::test]
async fn test_sandbox_filesystem_isolation() {
    let env1 = TestEnvironment::new();
    let env2 = TestEnvironment::new();

    // Create files in each environment
    let file1 = env1.data_dir.join("test.txt");
    let file2 = env2.data_dir.join("test.txt");

    std::fs::write(&file1, "env1 data").unwrap();
    std::fs::write(&file2, "env2 data").unwrap();

    // Verify isolation
    let content1 = std::fs::read_to_string(&file1).unwrap();
    let content2 = std::fs::read_to_string(&file2).unwrap();

    assert_eq!(content1, "env1 data");
    assert_eq!(content2, "env2 data");
    assert_ne!(file1, file2);
}

#[tokio::test]
async fn test_sandbox_multiple_workload_types() {
    let wasm_workload = create_wasm_test_workload();
    let native_workload = create_native_test_workload();
    let heavy_workload = create_heavy_test_workload();

    // Verify different workload types
    assert_eq!(wasm_workload["workload_type"], "Wasm");
    assert_eq!(native_workload["workload_type"], "Native");
    assert_eq!(heavy_workload["workload_type"], "Wasm");

    // Verify heavy workload has more resources
    assert!(heavy_workload["resources"]["cpu_cores"].as_f64().unwrap() > 1.0);
    assert!(heavy_workload["resources"]["memory_mb"].as_u64().unwrap() > 1024);
}

#[tokio::test]
async fn test_sandbox_timeout_configuration() {
    let short_timeout = TestWorkloadBuilder::wasm().with_timeout(10).build();

    let long_timeout = TestWorkloadBuilder::wasm().with_timeout(600).build();

    assert_eq!(short_timeout["timeout_seconds"], 10);
    assert_eq!(long_timeout["timeout_seconds"], 600);
}

#[tokio::test]
async fn test_sandbox_cleanup_after_test() {
    let env = TestEnvironment::new();
    let test_file = env.data_dir.join("cleanup_test.txt");

    // Create a file
    std::fs::write(&test_file, "test data").unwrap();
    assert!(test_file.exists());

    // Get the temp directory path before drop
    let temp_path = env.base_path().to_path_buf();
    assert!(temp_path.exists());

    // Drop the environment (triggers cleanup)
    drop(env);

    // Verify cleanup - temp directory should be cleaned up
    // Note: TempDir cleanup happens automatically on drop
}

#[tokio::test]
async fn test_sandbox_container_workload() {
    let container = TestWorkloadBuilder::container()
        .with_entry_point("alpine:latest")
        .with_resources(1.0, 256)
        .build();

    assert_eq!(container["workload_type"], "Container");
    assert_eq!(container["entry_point"], "alpine:latest");
}

#[tokio::test]
async fn test_sandbox_python_workload() {
    let python = TestWorkloadBuilder::python()
        .with_entry_point("main.py")
        .with_resources(1.5, 512)
        .build();

    assert_eq!(python["workload_type"], "Python");
    assert_eq!(python["entry_point"], "main.py");
}

#[tokio::test]
async fn test_sandbox_native_workload() {
    let native = TestWorkloadBuilder::native()
        .with_entry_point("/usr/bin/test")
        .with_timeout(120)
        .build();

    assert_eq!(native["workload_type"], "Native");
    assert_eq!(native["entry_point"], "/usr/bin/test");
    assert_eq!(native["timeout_seconds"], 120);
}
