// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::universal::{ComputeBuffer, OptimizationHints, Precision};

#[test]
fn test_cpu_resource_creation() {
    let cpu = CpuComputeResource::new()
        .expect("CPU resource creation should never fail on supported platforms");
    assert!(cpu.num_cores > 0);
    assert!(cpu.capabilities().memory.total_bytes > 0);
}

#[test]
fn test_cpu_capabilities() {
    let cpu = CpuComputeResource::new().expect("CPU resource creation should never fail");
    let caps = cpu.capabilities();

    // CPU should support these
    assert!(caps.precision.fp64); // CPUs excel at fp64
    assert!(caps.operations.branching_efficiency == BranchingEfficiency::High);
    assert!(caps.operations.atomic_ops);
    assert!(caps.memory.unified_memory);
}

#[tokio::test]
async fn test_cpu_can_execute() {
    let cpu = CpuComputeResource::new().expect("CPU resource creation should never fail");

    let requirements = ComputeRequirements {
        min_parallel_threads: 4,
        memory_bytes: 1024 * 1024,
        precision: Precision::Fp32,
        operations: vec![Operation::GeneralCompute],
        ..Default::default()
    };

    assert!(cpu.can_execute(&requirements));
}

#[tokio::test]
async fn test_cpu_context_creation() {
    let cpu = CpuComputeResource::new().expect("CPU resource creation should never fail");
    let context = cpu
        .create_context()
        .await
        .expect("Context creation should succeed");
    assert_eq!(context.resource_id(), "cpu-main");
}

#[tokio::test]
async fn test_cpu_execution() {
    let cpu = CpuComputeResource::new().expect("CPU resource creation should never fail");
    let mut context = cpu
        .create_context()
        .await
        .expect("Context creation should succeed");

    let workload = UniversalWorkload {
        id: "test-cpu-workload".to_string(),
        requirements: ComputeRequirements {
            min_parallel_threads: 4,
            memory_bytes: 1024,
            precision: Precision::Fp32,
            operations: vec![Operation::GeneralCompute],
            ..Default::default()
        },
        kernel: UniversalKernel::Operation {
            operation: Operation::GeneralCompute,
            parameters: std::collections::HashMap::new(),
        },
        inputs: vec![ComputeBuffer {
            name: "input".to_string(),
            data: vec![1, 2, 3, 4, 5].into(),
            element_type: crate::types::DataType::UInt8,
        }],
        output_size: 5,
        hints: OptimizationHints::default(),
    };

    let result = context
        .execute(&workload)
        .await
        .expect("CPU execution should succeed for valid workload");
    assert!(result.outputs.contains_key("output_0"));
    assert!(result.metrics.execution_time.as_micros() > 0);
}
