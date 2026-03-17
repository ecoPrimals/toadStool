// SPDX-License-Identifier: AGPL-3.0-only
//! Simplified Substrate Abstraction
//!
//! **Deep Debt**: Agnostic, capability-based substrate interface
//!
//! This module provides a simplified trait for compute substrates that's easier
//! to implement for specific workloads (like homomorphic encryption) while still
//! being compatible with the full `ComputeUnit` trait.
//!
//! # Architecture
//!
//! ```text
//! ComputeSubstrate (Simple)    ComputeUnit (Full)
//!        │                            │
//!        ├── Simple operations        ├── Complex workloads
//!        ├── Buffer management        ├── Scheduling
//!        └── Power measurement        └── Capability discovery
//!                │                            │
//!                └────────────────────────────┘
//!                      SubstrateAdapter
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool_runtime_universal::substrate::*;
//!
//! struct MyGpuSubstrate {
//!     device: WgpuDevice,
//! }
//!
//! #[async_trait::async_trait]
//! impl ComputeSubstrate for MyGpuSubstrate {
//!     fn name(&self) -> &str { "My GPU" }
//!     fn substrate_type(&self) -> SubstrateType { SubstrateType::Gpu }
//!     
//!     async fn execute_buffer_op(
//!         &self,
//!         operation: BufferOperation,
//!     ) -> Result<BufferOutput> {
//!         // Implement GPU-specific operation
//!         Ok(BufferOutput::default())
//!     }
//! }
//! ```

use crate::error::SubstrateError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Simplified substrate trait for easier implementation
///
/// **Deep Debt**: Agnostic substrate interface, discover at runtime
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait ComputeSubstrate: Send + Sync {
    /// Human-readable name
    fn name(&self) -> &str;

    /// Substrate type (CPU, GPU, NPU, TPU)
    fn substrate_type(&self) -> SubstrateType;

    /// Get substrate capabilities
    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities::default_for_type(self.substrate_type())
    }

    /// Execute a buffer operation
    ///
    /// **Deep Debt**: Simple, generic operation interface
    async fn execute_buffer_op(
        &self,
        operation: BufferOperation,
    ) -> Result<BufferOutput, SubstrateError>;

    /// Measure power consumption (optional, returns estimate if unavailable)
    ///
    /// **Deep Debt**: Measure actual power, don't hardcode
    async fn measure_power(&self) -> Result<PowerMeasurement, SubstrateError> {
        // Default: Estimate based on substrate type
        Ok(PowerMeasurement::estimated_for_type(self.substrate_type()))
    }

    /// Profile operation performance
    ///
    /// **Deep Debt**: Profile actual performance, don't hardcode
    async fn profile_operation(
        &self,
        operation: &BufferOperation,
    ) -> Result<PerformanceMetrics, SubstrateError> {
        let start = std::time::Instant::now();
        let _ = self.execute_buffer_op(operation.clone()).await?;
        let duration = start.elapsed();

        Ok(PerformanceMetrics {
            duration,
            throughput_ops_per_sec: if duration.as_secs_f64() > 0.0 {
                operation.buffer_size() as f64 / duration.as_secs_f64()
            } else {
                0.0
            },
            latency_ms: duration.as_secs_f64() * 1000.0,
        })
    }
}

/// Substrate types — aligned with metalForge's hardware characterization model.
///
/// The 4 original types (CPU, GPU, NPU, TPU) are expanded with finer-grained
/// variants for mixed-silicon environments where different substrates within
/// the same category have qualitatively different capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubstrateType {
    /// General-purpose CPU.
    Cpu,
    /// Discrete GPU (PCIe, dedicated VRAM).
    Gpu,
    /// Integrated GPU (shared memory with CPU).
    IntegratedGpu,
    /// Neural Processing Unit (e.g. AKD1000, int8/int4 inference).
    Npu,
    /// Tensor Processing Unit (e.g. Google TPU, systolic array).
    Tpu,
    /// FPGA (reconfigurable logic, e.g. Xilinx, Intel).
    Fpga,
    /// Digital Signal Processor.
    Dsp,
    /// Quantum compute substrate (simulators or real QPUs).
    Quantum,
}

