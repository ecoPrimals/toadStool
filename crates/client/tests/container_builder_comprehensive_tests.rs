// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Container workload builder.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_client::{JobPriority, ResourceRequirements, WorkloadSubmission};

// =============================================================================
// Container Builder Creation & Basic Tests
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
fn test_container_builder_with_docker_hub_image() {
    let _submission = WorkloadSubmission::container().image("nginx:1.25").build();
    // Docker Hub images work
}

#[test]
fn test_container_builder_with_custom_registry() {
    let _submission = WorkloadSubmission::container()
        .image("gcr.io/my-project/my-image:v1.0.0")
        .build();
    // Custom registry images work
}

#[test]
fn test_container_builder_missing_image() {
    let result = WorkloadSubmission::container().build();
    // Should return error without image
    assert!(result.is_err(), "Building without image should fail");
    assert!(
        result.unwrap_err().contains("Image is required"),
        "Error should mention missing image"
    );
}

#[test]
fn test_container_builder_empty_image() {
    let _submission = WorkloadSubmission::container().image("").build();
    // Empty image is allowed (will fail on execution)
}

// =============================================================================
// Command & Args Tests
// =============================================================================

#[test]
fn test_container_builder_with_command() {
    let command = vec!["echo".to_string(), "hello".to_string()];
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .command(command)
        .build();
}

#[test]
fn test_container_builder_with_shell_command() {
    let command = vec![
        "/bin/sh".to_string(),
        "-c".to_string(),
        "echo hello".to_string(),
    ];
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .command(command)
        .build();
}

#[test]
fn test_container_builder_with_args() {
    let args = vec!["--version".to_string(), "--help".to_string()];
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .args(args)
        .build();
}

#[test]
fn test_container_builder_with_command_and_args() {
    let command = vec!["python3".to_string()];
    let args = vec!["-c".to_string(), "print('Hello')".to_string()];
    let _submission = WorkloadSubmission::container()
        .image("python:3.11")
        .command(command)
        .args(args)
        .build();
}

#[test]
fn test_container_builder_with_empty_args() {
    let args = Vec::new();
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .args(args)
        .build();
}

// =============================================================================
// Working Directory Tests
// =============================================================================

#[test]
fn test_container_builder_with_working_dir() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .working_dir("/app")
        .build();
}

#[test]
fn test_container_builder_with_absolute_working_dir() {
    let _submission = WorkloadSubmission::container()
        .image("node:18")
        .working_dir("/usr/src/app")
        .build();
}

#[test]
fn test_container_builder_with_relative_working_dir() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .working_dir("./data")
        .build();
}

// =============================================================================
// Environment Variable Tests
// =============================================================================

#[test]
fn test_container_builder_with_single_env_var() {
    let mut environment = HashMap::new();
    environment.insert("NODE_ENV".to_string(), "production".to_string());

    let _submission = WorkloadSubmission::container()
        .image("node:18")
        .environment(environment)
        .build();
}

#[test]
fn test_container_builder_with_multiple_env_vars() {
    let mut environment = HashMap::new();
    environment.insert(
        "DATABASE_URL".to_string(),
        "postgresql://localhost/db".to_string(),
    );
    environment.insert("API_KEY".to_string(), "secret123".to_string());
    environment.insert("LOG_LEVEL".to_string(), "DEBUG".to_string());

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .environment(environment)
        .build();
}

#[test]
fn test_container_builder_with_empty_environment() {
    let environment = HashMap::new();

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .environment(environment)
        .build();
}

// =============================================================================
// Priority Tests
// =============================================================================

#[test]
fn test_container_builder_with_priority_low() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .priority(JobPriority::Low)
        .build();
}

#[test]
fn test_container_builder_with_priority_normal() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .priority(JobPriority::Normal)
        .build();
}

#[test]
fn test_container_builder_with_priority_high() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .priority(JobPriority::High)
        .build();
}

// =============================================================================
// Timeout Tests
// =============================================================================

#[test]
fn test_container_builder_with_timeout_short() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .timeout(Duration::from_secs(30))
        .build();
}

#[test]
fn test_container_builder_with_timeout_long() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .timeout(Duration::from_secs(3600))
        .build();
}

#[test]
fn test_container_builder_with_timeout_zero() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .timeout(Duration::from_secs(0))
        .build();
}

// =============================================================================
// Resource Requirements Tests
// =============================================================================

#[test]
fn test_container_builder_with_cpu_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: None,
        disk_mb: None,
        gpu_required: None,
    };

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .resources(resources)
        .build();
}

#[test]
fn test_container_builder_with_memory_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: Some(512),
        disk_mb: None,
        gpu_required: None,
    };

    let _submission = WorkloadSubmission::container()
        .image("nginx:latest")
        .resources(resources)
        .build();
}

