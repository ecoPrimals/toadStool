// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(clippy::float_cmp, reason = "test values are exact literals")]

use super::*;
use crate::types::{ComputeUnitDispatch, DataType, OperationType, WorkloadData, WorkloadParams};

fn simple_f32_workload(op: OperationType, input: WorkloadData) -> Workload {
    Workload {
        operation: op,
        data_type: DataType::F32,
        num_operations: 3,
        required_memory: 12,
        input,
        params: WorkloadParams::default(),
    }
}

#[test]
fn test_runtime_stats_default() {
    let stats = RuntimeStats::default();
    assert_eq!(stats.num_cpu, 0);
    assert_eq!(stats.num_gpu, 0);
    assert_eq!(stats.num_neuromorphic, 0);
    assert_eq!(stats.num_custom, 0);
    assert_eq!(stats.total_memory, 0);
    assert_eq!(stats.total_compute_throughput, 0.0);
}

#[test]
fn test_runtime_stats_display() {
    let stats = RuntimeStats {
        num_cpu: 2,
        num_gpu: 1,
        total_memory: 8_000_000_000,
        total_compute_throughput: 800e9,
        ..Default::default()
    };
    let s = format!("{stats}");
    assert!(s.contains("CPU units: 2"));
    assert!(s.contains("GPU units: 1"));
    assert!(s.contains("8.00 GB"));
    assert!(s.contains("800.00 GFLOPS"));
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_universal_runtime_discover_has_cpu() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    assert!(runtime.num_units() > 0);
    let stats = runtime.stats();
    assert!(stats.num_cpu > 0);
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_execute_on_cpu_unit() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let w = simple_f32_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = runtime.execute_on(0, w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_execute_optimal_dispatches() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let w = simple_f32_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = runtime.execute_optimal(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_execute_on_invalid_index_returns_error() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![]));
    let result = runtime.execute_on(9999, w).await;
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_execute_on_type_cpu() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
    let out = runtime
        .execute_on_type(ComputeUnitType::Cpu, w)
        .await
        .unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_units_by_type_cpu() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let cpu_units = runtime.units_by_type(ComputeUnitType::Cpu);
    assert!(!cpu_units.is_empty());
}

#[tokio::test]
#[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — use TOADSTOOL_WGPU_SAFE=1 on safe drivers"]
async fn test_execute_map_f32() {
    let runtime = UniversalRuntime::discover().await.unwrap();
    let input = vec![1.0f32, 2.0, 3.0];
    let out = runtime
        .execute_map_f32(input.clone(), |x| x * 2.0)
        .await
        .unwrap();
    assert_eq!(out.len(), 3);
}

// Tests using UniversalRuntime::new() — no wgpu discovery, safe for CI
#[tokio::test]
async fn test_runtime_new_with_cpu_units() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    assert_eq!(runtime.num_units(), 1);
    let stats = runtime.stats();
    assert_eq!(stats.num_cpu, 1);
}

#[tokio::test]
async fn test_runtime_new_execute_on_index() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = runtime.execute_on(0, w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
async fn test_runtime_new_execute_optimal() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(
        OperationType::Map,
        WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
    );
    let out = runtime.execute_optimal(w).await.unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
async fn test_runtime_new_execute_on_type_cpu() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
    let out = runtime
        .execute_on_type(ComputeUnitType::Cpu, w)
        .await
        .unwrap();
    assert!(matches!(out.data, WorkloadData::F32Vec(_)));
}

#[tokio::test]
async fn test_runtime_new_execute_on_invalid_index() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![]));
    let result = runtime.execute_on(9999, w).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_runtime_new_execute_on_type_nonexistent() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
    let result = runtime.execute_on_type(ComputeUnitType::GpuWgpu, w).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_runtime_new_units_by_type() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let cpu_units = runtime.units_by_type(ComputeUnitType::Cpu);
    assert_eq!(cpu_units.len(), 1);
    let gpu_units = runtime.units_by_type(ComputeUnitType::GpuWgpu);
    assert!(gpu_units.is_empty());
}

#[tokio::test]
async fn test_runtime_new_execute_map_f32() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let input = vec![1.0f32, 2.0, 3.0];
    let out = runtime
        .execute_map_f32(input.clone(), |x| x * 2.0)
        .await
        .unwrap();
    assert_eq!(out.len(), 3);
    // CPU Map uses x*2+1 internally (closure is not yet wired)
    assert!((out[0] - 3.0).abs() < 1e-5);
    assert!((out[1] - 5.0).abs() < 1e-5);
    assert!((out[2] - 7.0).abs() < 1e-5);
}

#[tokio::test]
async fn test_runtime_new_execute_map_f32_returns_vec() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let result = runtime.execute_map_f32(vec![1.0, 2.0], |x| x).await;
    assert!(result.is_ok());
    let out = result.unwrap();
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|&x| x > 0.0));
}

#[tokio::test]
async fn test_runtime_new_empty_units_fails_optimal() {
    let units: Vec<ComputeUnitDispatch> = vec![];
    let runtime = UniversalRuntime::new(units);
    let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
    let result = runtime.execute_optimal(w).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_runtime_new_stats_aggregation() {
    let cpu1 = crate::backends::CpuComputeUnit::discover();
    let cpu2 = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![
        ComputeUnitDispatch::Cpu(cpu1),
        ComputeUnitDispatch::Cpu(cpu2),
    ];
    let runtime = UniversalRuntime::new(units);
    let stats = runtime.stats();
    assert_eq!(stats.num_cpu, 2);
    assert!(stats.total_memory > 0);
    assert!(stats.total_compute_throughput > 0.0);
}

#[tokio::test]
async fn test_runtime_stats_display_multi_unit() {
    let cpu1 = crate::backends::CpuComputeUnit::discover();
    let cpu2 = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![
        ComputeUnitDispatch::Cpu(cpu1),
        ComputeUnitDispatch::Cpu(cpu2),
    ];
    let runtime = UniversalRuntime::new(units);
    let stats = runtime.stats();
    assert_eq!(stats.num_cpu, 2);
    let s = format!("{stats}");
    assert!(s.contains("CPU units: 2"));
}

#[tokio::test]
async fn test_runtime_stats_display_all_zero() {
    let stats = RuntimeStats::default();
    let s = format!("{stats}");
    assert!(s.contains("CPU units: 0"));
    assert!(s.contains("GPU units: 0"));
    assert!(s.contains("Neuromorphic units: 0"));
    assert!(s.contains("Custom units: 0"));
}

#[tokio::test]
async fn test_runtime_units_accessor() {
    let cpu = crate::backends::CpuComputeUnit::discover();
    let units: Vec<ComputeUnitDispatch> = vec![ComputeUnitDispatch::Cpu(cpu)];
    let runtime = UniversalRuntime::new(units);
    let units_ref = runtime.units();
    assert_eq!(units_ref.len(), 1);
    assert!(!units_ref[0].name().is_empty());
}