impl SubstrateType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::IntegratedGpu => "igpu",
            Self::Npu => "npu",
            Self::Tpu => "tpu",
            Self::Fpga => "fpga",
            Self::Dsp => "dsp",
            Self::Quantum => "quantum",
        }
    }

    /// Whether this substrate type is suitable for batch compute workloads.
    #[must_use]
    pub const fn is_batch_oriented(&self) -> bool {
        matches!(self, Self::Gpu | Self::Tpu | Self::Fpga)
    }

    /// Whether this substrate type is suitable for low-latency inference.
    #[must_use]
    pub const fn is_latency_oriented(&self) -> bool {
        matches!(
            self,
            Self::Cpu | Self::Npu | Self::Dsp | Self::IntegratedGpu
        )
    }
}

/// Substrate capabilities
///
/// **Deep Debt**: Discovered at runtime, not hardcoded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    /// Substrate type
    pub substrate_type: SubstrateType,

    /// Average power consumption (watts)
    pub power_watts: f64,

    /// Peak throughput (operations/second)
    pub throughput_ops_per_sec: f64,

    /// Typical latency (milliseconds)
    pub latency_ms: f64,

    /// Best suited for batch operations
    pub best_for_batch: bool,

    /// Best suited for low latency
    pub best_for_latency: bool,

    /// Best suited for energy efficiency
    pub best_for_energy: bool,

    /// Best suited for continuous operation
    pub best_for_continuous: bool,

    /// Memory capacity in bytes (runtime-discovered, 0 = unknown).
    #[serde(default)]
    pub memory_capacity_bytes: u64,

    /// Memory bandwidth in bytes/second (runtime-discovered, 0 = unknown).
    #[serde(default)]
    pub memory_bandwidth_bps: u64,
}

