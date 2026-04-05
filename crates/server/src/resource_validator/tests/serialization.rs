// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::resource_estimator::EstimationError;
use crate::resource_validator::{
    AvailabilityResult, ResourceGap, ResourceValidator, SystemCapabilities, ValidationError,
};

use super::helpers::{base_capabilities, base_estimate};

#[test]
fn test_resource_gap_serialization_roundtrip() {
    let gap = ResourceGap {
        resource_type: "cpu_cores".to_string(),
        required: 16,
        available: 8,
        shortage: 8,
        suggestion: "Add cores".to_string(),
    };
    let json = serde_json::to_string(&gap).unwrap();
    let restored: ResourceGap = serde_json::from_str(&json).unwrap();
    assert_eq!(gap.resource_type, restored.resource_type);
    assert_eq!(gap.shortage, restored.shortage);
}

#[test]
fn test_system_capabilities_serialization_roundtrip() {
    let caps = SystemCapabilities {
        total_cpu_cores: 16,
        available_cpu_cores: 12,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 24 * 1024 * 1024 * 1024,
        total_gpu_memory_bytes: 8192 * 1024 * 1024,
        available_gpu_memory_bytes: 6144 * 1024 * 1024,
        total_storage_bytes: 512 * 1024 * 1024 * 1024,
        available_storage_bytes: 256 * 1024 * 1024 * 1024,
        network_bandwidth_mbps: 1000,
        gpu_count: 1,
        gpu_types: vec!["NVIDIA RTX 3090".to_string()],
    };
    let json = serde_json::to_string(&caps).unwrap();
    let restored: SystemCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(caps.total_cpu_cores, restored.total_cpu_cores);
    assert_eq!(caps.gpu_types, restored.gpu_types);
}

#[test]
fn test_validation_error_display() {
    let err =
        ValidationError::EstimationFailed(crate::resource_estimator::EstimationError::CyclicGraph);
    assert!(err.to_string().contains("Estimation") || err.to_string().contains("cycle"));

    let err2 = ValidationError::SystemQueryFailed("disk read failed".to_string());
    assert!(err2.to_string().contains("disk read failed"));

    let err3 = ValidationError::InvalidConfiguration("bad config".to_string());
    assert!(err3.to_string().contains("Invalid configuration"));
}

#[test]
fn test_resource_validator_default() {
    let _v = ResourceValidator::default();
    let _v2 = ResourceValidator::new();
}

#[test]
fn availability_result_serialization_roundtrip() {
    let r = AvailabilityResult {
        graph_id: "g".to_string(),
        available: true,
        gaps: vec![],
        warnings: vec![],
        system_capabilities: base_capabilities(),
        estimated_requirements: base_estimate(),
    };
    let json = serde_json::to_string(&r).unwrap();
    let back: AvailabilityResult = serde_json::from_str(&json).unwrap();
    assert_eq!(back.graph_id, "g");
    assert!(back.available);
}

#[test]
fn validation_error_from_estimation_error() {
    let e: ValidationError = EstimationError::CyclicGraph.into();
    assert!(matches!(
        e,
        ValidationError::EstimationFailed(EstimationError::CyclicGraph)
    ));
}
