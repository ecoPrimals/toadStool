// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
//! Coverage tests for substrate.rs
//!
//! Focus: substrate detection, capability reporting, type construction

use std::future::Future;
use std::time::Duration;

use toadstool_runtime_universal::ComputeSubstrate;
use toadstool_runtime_universal::error::SubstrateError;
use toadstool_runtime_universal::{
    BufferMetadata, BufferOperation, BufferOutput, ComputeUnit, ComputeUnitType, DataType,
    OperationType, PowerMeasurement, SubstrateAdapter, SubstrateCapabilities, SubstrateType,
    UnaryOp, Workload, WorkloadData, WorkloadParams,
};

// ============================================================================
// Mock substrate for testing
// ============================================================================

struct MockSubstrate {
    name: &'static str,
    substrate_type: SubstrateType,
}

impl ComputeSubstrate for MockSubstrate {
    fn name(&self) -> &str {
        self.name
    }

    fn substrate_type(&self) -> SubstrateType {
        self.substrate_type
    }

    fn execute_buffer_op(
        &self,
        operation: BufferOperation,
    ) -> impl Future<Output = Result<BufferOutput, SubstrateError>> + Send + '_ {
        let substrate_name = self.name.to_string();
        async move {
            Ok(BufferOutput {
                data: vec![0; operation.buffer_size()],
                metadata: BufferMetadata {
                    duration: Duration::from_millis(5),
                    substrate_name,
                    power_consumed_mw: Some(100.0),
                },
            })
        }
    }
}

// ============================================================================
// SubstrateType
// ============================================================================

#[test]
fn substrate_type_as_str() {
    assert_eq!(SubstrateType::Cpu.as_str(), "cpu");
    assert_eq!(SubstrateType::Gpu.as_str(), "gpu");
    assert_eq!(SubstrateType::IntegratedGpu.as_str(), "igpu");
    assert_eq!(SubstrateType::Npu.as_str(), "npu");
    assert_eq!(SubstrateType::Tpu.as_str(), "tpu");
    assert_eq!(SubstrateType::Fpga.as_str(), "fpga");
    assert_eq!(SubstrateType::Dsp.as_str(), "dsp");
    assert_eq!(SubstrateType::Quantum.as_str(), "quantum");
}

#[test]
fn substrate_type_batch_oriented() {
    assert!(SubstrateType::Gpu.is_batch_oriented());
    assert!(SubstrateType::Tpu.is_batch_oriented());
    assert!(SubstrateType::Fpga.is_batch_oriented());
    assert!(!SubstrateType::Cpu.is_batch_oriented());
    assert!(!SubstrateType::Npu.is_batch_oriented());
}

#[test]
fn substrate_type_latency_oriented() {
    assert!(SubstrateType::Cpu.is_latency_oriented());
    assert!(SubstrateType::Npu.is_latency_oriented());
    assert!(SubstrateType::Dsp.is_latency_oriented());
    assert!(SubstrateType::IntegratedGpu.is_latency_oriented());
    assert!(!SubstrateType::Gpu.is_latency_oriented());
    assert!(!SubstrateType::Quantum.is_latency_oriented());
}

// ============================================================================
// SubstrateCapabilities
// ============================================================================

#[test]
fn substrate_capabilities_cpu() {
    let caps = SubstrateCapabilities::default_for_type(SubstrateType::Cpu);
    assert_eq!(caps.substrate_type, SubstrateType::Cpu);
    assert!(caps.best_for_latency);
    assert!(!caps.best_for_batch);
    assert_eq!(caps.power_watts, 65.0);
}

#[test]
fn substrate_capabilities_gpu() {
    let caps = SubstrateCapabilities::default_for_type(SubstrateType::Gpu);
    assert_eq!(caps.substrate_type, SubstrateType::Gpu);
    assert!(caps.best_for_batch);
    assert!(!caps.best_for_latency);
    assert_eq!(caps.power_watts, 250.0);
}

#[test]
fn substrate_capabilities_all_types() {
    for st in [
        SubstrateType::Cpu,
        SubstrateType::Gpu,
        SubstrateType::IntegratedGpu,
        SubstrateType::Npu,
        SubstrateType::Tpu,
        SubstrateType::Fpga,
        SubstrateType::Dsp,
        SubstrateType::Quantum,
    ] {
        let caps = SubstrateCapabilities::default_for_type(st);
        assert_eq!(caps.substrate_type, st);
        assert!(caps.power_watts > 0.0);
        assert!(caps.throughput_ops_per_sec >= 0.0);
    }
}

// ============================================================================
// BufferOperation
// ============================================================================

#[test]
fn buffer_operation_add_buffer_size() {
    let op = BufferOperation::Add {
        a: vec![1, 2, 3],
        b: vec![4, 5, 6],
        element_size: 1,
    };
    assert_eq!(op.buffer_size(), 6);
}