#[test]
fn test_container_builder_with_disk_requirement() {
    let resources = ResourceRequirements {
        cpu_cores: None,
        memory_mb: None,
        disk_mb: Some(5000),
        gpu_required: None,
    };

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .resources(resources)
        .build();
}

#[test]
fn test_container_builder_with_all_resource_requirements() {
    let resources = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(4096),
        disk_mb: Some(20000),
        gpu_required: Some(false),
    };

    let _submission = WorkloadSubmission::container()
        .image("tensorflow/tensorflow:latest")
        .resources(resources)
        .build();
}

// =============================================================================
// Metadata Tests
// =============================================================================

#[test]
fn test_container_builder_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "test_user".to_string());
    metadata.insert("version".to_string(), "1.0.0".to_string());

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .metadata(metadata)
        .build();
}

#[test]
fn test_container_builder_with_empty_metadata() {
    let metadata = HashMap::new();

    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .metadata(metadata)
        .build();
}

// =============================================================================
// Complex Integration Tests
// =============================================================================

#[test]
fn test_container_builder_full_configuration() {
    let mut environment = HashMap::new();
    environment.insert("NODE_ENV".to_string(), "production".to_string());
    environment.insert("PORT".to_string(), "3000".to_string());

    let resources = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(2048),
        disk_mb: Some(10000),
        gpu_required: Some(false),
    };

    let mut metadata = HashMap::new();
    metadata.insert("service".to_string(), "api".to_string());
    metadata.insert("region".to_string(), "us-west-2".to_string());

    let command = vec!["node".to_string()];
    let args = vec!["server.js".to_string()];

    let _submission = WorkloadSubmission::container()
        .image("node:18-alpine")
        .command(command)
        .args(args)
        .working_dir("/app")
        .environment(environment)
        .priority(JobPriority::High)
        .timeout(Duration::from_secs(300))
        .resources(resources)
        .metadata(metadata)
        .build();
}

#[test]
fn test_container_builder_web_service() {
    let command = vec![
        "nginx".to_string(),
        "-g".to_string(),
        "daemon off;".to_string(),
    ];

    let _submission = WorkloadSubmission::container()
        .image("nginx:1.25-alpine")
        .command(command)
        .working_dir("/usr/share/nginx/html")
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(3600))
        .build();
}

#[test]
fn test_container_builder_batch_job() {
    let command = vec!["python3".to_string()];
    let args = vec![
        "process_data.py".to_string(),
        "--input".to_string(),
        "/data".to_string(),
    ];

    let resources = ResourceRequirements {
        cpu_cores: Some(8),
        memory_mb: Some(16384),
        disk_mb: Some(50000),
        gpu_required: Some(false),
    };

    let _submission = WorkloadSubmission::container()
        .image("python:3.11")
        .command(command)
        .args(args)
        .working_dir("/workspace")
        .resources(resources)
        .priority(JobPriority::Low)
        .timeout(Duration::from_secs(7200))
        .build();
}

#[test]
fn test_container_builder_ml_training() {
    let mut environment = HashMap::new();
    environment.insert("CUDA_VISIBLE_DEVICES".to_string(), "0,1".to_string());
    environment.insert("TF_FORCE_GPU_ALLOW_GROWTH".to_string(), "true".to_string());

    let resources = ResourceRequirements {
        cpu_cores: Some(16),
        memory_mb: Some(65536),
        disk_mb: Some(100000),
        gpu_required: Some(true),
    };

    let _submission = WorkloadSubmission::container()
        .image("tensorflow/tensorflow:latest-gpu")
        .command(vec!["python3".to_string()])
        .args(vec!["train.py".to_string()])
        .working_dir("/workspace")
        .environment(environment)
        .resources(resources)
        .priority(JobPriority::High)
        .timeout(Duration::from_secs(86400))
        .build();
}

#[test]
fn test_container_builder_with_chained_methods() {
    let _submission = WorkloadSubmission::container()
        .image("alpine:latest")
        .command(vec!["echo".to_string()])
        .args(vec!["hello".to_string()])
        .working_dir("/tmp")
        .priority(JobPriority::Normal)
        .timeout(Duration::from_secs(60))
        .build();
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_container_builder_with_image_tag() {
    let _submission = WorkloadSubmission::container()
        .image("ubuntu:22.04")
        .build();
}

#[test]
fn test_container_builder_with_image_digest() {
    let _submission = WorkloadSubmission::container()
        .image("alpine@sha256:1234567890abcdef")
        .build();
}

#[test]
fn test_container_builder_with_localhost_registry() {
    let _submission = WorkloadSubmission::container()
        .image("localhost:5000/my-image:latest")
        .build();
}

#[test]
fn test_container_builder_minimal_configuration() {
    let _submission = WorkloadSubmission::container().image("alpine").build();
}
