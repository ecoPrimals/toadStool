// SPDX-License-Identifier: AGPL-3.0-only
#![expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]

use super::{
    Capabilities, ComputeError, ComputeUnitType, DataType, ExecutionModel, LatencyProfile,
    OperationType, Parallelism, ParamValue, PowerProfile, Workload, WorkloadBuilder, WorkloadData,
    WorkloadParams,
};

fn make_caps(
    supported_ops: Vec<OperationType>,
    supported_types: Vec<DataType>,
    memory: usize,
) -> Capabilities {
    Capabilities {
        unit_type: ComputeUnitType::Cpu,
        parallelism: Parallelism {
            num_units: 4,
            model: ExecutionModel::Mimd,
        },
        power_profile: PowerProfile::Medium,
        latency: LatencyProfile {
            typical_ms: 1,
            deterministic: true,
        },
        memory_capacity: memory,
        memory_bandwidth: 50_000_000_000,
        compute_throughput: 400e9,
        optimal_batch_size: 100,
        supported_ops,
        supported_types,
    }
}

fn make_workload(op: OperationType, dtype: DataType, mem: usize) -> Workload {
    Workload {
        operation: op,
        data_type: dtype,
        num_operations: 100,
        required_memory: mem,
        input: WorkloadData::F32Vec(vec![]),
        params: WorkloadParams::default(),
    }
}

#[test]
fn test_capabilities_supports_workload_ok() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
    let w = make_workload(OperationType::Map, DataType::F32, 512);
    assert!(caps.supports_workload(&w));
}

#[test]
fn test_capabilities_unsupported_op() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
    let w = make_workload(OperationType::Reduce, DataType::F32, 0);
    assert!(!caps.supports_workload(&w));
}

#[test]
fn test_capabilities_unsupported_dtype() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
    let w = make_workload(OperationType::Map, DataType::F64, 0);
    assert!(!caps.supports_workload(&w));
}

#[test]
fn test_capabilities_insufficient_memory() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 100);
    let w = make_workload(OperationType::Map, DataType::F32, 200);
    assert!(!caps.supports_workload(&w));
}

#[test]
fn test_capabilities_estimate_duration_positive() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
    let w = make_workload(OperationType::Map, DataType::F32, 0);
    let dur = caps.estimate_duration(&w);
    assert!(dur.as_nanos() > 0);
}

#[test]
fn test_capabilities_score_unsupported_returns_zero() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
    let w = make_workload(OperationType::Reduce, DataType::F32, 0);
    assert_eq!(caps.score_for_workload(&w), 0.0);
}

#[test]
fn test_capabilities_score_supported_positive() {
    let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
    let w = make_workload(OperationType::Map, DataType::F32, 0);
    assert!(caps.score_for_workload(&w) > 0.0);
}

#[test]
fn test_compute_unit_type_display() {
    assert_eq!(format!("{}", ComputeUnitType::Cpu), "CPU");
    assert_eq!(format!("{}", ComputeUnitType::GpuOpenCl), "GPU (OpenCL)");
    assert_eq!(format!("{}", ComputeUnitType::GpuWgpu), "GPU (wgpu)");
    assert_eq!(format!("{}", ComputeUnitType::GpuVulkan), "GPU (Vulkan)");
    assert_eq!(format!("{}", ComputeUnitType::Neuromorphic), "Neuromorphic");
    assert_eq!(format!("{}", ComputeUnitType::Custom(42)), "Custom(42)");
}

#[test]
fn test_power_profile_variants_exist() {
    let _ = PowerProfile::UltraLow;
    let _ = PowerProfile::Low;
    let _ = PowerProfile::Medium;
    let _ = PowerProfile::High;
}

#[test]
fn test_workload_builder_builds_ok() {
    let w = WorkloadBuilder::new()
        .operation(OperationType::Map)
        .data_f32(vec![1.0, 2.0, 3.0])
        .build()
        .unwrap();
    assert_eq!(w.operation, OperationType::Map);
    assert_eq!(w.data_type, DataType::F32);
    assert_eq!(w.num_operations, 3);
}

#[test]
fn test_workload_builder_missing_op_fails() {
    let result = WorkloadBuilder::new().data_f32(vec![1.0]).build();
    assert!(matches!(result, Err(ComputeError::ExecutionFailed(_))));
}

#[test]
fn test_workload_builder_missing_data_fails() {
    let result = WorkloadBuilder::new().operation(OperationType::Map).build();
    assert!(result.is_err());
}

#[test]
fn test_workload_builder_with_param() {
    let w = WorkloadBuilder::new()
        .operation(OperationType::ElementwiseBinary)
        .data_f32(vec![1.0])
        .param("op", ParamValue::String("add".into()))
        .build()
        .unwrap();
    assert!(w.params.params.contains_key("op"));
}

#[test]
fn test_compute_error_display() {
    let e = ComputeError::UnsupportedWorkload;
    assert!(!format!("{e}").is_empty());
    let e2 = ComputeError::ExecutionFailed("oops".into());
    assert!(format!("{e2}").contains("oops"));
}