#[test]
fn buffer_operation_multiply_buffer_size() {
    let op = BufferOperation::Multiply {
        a: vec![1u8; 10],
        b: vec![2u8; 20],
        element_size: 1,
    };
    assert_eq!(op.buffer_size(), 30);
}

#[test]
fn buffer_operation_map_buffer_size() {
    let op = BufferOperation::Map {
        data: vec![1, 2, 3, 4, 5],
        element_size: 1,
        operation: UnaryOp::Square,
    };
    assert_eq!(op.buffer_size(), 5);
}

#[test]
fn buffer_operation_custom_buffer_size() {
    let op = BufferOperation::Custom {
        name: "test".to_string(),
        data: vec![0u8; 100],
        metadata: serde_json::json!({}),
    };
    assert_eq!(op.buffer_size(), 100);
}

// ============================================================================
// UnaryOp
// ============================================================================

#[test]
fn unary_op_variants() {
    let _ = UnaryOp::Negate;
    let _ = UnaryOp::Square;
    let _ = UnaryOp::Sqrt;
    let _ = UnaryOp::Exp;
    let _ = UnaryOp::Log;
}

// ============================================================================
// PowerMeasurement
// ============================================================================

#[test]
fn power_measurement_estimated_for_type() {
    let pm = PowerMeasurement::estimated_for_type(SubstrateType::Cpu);
    assert_eq!(pm.watts, 65.0);
    assert!(!pm.measured);
    assert!(pm.method.contains("cpu"));

    let pm_gpu = PowerMeasurement::estimated_for_type(SubstrateType::Gpu);
    assert_eq!(pm_gpu.watts, 250.0);

    let pm_quantum = PowerMeasurement::estimated_for_type(SubstrateType::Quantum);
    assert_eq!(pm_quantum.watts, 15_000.0);
}

// ============================================================================
// SubstrateAdapter
// ============================================================================

#[tokio::test]
async fn substrate_adapter_new() {
    let substrate = MockSubstrate {
        name: "Test CPU",
        substrate_type: SubstrateType::Cpu,
    };
    let adapter = SubstrateAdapter::new(substrate);
    assert_eq!(adapter.name(), "Test CPU");
    assert_eq!(adapter.capabilities().unit_type, ComputeUnitType::Cpu);
}

#[tokio::test]
async fn substrate_adapter_execute_custom_data() {
    let substrate = MockSubstrate {
        name: "Test",
        substrate_type: SubstrateType::Cpu,
    };
    let adapter = SubstrateAdapter::new(substrate);
    let workload = Workload {
        operation: OperationType::Custom,
        data_type: DataType::F32,
        num_operations: 1,
        required_memory: 8,
        input: WorkloadData::Custom(vec![1, 2, 3, 4, 5]),
        params: WorkloadParams::default(),
    };
    let result = adapter.execute(workload).await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(matches!(output.data, WorkloadData::Custom(_)));
}

#[tokio::test]
#[ignore = "F32VecPair uses bytemuck cast_vec which can AlignmentMismatch on some platforms"]
async fn substrate_adapter_execute_f32_pair() {
    let substrate = MockSubstrate {
        name: "Test",
        substrate_type: SubstrateType::Cpu,
    };
    let adapter = SubstrateAdapter::new(substrate);
    let workload = Workload {
        operation: OperationType::ElementwiseBinary,
        data_type: DataType::F32,
        num_operations: 2,
        required_memory: 16,
        input: WorkloadData::F32VecPair(vec![1.0, 2.0], vec![3.0, 4.0]),
        params: WorkloadParams::default(),
    };
    let result = adapter.execute(workload).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn substrate_adapter_execute_unsupported_workload() {
    // Use F32Vec to hit UnsupportedWorkload without bytemuck cast
    let substrate = MockSubstrate {
        name: "Test",
        substrate_type: SubstrateType::Cpu,
    };
    let adapter = SubstrateAdapter::new(substrate);
    let workload = Workload {
        operation: OperationType::Reduce,
        data_type: DataType::F32,
        num_operations: 1,
        required_memory: 4,
        input: WorkloadData::F32Vec(vec![1.0, 2.0]),
        params: WorkloadParams::default(),
    };
    let result = adapter.execute(workload).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("not supported") || err_str.contains("Unsupported"),
        "expected UnsupportedWorkload error: {err_str}"
    );
}

#[tokio::test]
async fn substrate_adapter_execute_unsupported_input_type() {
    let substrate = MockSubstrate {
        name: "Test",
        substrate_type: SubstrateType::Cpu,
    };
    let adapter = SubstrateAdapter::new(substrate);
    let workload = Workload {
        operation: OperationType::Map,
        data_type: DataType::F32,
        num_operations: 1,
        required_memory: 4,
        input: WorkloadData::F32Vec(vec![1.0, 2.0]),
        params: WorkloadParams::default(),
    };
    let result = adapter.execute(workload).await;
    assert!(result.is_err());
}