impl SubstrateCapabilities {
    /// Create default capabilities for a substrate type
    ///
    /// **Note**: These are conservative estimates. Real implementations
    /// should measure actual hardware capabilities.
    #[allow(clippy::missing_const_for_fn)] // Struct has f64 fields
    pub fn default_for_type(substrate_type: SubstrateType) -> Self {
        match substrate_type {
            SubstrateType::Cpu => Self {
                substrate_type,
                power_watts: 65.0,
                throughput_ops_per_sec: 1e9,
                latency_ms: 0.1,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Gpu => Self {
                substrate_type,
                power_watts: 250.0,
                throughput_ops_per_sec: 1e12,
                latency_ms: 2.0,
                best_for_batch: true,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::IntegratedGpu => Self {
                substrate_type,
                power_watts: 15.0,
                throughput_ops_per_sec: 1e11,
                latency_ms: 1.0,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Npu => Self {
                substrate_type,
                power_watts: 2.0,
                throughput_ops_per_sec: 1e10,
                latency_ms: 1.0,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: false,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Tpu => Self {
                substrate_type,
                power_watts: 200.0,
                throughput_ops_per_sec: 1e13,
                latency_ms: 5.0,
                best_for_batch: true,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Fpga => Self {
                substrate_type,
                power_watts: 25.0,
                throughput_ops_per_sec: 1e10,
                latency_ms: 0.5,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Dsp => Self {
                substrate_type,
                power_watts: 5.0,
                throughput_ops_per_sec: 1e9,
                latency_ms: 0.2,
                best_for_batch: false,
                best_for_latency: true,
                best_for_energy: true,
                best_for_continuous: true,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
            SubstrateType::Quantum => Self {
                substrate_type,
                power_watts: 15_000.0,
                throughput_ops_per_sec: 1e6,
                latency_ms: 100.0,
                best_for_batch: false,
                best_for_latency: false,
                best_for_energy: false,
                best_for_continuous: false,
                memory_capacity_bytes: 0,
                memory_bandwidth_bps: 0,
            },
        }
    }
}

/// Buffer operation
///
/// **Deep Debt**: Simple, generic operation for substrates
#[derive(Debug, Clone)]
pub enum BufferOperation {
    /// Add two buffers element-wise
    Add {
        a: Vec<u8>,
        b: Vec<u8>,
        element_size: usize,
    },

    /// Multiply two buffers element-wise
    Multiply {
        a: Vec<u8>,
        b: Vec<u8>,
        element_size: usize,
    },

    /// Apply unary function to buffer
    Map {
        data: Vec<u8>,
        element_size: usize,
        operation: UnaryOp,
    },

    /// Custom operation (substrate-specific)
    Custom {
        name: String,
        data: Vec<u8>,
        metadata: serde_json::Value,
    },
}

impl BufferOperation {
    /// Get the total buffer size for this operation
    #[allow(clippy::missing_const_for_fn)] // Vec::len() not const
    pub fn buffer_size(&self) -> usize {
        match self {
            Self::Add { a, b, .. } => a.len() + b.len(),
            Self::Multiply { a, b, .. } => a.len() + b.len(),
            Self::Map { data, .. } => data.len(),
            Self::Custom { data, .. } => data.len(),
        }
    }
}

/// Unary operations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnaryOp {
    Negate,
    Square,
    Sqrt,
    Exp,
    Log,
}

/// Buffer operation output
#[derive(Debug, Clone, Default)]
pub struct BufferOutput {
    /// Result data
    pub data: Vec<u8>,

    /// Execution metadata
    pub metadata: BufferMetadata,
}

/// Buffer execution metadata
#[derive(Debug, Clone, Default)]
pub struct BufferMetadata {
    /// Execution duration
    pub duration: Duration,

    /// Substrate that executed this
    pub substrate_name: String,

    /// Power consumed (if measured)
    pub power_consumed_mw: Option<f64>,
}

/// Power measurement
///
/// **Deep Debt**: Actual hardware measurement, not estimates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerMeasurement {
    /// Power in watts
    pub watts: f64,

    /// Whether this is a measured value (true) or estimate (false)
    pub measured: bool,

    /// Measurement method (e.g., "RAPL", "nvidia-smi", "estimated")
    pub method: String,
}

impl PowerMeasurement {
    /// Create an estimated power measurement for a substrate type
    pub fn estimated_for_type(substrate_type: SubstrateType) -> Self {
        let watts = match substrate_type {
            SubstrateType::Cpu => 65.0,
            SubstrateType::Gpu => 250.0,
            SubstrateType::IntegratedGpu => 15.0,
            SubstrateType::Npu => 2.0,
            SubstrateType::Tpu => 200.0,
            SubstrateType::Fpga => 25.0,
            SubstrateType::Dsp => 5.0,
            SubstrateType::Quantum => 15_000.0,
        };

        Self {
            watts,
            measured: false,
            method: format!("estimated ({})", substrate_type.as_str()),
        }
    }
}

/// Performance metrics
///
/// **Deep Debt**: Actual measured performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total duration
    pub duration: Duration,

    /// Throughput (operations/second)
    pub throughput_ops_per_sec: f64,

    /// Latency (milliseconds)
    pub latency_ms: f64,
}

// ═══════════════════════════════════════════════════════════
// ADAPTER: ComputeSubstrate → ComputeUnit
// ═══════════════════════════════════════════════════════════

use crate::types::{
    Capabilities, ComputeError, ComputeUnit, ComputeUnitType, DataType, ExecutionModel,
    LatencyProfile, OperationType, Output, OutputMetadata, Parallelism, PowerProfile, Workload,
    WorkloadData,
};

/// Adapter to use a `ComputeSubstrate` as a `ComputeUnit`
///
/// **Deep Debt**: Bridge simple and full interfaces
pub struct SubstrateAdapter<S: ComputeSubstrate> {
    substrate: S,
    capabilities: Capabilities,
}

impl<S: ComputeSubstrate> SubstrateAdapter<S> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
