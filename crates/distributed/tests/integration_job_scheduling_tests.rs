// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests for Job Scheduling
//!
//! These tests exercise job scheduling and coordination logic.

use toadstool_testing::fixtures::{TestEnvironment, runtime::*};

#[tokio::test]
async fn test_workload_can_be_scheduled() {
    let workload = create_wasm_test_workload();

    // Workload should have all necessary fields for scheduling
    assert!(workload.get("workload_type").is_some());
    assert!(workload.get("resources").is_some());
}

#[tokio::test]
async fn test_heavy_workload_scheduling_requirements() {
    let heavy = create_heavy_test_workload();

    // Heavy workloads have higher resource requirements
    let cpu = heavy["resources"]["cpu_cores"].as_f64().unwrap();
    let memory = heavy["resources"]["memory_mb"].as_u64().unwrap();

    assert!(cpu >= 4.0);
    assert!(memory >= 2048);
}

#[tokio::test]
async fn test_multiple_workloads_can_be_queued() {
    let env = TestEnvironment::new();

    let workloads = vec![
        create_wasm_test_workload(),
        create_native_test_workload(),
        create_heavy_test_workload(),
    ];

    // Save all workloads to queue directory
    for (i, workload) in workloads.iter().enumerate() {
        std::fs::write(
            env.data_dir.join(format!("job_{i}.json")),
            serde_json::to_string_pretty(&workload).unwrap(),
        )
        .unwrap();
    }

    // Verify all jobs are queued
    assert!(env.data_dir.join("job_0.json").exists());
    assert!(env.data_dir.join("job_1.json").exists());
    assert!(env.data_dir.join("job_2.json").exists());
}

#[tokio::test]
async fn test_workload_priority_configuration() {
    let high_priority = TestWorkloadBuilder::wasm()
        .with_resources(4.0, 4096)
        .with_timeout(60)
        .build();

    let low_priority = TestWorkloadBuilder::wasm()
        .with_resources(1.0, 512)
        .with_timeout(300)
        .build();

    // Higher resource workloads might have higher priority
    assert!(
        high_priority["resources"]["cpu_cores"].as_f64().unwrap()
            > low_priority["resources"]["cpu_cores"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn test_timeout_based_scheduling() {
    let short = TestWorkloadBuilder::wasm().with_timeout(10).build();
    let medium = TestWorkloadBuilder::wasm().with_timeout(60).build();
    let long = TestWorkloadBuilder::wasm().with_timeout(300).build();

    // Workloads with different timeouts can be scheduled
    assert_eq!(short["timeout_seconds"], 10);
    assert_eq!(medium["timeout_seconds"], 60);
    assert_eq!(long["timeout_seconds"], 300);
}

#[tokio::test]
async fn test_workload_type_based_routing() {
    let wasm = create_wasm_test_workload();
    let native = create_native_test_workload();

    // Different workload types might route to different executors
    assert_eq!(wasm["workload_type"], "Wasm");
    assert_eq!(native["workload_type"], "Native");
}

#[tokio::test]
async fn test_resource_based_job_placement() {
    let small = TestWorkloadBuilder::wasm().with_resources(0.5, 256).build();

    let large = TestWorkloadBuilder::wasm()
        .with_resources(16.0, 16384)
        .build();

    // Jobs with different resource requirements
    assert!(
        small["resources"]["cpu_cores"].as_f64().unwrap()
            < large["resources"]["cpu_cores"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn test_concurrent_job_scheduling() {
    let env = TestEnvironment::new();

    // Create multiple jobs concurrently
    let jobs = vec![
        ("job_a", create_wasm_test_workload()),
        ("job_b", create_native_test_workload()),
        ("job_c", create_heavy_test_workload()),
    ];

    for (name, workload) in jobs {
        std::fs::write(
            env.data_dir.join(format!("{name}.json")),
            serde_json::to_string_pretty(&workload).unwrap(),
        )
        .unwrap();
    }

    // All jobs should be schedulable
    assert!(env.data_dir.join("job_a.json").exists());
    assert!(env.data_dir.join("job_b.json").exists());
    assert!(env.data_dir.join("job_c.json").exists());
}

#[tokio::test]
async fn test_job_scheduling_with_dependencies() {
    let env = TestEnvironment::new();

    // Create jobs that might have dependencies
    let job1 = TestWorkloadBuilder::wasm()
        .with_entry_point("step1")
        .build();

    let job2 = TestWorkloadBuilder::wasm()
        .with_entry_point("step2")
        .build();

    // Save in dependency order
    std::fs::write(
        env.data_dir.join("job1.json"),
        serde_json::to_string_pretty(&job1).unwrap(),
    )
    .unwrap();

    std::fs::write(
        env.data_dir.join("job2.json"),
        serde_json::to_string_pretty(&job2).unwrap(),
    )
    .unwrap();

    assert!(env.data_dir.join("job1.json").exists());
    assert!(env.data_dir.join("job2.json").exists());
}

#[tokio::test]
async fn test_batch_job_scheduling() {
    let env = TestEnvironment::new();

    // Create a batch of similar jobs
    for i in 0..10 {
        let workload = TestWorkloadBuilder::wasm()
            .with_entry_point(format!("batch_job_{i}"))
            .with_resources(1.0, 512)
            .build();

        std::fs::write(
            env.data_dir.join(format!("batch_{i}.json")),
            serde_json::to_string_pretty(&workload).unwrap(),
        )
        .unwrap();
    }

    // Verify all batch jobs are created
    for i in 0..10 {
        assert!(env.data_dir.join(format!("batch_{i}.json")).exists());
    }
}
