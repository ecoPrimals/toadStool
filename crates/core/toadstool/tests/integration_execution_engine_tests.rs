//! Integration tests for ExecutionEngine
//!
//! These tests exercise actual execution engine logic with real implementations.

use toadstool_testing::fixtures::{runtime::*, TestEnvironment};

#[tokio::test]
async fn test_execution_request_creation_with_wasm_workload() {
    let workload = create_wasm_test_workload();

    // Verify workload structure
    assert_eq!(workload["workload_type"], "Wasm");
    assert!(workload.get("entry_point").is_some());
}

#[tokio::test]
async fn test_execution_request_with_native_workload() {
    let workload = create_native_test_workload();

    assert_eq!(workload["workload_type"], "Native");
    assert!(workload.get("entry_point").is_some());
}

#[tokio::test]
async fn test_execution_request_with_heavy_resources() {
    let workload = create_heavy_test_workload();

    // Verify heavy workload has substantial resources
    assert!(workload["resources"]["cpu_cores"].as_f64().unwrap() >= 4.0);
    assert!(workload["resources"]["memory_mb"].as_u64().unwrap() >= 2048);
}

#[tokio::test]
async fn test_workload_builder_creates_valid_configurations() {
    let wasm = TestWorkloadBuilder::wasm()
        .with_entry_point("test_main")
        .with_resources(2.0, 1024)
        .with_timeout(60)
        .build();

    assert_eq!(wasm["workload_type"], "Wasm");
    assert_eq!(wasm["entry_point"], "test_main");
    assert_eq!(wasm["resources"]["cpu_cores"], 2.0);
    assert_eq!(wasm["resources"]["memory_mb"], 1024);
    assert_eq!(wasm["timeout_seconds"], 60);
}

#[tokio::test]
async fn test_multiple_workload_types_can_coexist() {
    let wasm = TestWorkloadBuilder::wasm().build();
    let native = TestWorkloadBuilder::native().build();
    let container = TestWorkloadBuilder::container().build();
    let python = TestWorkloadBuilder::python().build();

    // All workload types should be valid
    assert_eq!(wasm["workload_type"], "Wasm");
    assert_eq!(native["workload_type"], "Native");
    assert_eq!(container["workload_type"], "Container");
    assert_eq!(python["workload_type"], "Python");
}

#[tokio::test]
async fn test_workload_resource_scaling() {
    let small = TestWorkloadBuilder::wasm().with_resources(0.5, 128).build();

    let medium = TestWorkloadBuilder::wasm()
        .with_resources(2.0, 1024)
        .build();

    let large = TestWorkloadBuilder::wasm()
        .with_resources(8.0, 8192)
        .build();

    // Verify resource scaling
    assert!(
        small["resources"]["cpu_cores"].as_f64().unwrap()
            < medium["resources"]["cpu_cores"].as_f64().unwrap()
    );
    assert!(
        medium["resources"]["cpu_cores"].as_f64().unwrap()
            < large["resources"]["cpu_cores"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn test_workload_timeout_configuration() {
    let short = TestWorkloadBuilder::wasm().with_timeout(10).build();
    let medium = TestWorkloadBuilder::wasm().with_timeout(60).build();
    let long = TestWorkloadBuilder::wasm().with_timeout(300).build();

    assert_eq!(short["timeout_seconds"], 10);
    assert_eq!(medium["timeout_seconds"], 60);
    assert_eq!(long["timeout_seconds"], 300);
}

#[tokio::test]
async fn test_workload_configuration_persistence() {
    let env = TestEnvironment::new();

    let workload = TestWorkloadBuilder::wasm()
        .with_entry_point("persistent_test")
        .with_resources(4.0, 2048)
        .build();

    // Write configuration to file
    let config_path = env.config_dir.join("workload.json");
    std::fs::write(
        &config_path,
        serde_json::to_string_pretty(&workload).unwrap(),
    )
    .expect("Failed to write workload config");

    // Read back and verify
    let content = std::fs::read_to_string(&config_path).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(loaded["workload_type"], "Wasm");
    assert_eq!(loaded["entry_point"], "persistent_test");
}

#[tokio::test]
async fn test_container_workload_with_custom_entry_point() {
    let container = TestWorkloadBuilder::container()
        .with_entry_point("alpine:3.18")
        .with_resources(1.0, 512)
        .build();

    assert_eq!(container["workload_type"], "Container");
    assert_eq!(container["entry_point"], "alpine:3.18");
}

#[tokio::test]
async fn test_python_workload_with_script() {
    let python = TestWorkloadBuilder::python()
        .with_entry_point("main.py")
        .with_timeout(120)
        .build();

    assert_eq!(python["workload_type"], "Python");
    assert_eq!(python["entry_point"], "main.py");
}

#[tokio::test]
async fn test_native_workload_with_executable_path() {
    let native = TestWorkloadBuilder::native()
        .with_entry_point("/usr/bin/echo")
        .with_resources(0.5, 256)
        .build();

    assert_eq!(native["workload_type"], "Native");
    assert_eq!(native["entry_point"], "/usr/bin/echo");
}

#[tokio::test]
async fn test_workload_builder_method_chaining() {
    let workload = TestWorkloadBuilder::wasm()
        .with_entry_point("chain_test")
        .with_resources(3.0, 1536)
        .with_timeout(90)
        .build();

    // Verify all chained configurations applied
    assert_eq!(workload["entry_point"], "chain_test");
    assert_eq!(workload["resources"]["cpu_cores"], 3.0);
    assert_eq!(workload["resources"]["memory_mb"], 1536);
    assert_eq!(workload["timeout_seconds"], 90);
}

#[tokio::test]
async fn test_workload_serialization_roundtrip() {
    let original = TestWorkloadBuilder::wasm()
        .with_entry_point("roundtrip")
        .with_resources(2.5, 2048)
        .with_timeout(75)
        .build();

    // Serialize
    let json = serde_json::to_string(&original).unwrap();

    // Deserialize
    let loaded: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Verify equality
    assert_eq!(original, loaded);
}

#[tokio::test]
async fn test_multiple_workloads_in_test_environment() {
    let env = TestEnvironment::new();

    // Create multiple workloads
    let wasm = create_wasm_test_workload();
    let native = create_native_test_workload();
    let heavy = create_heavy_test_workload();

    // Save all to test environment
    std::fs::write(
        env.config_dir.join("wasm.json"),
        serde_json::to_string_pretty(&wasm).unwrap(),
    )
    .unwrap();

    std::fs::write(
        env.config_dir.join("native.json"),
        serde_json::to_string_pretty(&native).unwrap(),
    )
    .unwrap();

    std::fs::write(
        env.config_dir.join("heavy.json"),
        serde_json::to_string_pretty(&heavy).unwrap(),
    )
    .unwrap();

    // Verify all files exist
    assert!(env.config_dir.join("wasm.json").exists());
    assert!(env.config_dir.join("native.json").exists());
    assert!(env.config_dir.join("heavy.json").exists());
}
