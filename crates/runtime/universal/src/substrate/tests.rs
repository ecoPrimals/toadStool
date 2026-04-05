// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use async_trait::async_trait;

use crate::error::SubstrateError;
use crate::types::{ComputeUnit, ComputeUnitType};

use super::{
    BufferMetadata, BufferOperation, BufferOutput, ComputeSubstrate, SubstrateAdapter,
    SubstrateCapabilities, SubstrateType,
};

struct MockSubstrate;

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ComputeSubstrate for MockSubstrate {
    fn name(&self) -> &'static str {
        "Mock Substrate"
    }

    fn substrate_type(&self) -> SubstrateType {
        SubstrateType::Cpu
    }

    async fn execute_buffer_op(
        &self,
        operation: BufferOperation,
    ) -> Result<BufferOutput, SubstrateError> {
        Ok(BufferOutput {
            data: vec![0; operation.buffer_size()],
            metadata: BufferMetadata {
                duration: Duration::from_millis(10),
                substrate_name: self.name().to_string(),
                power_consumed_mw: Some(65000.0),
            },
        })
    }
}

#[tokio::test]
async fn test_substrate_trait() {
    let substrate = MockSubstrate;
    assert_eq!(substrate.name(), "Mock Substrate");
    assert_eq!(substrate.substrate_type(), SubstrateType::Cpu);

    let op = BufferOperation::Add {
        a: vec![1, 2, 3],
        b: vec![4, 5, 6],
        element_size: 1,
    };

    let result = substrate.execute_buffer_op(op).await.unwrap();
    assert_eq!(result.data.len(), 6);
}

#[tokio::test]
async fn test_substrate_adapter() {
    let substrate = MockSubstrate;
    let adapter = SubstrateAdapter::new(substrate);

    assert_eq!(adapter.name(), "Mock Substrate");
    assert_eq!(adapter.capabilities().unit_type, ComputeUnitType::Cpu);
}

#[test]
fn test_substrate_capabilities() {
    let cpu_caps = SubstrateCapabilities::default_for_type(SubstrateType::Cpu);
    assert_eq!(cpu_caps.substrate_type, SubstrateType::Cpu);
    assert!(cpu_caps.best_for_latency);

    let gpu_caps = SubstrateCapabilities::default_for_type(SubstrateType::Gpu);
    assert_eq!(gpu_caps.substrate_type, SubstrateType::Gpu);
    assert!(gpu_caps.best_for_batch);

    let npu_caps = SubstrateCapabilities::default_for_type(SubstrateType::Npu);
    assert_eq!(npu_caps.substrate_type, SubstrateType::Npu);
    assert!(npu_caps.best_for_energy);

    let igpu_caps = SubstrateCapabilities::default_for_type(SubstrateType::IntegratedGpu);
    assert_eq!(igpu_caps.substrate_type, SubstrateType::IntegratedGpu);
    assert!(igpu_caps.best_for_energy);

    let fpga_caps = SubstrateCapabilities::default_for_type(SubstrateType::Fpga);
    assert!(fpga_caps.best_for_latency);

    let quantum_caps = SubstrateCapabilities::default_for_type(SubstrateType::Quantum);
    assert!(!quantum_caps.best_for_batch);
}

#[test]
fn test_substrate_type_classification() {
    assert!(SubstrateType::Gpu.is_batch_oriented());
    assert!(SubstrateType::Tpu.is_batch_oriented());
    assert!(!SubstrateType::Cpu.is_batch_oriented());

    assert!(SubstrateType::Cpu.is_latency_oriented());
    assert!(SubstrateType::Npu.is_latency_oriented());
    assert!(!SubstrateType::Gpu.is_latency_oriented());
}
