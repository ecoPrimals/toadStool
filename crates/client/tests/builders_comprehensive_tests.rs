//! Comprehensive tests for client workload builders
//!
//! Week 13 Day 1 (Final Push): Client Builder Tests
//! Target: Native, WASM, and Container builder functionality

use std::time::Duration;
use toadstool_client::{JobPriority, ResourceRequirements, WorkloadSubmission};

// =============================================================================
// Native Workload Builder Tests
// =============================================================================

#[test]
fn test_native_builder_creation() {
    let _builder = WorkloadSubmission::native();
    // Should create successfully
}

#[test]
fn test_native_builder_executable() {
    let result = WorkloadSubmission::native().executable("/bin/echo").build();

    assert!(result.is_ok());
}

#[test]
fn test_native_builder_with_args() {
    let args = vec!["--version".to_string(), "--help".to_string()];
    let result = WorkloadSubmission::native()
        .executable("/bin/test")
        .args(args)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_native_builder_with_timeout() {
    let result = WorkloadSubmission::native()
        .executable("/bin/sleep")
        .timeout(Duration::from_secs(60))
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_native_builder_with_priority() {
    let result = WorkloadSubmission::native()
        .executable("/bin/echo")
        .priority(JobPriority::High)
        .build();

    assert!(result.is_ok());
}

#[test]
fn test_native_builder_missing_executable() {
    let result = WorkloadSubmission::native().build();
    // Should fail without executable
    assert!(result.is_err());
}

#[test]
fn test_native_builder_with_resources() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(1000),
        disk_mb: Some(10000),
        gpu_required: Some(false),
    };

    let result = WorkloadSubmission::native()
        .executable("/bin/test")
        .resources(resources)
        .build();

    assert!(result.is_ok());
}

// =============================================================================
// WASM Workload Builder Tests
// =============================================================================

#[test]
fn test_wasm_builder_creation() {
    let _builder = WorkloadSubmission::wasm();
    // Should create successfully
}

#[test]
fn test_wasm_builder_with_module_data() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic number
    let _submission = WorkloadSubmission::wasm().module_data(module_data).build();
    // Build succeeds with module data
}

#[test]
fn test_wasm_builder_with_args() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d];
    let args = vec!["arg1".to_string(), "arg2".to_string()];

    let _submission = WorkloadSubmission::wasm()
        .module_data(module_data)
        .args(args)
        .build();
    // Build succeeds
}

#[test]
fn test_wasm_builder_with_timeout() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d];

    let _submission = WorkloadSubmission::wasm()
        .module_data(module_data)
        .timeout(Duration::from_secs(30))
        .build();
    // Build succeeds
}

#[test]
fn test_wasm_builder_with_priority() {
    let module_data = vec![0x00, 0x61, 0x73, 0x6d];

    let _submission = WorkloadSubmission::wasm()
        .module_data(module_data)
        .priority(JobPriority::Low)
        .build();
    // Build succeeds
}

#[test]
#[should_panic(expected = "Module data is required")]
fn test_wasm_builder_missing_module_data() {
    let _result = WorkloadSubmission::wasm().build();
    // Should panic without module data
}

// =============================================================================
// Container Workload Builder Tests
// =============================================================================

#[test]
fn test_container_builder_creation() {
    let _builder = WorkloadSubmission::container();
    // Should create successfully
}

#[test]
fn test_container_builder_with_image() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .build();
    // Build succeeds with image
}

#[test]
fn test_container_builder_with_command() {
    let command = vec!["sh".to_string(), "-c".to_string()];
    let _submission = WorkloadSubmission::container()
        .image("ubuntu:22.04")
        .command(command)
        .build();
    // Build succeeds
}

#[test]
fn test_container_builder_with_args() {
    let args = vec!["echo".to_string(), "hello".to_string()];
    let _submission = WorkloadSubmission::container()
        .image("busybox:latest")
        .args(args)
        .build();
    // Build succeeds
}

#[test]
fn test_container_builder_with_timeout() {
    let _submission = WorkloadSubmission::container()
        .image("python:3.11")
        .timeout(Duration::from_secs(180))
        .build();
    // Build succeeds
}

#[test]
fn test_container_builder_with_priority() {
    let _submission = WorkloadSubmission::container()
        .image("redis:7")
        .priority(JobPriority::High)
        .build();
    // Build succeeds
}

// =============================================================================
// Builder Pattern Consistency Tests
// =============================================================================

#[test]
fn test_builder_chaining_consistency() {
    // Test that all builders support method chaining properly

    let _native = WorkloadSubmission::native()
        .executable("/bin/test")
        .timeout(Duration::from_secs(30))
        .priority(JobPriority::Normal);

    let _wasm = WorkloadSubmission::wasm()
        .module_data(vec![0x00, 0x61, 0x73, 0x6d])
        .timeout(Duration::from_secs(30))
        .priority(JobPriority::Normal);

    let _container = WorkloadSubmission::container()
        .image("test:latest")
        .timeout(Duration::from_secs(30))
        .priority(JobPriority::Normal);

    // All should chain successfully
}
