// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Tests for Universal Compute Operations
//!
//! Testing strategy:
//! - Platform status types
//! - Benchmark types
//! - System information structures
//! - Serialization/deserialization

use std::collections::HashMap;
use toadstool_cli::universal::{BenchmarkTest, BenchmarkType, PlatformStatus, SystemInfo};
use tokio::time::Duration;

#[test]
fn test_platform_status_available() {
    let status = PlatformStatus::Available;
    match status {
        PlatformStatus::Available => {} // OK - variant matches
        _ => panic!("Expected Available status"),
    }
}

#[test]
fn test_platform_status_testing() {
    let status = PlatformStatus::Testing;
    match status {
        PlatformStatus::Testing => {} // OK - variant matches
        _ => panic!("Expected Testing status"),
    }
}

#[test]
fn test_platform_status_degraded() {
    let status = PlatformStatus::Degraded;
    match status {
        PlatformStatus::Degraded => {} // OK - variant matches
        _ => panic!("Expected Degraded status"),
    }
}

#[test]
fn test_platform_status_unavailable() {
    let status = PlatformStatus::Unavailable;
    match status {
        PlatformStatus::Unavailable => {} // OK - variant matches
        _ => panic!("Expected Unavailable status"),
    }
}

#[test]
fn test_platform_status_error() {
    let status = PlatformStatus::Error("Test error".to_string());
    match status {
        PlatformStatus::Error(msg) => {
            assert_eq!(msg, "Test error");
        }
        _ => panic!("Expected Error status"),
    }
}

#[test]
fn test_benchmark_type_cpu_integer() {
    let bench_type = BenchmarkType::CpuInteger;
    match bench_type {
        BenchmarkType::CpuInteger => {} // OK - variant matches
        _ => panic!("Expected CpuInteger"),
    }
}

#[test]
fn test_benchmark_type_cpu_float() {
    let bench_type = BenchmarkType::CpuFloat;
    match bench_type {
        BenchmarkType::CpuFloat => {} // OK - variant matches
        _ => panic!("Expected CpuFloat"),
    }
}

#[test]
fn test_benchmark_type_memory() {
    let bench_type = BenchmarkType::Memory;
    match bench_type {
        BenchmarkType::Memory => {} // OK - variant matches
        _ => panic!("Expected Memory"),
    }
}

#[test]
fn test_benchmark_type_storage() {
    let bench_type = BenchmarkType::Storage;
    match bench_type {
        BenchmarkType::Storage => {} // OK - variant matches
        _ => panic!("Expected Storage"),
    }
}

#[test]
fn test_benchmark_type_network() {
    let bench_type = BenchmarkType::Network;
    match bench_type {
        BenchmarkType::Network => {} // OK - variant matches
        _ => panic!("Expected Network"),
    }
}

#[test]
fn test_benchmark_type_gpu() {
    let bench_type = BenchmarkType::Gpu;
    match bench_type {
        BenchmarkType::Gpu => {} // OK - variant matches
        _ => panic!("Expected Gpu"),
    }
}

#[test]
fn test_benchmark_type_wasm_execution() {
    let bench_type = BenchmarkType::WasmExecution;
    match bench_type {
        BenchmarkType::WasmExecution => {} // OK - variant matches
        _ => panic!("Expected WasmExecution"),
    }
}

#[test]
fn test_benchmark_type_container_startup() {
    let bench_type = BenchmarkType::ContainerStartup;
    match bench_type {
        BenchmarkType::ContainerStartup => {} // OK - variant matches
        _ => panic!("Expected ContainerStartup"),
    }
}

#[test]
fn test_benchmark_type_custom() {
    let bench_type = BenchmarkType::Custom("my-benchmark".to_string());
    match bench_type {
        BenchmarkType::Custom(name) => {
            assert_eq!(name, "my-benchmark");
        }
        _ => panic!("Expected Custom benchmark type"),
    }
}

#[test]
fn test_system_info_structure() {
    let sys_info = SystemInfo {
        os: "Linux".to_string(),
        arch: "x86_64".to_string(),
        cpu_model: "Intel i7".to_string(),
        cpu_cores: 8,
        memory_gb: 16.0,
        storage_type: "SSD".to_string(),
        gpu_info: Some("NVIDIA GTX".to_string()),
    };

    assert_eq!(sys_info.os, "Linux");
    assert_eq!(sys_info.arch, "x86_64");
    assert_eq!(sys_info.cpu_cores, 8);
    assert_eq!(sys_info.memory_gb, 16.0);
    assert!(sys_info.gpu_info.is_some());
}

