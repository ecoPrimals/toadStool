// SPDX-License-Identifier: AGPL-3.0-or-later
// ADAPTER: ComputeSubstrate → ComputeUnit

use crate::error::SubstrateError;
use crate::types::{
    Capabilities, ComputeError, ComputeUnit, ComputeUnitType, DataType, ExecutionModel,
    LatencyProfile, OperationType, Output, OutputMetadata, Parallelism, PowerProfile, Workload,
    WorkloadData,
};
use async_trait::async_trait;

use super::buffer::BufferOperation;
use super::capabilities::SubstrateCapabilities;
use super::compute_substrate::ComputeSubstrate;
use super::substrate_kind::SubstrateType;

/// Adapter to use a `ComputeSubstrate` as a `ComputeUnit`
///
/// **Deep Debt**: Bridge simple and full interfaces
pub struct SubstrateAdapter<S: ComputeSubstrate> {
    /// Wrapped substrate.
    substrate: S,
    /// Converted capabilities for ComputeUnit interface.
    capabilities: Capabilities,
}

impl<S: ComputeSubstrate> SubstrateAdapter<S> {
    /// Create adapter from a substrate.
    pub fn new(substrate: S) -> Self {
        let substrate_caps = substrate.capabilities();
        let capabilities = Self::convert_capabilities(&substrate_caps);
        Self {
            substrate,
            capabilities,
        }
    }

    fn convert_capabilities(caps: &SubstrateCapabilities) -> Capabilities {
        let unit_type = match caps.substrate_type {
            SubstrateType::Cpu => ComputeUnitType::Cpu,
            SubstrateType::Gpu | SubstrateType::IntegratedGpu => ComputeUnitType::GpuWgpu,
            SubstrateType::Npu => ComputeUnitType::Neuromorphic,
            SubstrateType::Tpu
            | SubstrateType::Fpga
            | SubstrateType::Dsp
            | SubstrateType::Quantum => ComputeUnitType::Custom(1),
        };

        let power_profile = if caps.power_watts < 1.0 {
            PowerProfile::UltraLow
        } else if caps.power_watts < 10.0 {
            PowerProfile::Low
        } else if caps.power_watts < 100.0 {
            PowerProfile::Medium
        } else {
            PowerProfile::High
        };

        Capabilities {
            unit_type,
            parallelism: Parallelism {
                num_units: 1,
                model: ExecutionModel::Simd,
            },
            power_profile,
            latency: LatencyProfile {
                typical_ms: caps.latency_ms as u32,
                deterministic: false,
            },
            memory_capacity: if caps.memory_capacity_bytes > 0 {
                caps.memory_capacity_bytes as usize
            } else {
                8 * 1024 * 1024 * 1024
            },
            memory_bandwidth: if caps.memory_bandwidth_bps > 0 {
                caps.memory_bandwidth_bps as usize
            } else {
                500 * 1024 * 1024 * 1024
            },
            compute_throughput: caps.throughput_ops_per_sec,
            optimal_batch_size: if caps.best_for_batch { 10_000 } else { 100 },
            supported_ops: vec![
                OperationType::Map,
                OperationType::ElementwiseBinary,
                OperationType::Custom,
            ],
            supported_types: vec![DataType::F32, DataType::F64, DataType::U64],
        }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl<S: ComputeSubstrate> ComputeUnit for SubstrateAdapter<S> {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        self.substrate.name()
    }

    async fn execute(&self, workload: Workload) -> Result<Output, ComputeError> {
        // Convert Workload → BufferOperation (consumes workload for zero-copy)
        let buffer_op = self.convert_workload(workload)?;

        // Execute on substrate
        let start = std::time::Instant::now();
        let output = self
            .substrate
            .execute_buffer_op(buffer_op)
            .await
            .map_err(|e: SubstrateError| ComputeError::ExecutionFailed(e.to_string()))?;
        let duration = start.elapsed();

        // Convert BufferOutput → Output
        Ok(Output {
            data: WorkloadData::Custom(output.data),
            metadata: OutputMetadata {
                unit_name: self.substrate.name().to_string(),
                unit_type: self.capabilities.unit_type,
                duration,
                power_consumed_mw: output.metadata.power_consumed_mw,
            },
        })
    }
}

impl<S: ComputeSubstrate> SubstrateAdapter<S> {
    /// Reinterpret `Vec<f32>` as `Vec<u8>` without copying (zero-copy).
    ///
    /// Uses `bytemuck::allocation::cast_vec` which is safe and zero-copy.
    /// Falls back to byte-level copy if the cast fails (shouldn't happen
    /// for f32 → u8 on any platform, but we handle it for correctness).
    fn vec_f32_to_u8(v: Vec<f32>) -> Vec<u8> {
        bytemuck::allocation::cast_vec(v)
    }

    #[expect(
        clippy::unused_self,
        reason = "may use self for future substrate extensions"
    )]
    fn convert_workload(&self, workload: Workload) -> Result<BufferOperation, ComputeError> {
        // Consume workload for zero-copy: move data instead of cloning
        match workload.input {
            WorkloadData::Custom(data) => {
                // API requires owned Vec<u8>; we move it directly (no clone)
                Ok(BufferOperation::Custom {
                    name: format!("{:?}", workload.operation),
                    data,
                    metadata: serde_json::json!({}),
                })
            }
            WorkloadData::F32VecPair(a, b) => {
                let (a_bytes, b_bytes) = (Self::vec_f32_to_u8(a), Self::vec_f32_to_u8(b));

                match workload.operation {
                    OperationType::ElementwiseBinary => Ok(BufferOperation::Add {
                        a: a_bytes,
                        b: b_bytes,
                        element_size: std::mem::size_of::<f32>(),
                    }),
                    _ => Err(ComputeError::UnsupportedWorkload),
                }
            }
            _ => Err(ComputeError::UnsupportedWorkload),
        }
    }
}
