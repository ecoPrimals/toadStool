// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for Resource Management
//!
//! These tests exercise resource allocation, monitoring, and limits.

use toadstool_testing::fixtures::{runtime::*, TestEnvironment};

#[tokio::test]
async fn test_resource_requirements_in_workloads() {
    let workload = TestWorkloadBuilder::wasm()
        .with_resources(4.0, 4096)
        .build();

    let resources = &workload["resources"];
    assert_eq!(resources["cpu_cores"], 4.0);
    assert_eq!(resources["memory_mb"], 4096);
}

#[tokio::test]
async fn test_minimal_resource_allocation() {
    let minimal = TestWorkloadBuilder::wasm().with_resources(0.25, 64).build();

    assert_eq!(minimal["resources"]["cpu_cores"], 0.25);
    assert_eq!(minimal["resources"]["memory_mb"], 64);
}

#[tokio::test]
async fn test_maximum_resource_allocation() {
    let maximum = TestWorkloadBuilder::wasm()
        .with_resources(32.0, 32768)
        .build();

    assert_eq!(maximum["resources"]["cpu_cores"], 32.0);
    assert_eq!(maximum["resources"]["memory_mb"], 32768);
}

#[tokio::test]
async fn test_resource_requirements_for_different_workload_types() {
    let wasm = TestWorkloadBuilder::wasm().with_resources(1.0, 512).build();

    let native = TestWorkloadBuilder::native()
        .with_resources(2.0, 1024)
        .build();

    let container = TestWorkloadBuilder::container()
        .with_resources(4.0, 2048)
        .build();

    // Each workload type can have different resource requirements
    assert_eq!(wasm["resources"]["cpu_cores"], 1.0);
    assert_eq!(native["resources"]["cpu_cores"], 2.0);
    assert_eq!(container["resources"]["cpu_cores"], 4.0);
}

#[tokio::test]
async fn test_heavy_workload_has_substantial_resources() {
    let heavy = create_heavy_test_workload();

    let cpu = heavy["resources"]["cpu_cores"].as_f64().unwrap();
    let memory = heavy["resources"]["memory_mb"].as_u64().unwrap();

    // Heavy workloads should have significant resources
    assert!(
        cpu >= 4.0,
        "Heavy workload should have at least 4 CPU cores"
    );
    assert!(
        memory >= 2048,
        "Heavy workload should have at least 2GB RAM"
    );
}

#[tokio::test]
async fn test_resource_scaling_across_workload_sizes() {
    let small = TestWorkloadBuilder::wasm().with_resources(0.5, 256).build();
    let medium = TestWorkloadBuilder::wasm()
        .with_resources(2.0, 1024)
        .build();
    let large = TestWorkloadBuilder::wasm()
        .with_resources(8.0, 8192)
        .build();

    let small_cpu = small["resources"]["cpu_cores"].as_f64().unwrap();
    let medium_cpu = medium["resources"]["cpu_cores"].as_f64().unwrap();
    let large_cpu = large["resources"]["cpu_cores"].as_f64().unwrap();

    // Verify proper scaling
    assert!(small_cpu < medium_cpu);
    assert!(medium_cpu < large_cpu);
}

#[tokio::test]
async fn test_resource_configuration_persistence() {
    let env = TestEnvironment::new();

    let workload = TestWorkloadBuilder::wasm()
        .with_resources(6.0, 6144)
        .build();

    let path = env.config_dir.join("resources.json");
    std::fs::write(&path, serde_json::to_string_pretty(&workload).unwrap()).unwrap();

    let loaded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(loaded["resources"]["cpu_cores"], 6.0);
    assert_eq!(loaded["resources"]["memory_mb"], 6144);
}

#[tokio::test]
async fn test_fractional_cpu_allocation() {
    let fractional = TestWorkloadBuilder::wasm()
        .with_resources(0.75, 512)
        .build();

    assert_eq!(fractional["resources"]["cpu_cores"], 0.75);
}