#[test]
fn test_system_info_no_gpu() {
    let sys_info = SystemInfo {
        os: "macOS".to_string(),
        arch: "arm64".to_string(),
        cpu_model: "Apple M1".to_string(),
        cpu_cores: 8,
        memory_gb: 16.0,
        storage_type: "SSD".to_string(),
        gpu_info: None,
    };

    assert!(sys_info.gpu_info.is_none());
}

#[test]
fn test_benchmark_test_structure() {
    let mut details = HashMap::new();
    details.insert("iterations".to_string(), serde_json::json!(1000));

    let bench_test = BenchmarkTest {
        name: "CPU Test".to_string(),
        test_type: BenchmarkType::CpuInteger,
        duration: Duration::from_millis(100),
        score: 95.5,
        unit: "ops/sec".to_string(),
        details,
    };

    assert_eq!(bench_test.name, "CPU Test");
    assert_eq!(bench_test.score, 95.5);
    assert_eq!(bench_test.unit, "ops/sec");
    assert!(bench_test.details.contains_key("iterations"));
}

#[test]
fn test_platform_status_serialization() {
    let status = PlatformStatus::Available;
    let json = serde_json::to_string(&status).expect("Should serialize");
    assert!(json.contains("Available"));
}

#[test]
fn test_platform_status_deserialization() {
    let json = r#""Available""#;
    let status: PlatformStatus = serde_json::from_str(json).expect("Should deserialize");
    match status {
        PlatformStatus::Available => {} // OK - deserialized correctly
        _ => panic!("Expected Available"),
    }
}

#[test]
fn test_benchmark_type_serialization() {
    let bench_type = BenchmarkType::Memory;
    let json = serde_json::to_string(&bench_type).expect("Should serialize");
    assert!(json.contains("Memory"));
}

#[test]
fn test_benchmark_type_custom_serialization() {
    let bench_type = BenchmarkType::Custom("test".to_string());
    let json = serde_json::to_string(&bench_type).expect("Should serialize");
    assert!(json.contains("Custom"));
    assert!(json.contains("test"));
}

#[test]
fn test_system_info_serialization() {
    let sys_info = SystemInfo {
        os: "Linux".to_string(),
        arch: "x86_64".to_string(),
        cpu_model: "Intel".to_string(),
        cpu_cores: 4,
        memory_gb: 8.0,
        storage_type: "HDD".to_string(),
        gpu_info: None,
    };

    let json = serde_json::to_string(&sys_info).expect("Should serialize");
    assert!(json.contains("Linux"));
    assert!(json.contains("x86_64"));
    assert!(json.contains("Intel"));
}

#[test]
fn test_system_info_deserialization() {
    let json = r#"{"os":"Linux","arch":"x86_64","cpu_model":"Intel","cpu_cores":4,"memory_gb":8.0,"storage_type":"HDD","gpu_info":null}"#;
    let sys_info: SystemInfo = serde_json::from_str(json).expect("Should deserialize");

    assert_eq!(sys_info.os, "Linux");
    assert_eq!(sys_info.cpu_cores, 4);
    assert_eq!(sys_info.memory_gb, 8.0);
}

#[test]
fn test_platform_status_clone() {
    let status1 = PlatformStatus::Available;
    let status2 = status1.clone();

    match (status1, status2) {
        (PlatformStatus::Available, PlatformStatus::Available) => {} // OK - clone works
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_benchmark_type_clone() {
    let bench1 = BenchmarkType::CpuInteger;
    let bench2 = bench1.clone();

    match (bench1, bench2) {
        (BenchmarkType::CpuInteger, BenchmarkType::CpuInteger) => {} // OK - clone works
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_system_info_clone() {
    let sys1 = SystemInfo {
        os: "Linux".to_string(),
        arch: "x86_64".to_string(),
        cpu_model: "Intel".to_string(),
        cpu_cores: 8,
        memory_gb: 16.0,
        storage_type: "SSD".to_string(),
        gpu_info: None,
    };

    let sys2 = sys1.clone();
    assert_eq!(sys1.os, sys2.os);
    assert_eq!(sys1.cpu_cores, sys2.cpu_cores);
}
