//! OpenCL compute unit implementation (placeholder)
//!
//! This shows how OpenCL GPUs are treated as ComputeUnits.
//! Full implementation would use the ocl crate.

use crate::types::*;

/// OpenCL compute unit
pub struct OpenClComputeUnit {
    name: String,
    capabilities: Capabilities,
    _device: ocl::Device,
}

impl OpenClComputeUnit {
    /// Create from an OpenCL device
    pub fn from_device(device: ocl::Device) -> Result<Self, ComputeError> {
        // Query device properties
        let name = device
            .name()
            .map_err(|e| ComputeError::BackendError(e.into()))?;

        let max_compute_units = device
            .max_compute_units()
            .map_err(|e| ComputeError::BackendError(e.into()))?;

        let global_mem_size = device
            .global_mem_size()
            .map_err(|e| ComputeError::BackendError(e.into()))?;

        // Estimate throughput based on compute units
        let compute_throughput = (max_compute_units as f64) * 1e9; // Rough estimate

        let capabilities = Capabilities {
            unit_type: ComputeUnitType::GpuOpenCl,
            parallelism: Parallelism {
                num_units: max_compute_units as usize,
                model: ExecutionModel::Simd,
            },
            power_profile: PowerProfile::High, // GPUs typically high power
            latency: LatencyProfile {
                typical_ms: 1, // GPU has some latency for kernel launch
                deterministic: false,
            },
            memory_capacity: global_mem_size as usize,
            memory_bandwidth: 500_000_000_000, // ~500 GB/s typical for modern GPUs
            compute_throughput,
            optimal_batch_size: 10_000, // GPUs like large batches
            supported_ops: vec![
                OperationType::Map,
                OperationType::Reduce,
                OperationType::MatMul,
                OperationType::Conv,
            ],
            supported_types: vec![
                DataType::F32,
                DataType::F64,
                DataType::I32,
                DataType::I64,
            ],
        };

        Ok(Self {
            name,
            capabilities,
            _device: device,
        })
    }
}

#[async_trait::async_trait]
impl ComputeUnit for OpenClComputeUnit {
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, _workload: Workload) -> Result<Output, ComputeError> {
        // Placeholder - full implementation would use ocl crate
        Err(ComputeError::ExecutionFailed(
            "OpenCL execution not yet fully implemented in universal runtime".to_string(),
        ))
    }
}