#[tokio::test]
async fn test_memory_allocation_in_multiples_of_64mb() {
    let allocations = vec![64, 128, 256, 512, 1024, 2048, 4096, 8192];

    for memory in allocations {
        let workload = TestWorkloadBuilder::wasm()
            .with_resources(1.0, memory)
            .build();

        assert_eq!(workload["resources"]["memory_mb"], memory);
    }
}

#[tokio::test]
async fn test_resource_requirements_independence() {
    let high_cpu_low_mem = TestWorkloadBuilder::wasm()
        .with_resources(16.0, 512)
        .build();

    let low_cpu_high_mem = TestWorkloadBuilder::wasm()
        .with_resources(0.5, 16384)
        .build();

    // CPU and memory can be configured independently
    assert_eq!(high_cpu_low_mem["resources"]["cpu_cores"], 16.0);
    assert_eq!(high_cpu_low_mem["resources"]["memory_mb"], 512);

    assert_eq!(low_cpu_high_mem["resources"]["cpu_cores"], 0.5);
    assert_eq!(low_cpu_high_mem["resources"]["memory_mb"], 16384);
}

#[tokio::test]
async fn test_default_resource_allocation() {
    let default_wasm = TestWorkloadBuilder::wasm().build();

    // Should have default resources
    assert!(default_wasm["resources"].is_object());
    assert!(default_wasm["resources"]["cpu_cores"].is_number());
    assert!(default_wasm["resources"]["memory_mb"].is_number());
}

#[tokio::test]
async fn test_resource_configuration_for_all_runtime_types() {
    let runtimes = vec![
        TestWorkloadBuilder::wasm().with_resources(1.0, 512).build(),
        TestWorkloadBuilder::native()
            .with_resources(2.0, 1024)
            .build(),
        TestWorkloadBuilder::container()
            .with_resources(4.0, 2048)
            .build(),
        TestWorkloadBuilder::python()
            .with_resources(3.0, 1536)
            .build(),
    ];

    // All runtime types should support resource configuration
    for workload in runtimes {
        assert!(workload["resources"]["cpu_cores"].as_f64().unwrap() > 0.0);
        assert!(workload["resources"]["memory_mb"].as_u64().unwrap() > 0);
    }
}

#[tokio::test]
async fn test_resource_limits_validation() {
    // Test that we can create workloads with various resource limits
    let configs = vec![
        (0.1, 128),    // Minimal
        (1.0, 512),    // Small
        (4.0, 2048),   // Medium
        (16.0, 8192),  // Large
        (32.0, 32768), // Maximum
    ];

    for (cpu, mem) in configs {
        let workload = TestWorkloadBuilder::wasm().with_resources(cpu, mem).build();

        assert_eq!(workload["resources"]["cpu_cores"], cpu);
        assert_eq!(workload["resources"]["memory_mb"], mem);
    }
}

#[tokio::test]
async fn test_resource_configuration_in_isolated_environments() {
    let env1 = TestEnvironment::new();
    let env2 = TestEnvironment::new();

    // Configure different resources in each environment
    let workload1 = TestWorkloadBuilder::wasm()
        .with_resources(2.0, 1024)
        .build();

    let workload2 = TestWorkloadBuilder::wasm()
        .with_resources(8.0, 8192)
        .build();

    std::fs::write(
        env1.config_dir.join("workload.json"),
        serde_json::to_string_pretty(&workload1).unwrap(),
    )
    .unwrap();

    std::fs::write(
        env2.config_dir.join("workload.json"),
        serde_json::to_string_pretty(&workload2).unwrap(),
    )
    .unwrap();

    // Verify isolation
    let loaded1: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(env1.config_dir.join("workload.json")).unwrap(),
    )
    .unwrap();

    let loaded2: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(env2.config_dir.join("workload.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(loaded1["resources"]["cpu_cores"], 2.0);
    assert_eq!(loaded2["resources"]["cpu_cores"], 8.0);
}
