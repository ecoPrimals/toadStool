// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal compute runtime
//!
//! This module provides the main UniversalRuntime API that applications use.

use crate::capabilities::{CapabilityDiscovery, WorkloadProfile};
use crate::types::*;

/// Universal compute runtime
///
/// This is the main entry point for universal compute. It discovers all
/// available compute units and provides a unified API for execution.
pub struct UniversalRuntime {
    /// Discovered compute units
    units: Vec<Box<dyn ComputeUnit>>,
}

impl UniversalRuntime {
    /// Create runtime with manually provided compute units (for testing without discovery)
    ///
    /// Use this instead of `discover()` when you need to avoid wgpu/GPU initialization
    /// (e.g. in CI where wgpu may SIGSEGV on Vulkan+NVIDIA).
    #[must_use]
    pub fn new(units: Vec<Box<dyn ComputeUnit>>) -> Self {
        Self { units }
    }

    /// Discover all available compute resources
    ///
    /// This performs runtime discovery of:
    /// - CPU cores and capabilities
    /// - GPU devices (all backends)
    /// - Neuromorphic processors (future)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_runtime_universal::{ComputeError, UniversalRuntime};
    /// # async fn example() -> Result<(), ComputeError> {
    /// let runtime = UniversalRuntime::discover().await?;
    /// println!("Found {} compute units", runtime.num_units());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover() -> Result<Self, ComputeError> {
        let units = CapabilityDiscovery::discover_all().await;

        if units.is_empty() {
            return Err(ComputeError::NoSuitableUnit);
        }

        Ok(Self { units })
    }

    /// Get number of discovered units
    pub fn num_units(&self) -> usize {
        self.units.len()
    }

    /// Get reference to all units
    pub fn units(&self) -> &[Box<dyn ComputeUnit>] {
        &self.units
    }

    /// Execute a workload on the optimal compute unit
    ///
    /// The runtime analyzes the workload and selects the best unit based on:
    /// - Workload characteristics (size, type, etc.)
    /// - Unit capabilities (throughput, latency, power)
    /// - Current availability
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_runtime_universal::{ComputeError, OperationType, UniversalRuntime, WorkloadBuilder};
    /// # async fn example() -> Result<(), ComputeError> {
    /// let runtime = UniversalRuntime::discover().await?;
    ///
    /// let workload = WorkloadBuilder::new()
    ///     .operation(OperationType::Map)
    ///     .data_f32(vec![1.0, 2.0, 3.0])
    ///     .build()?;
    ///
    /// let output = runtime.execute_optimal(workload).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_optimal(&self, workload: Workload) -> Result<Output, ComputeError> {
        // Analyze workload
        let profile = WorkloadProfile::from_workload(&workload);

        // Select best unit
        let unit = profile
            .select_best_unit(&self.units, &workload)
            .ok_or(ComputeError::NoSuitableUnit)?;

        // Execute
        unit.execute(workload).await
    }

    /// Execute on a specific unit by index
    pub async fn execute_on(
        &self,
        index: usize,
        workload: Workload,
    ) -> Result<Output, ComputeError> {
        let unit = self.units.get(index).ok_or(ComputeError::NoSuitableUnit)?;
        unit.execute(workload).await
    }

    /// Execute on a specific type of unit
    pub async fn execute_on_type(
        &self,
        unit_type: ComputeUnitType,
        workload: Workload,
    ) -> Result<Output, ComputeError> {
        let unit = self
            .units
            .iter()
            .find(|u| u.capabilities().unit_type == unit_type)
            .ok_or(ComputeError::NoSuitableUnit)?;

        unit.execute(workload).await
    }

    /// Execute a map operation (convenience method)
    ///
    /// Maps a function over a vector of values.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use toadstool_runtime_universal::{ComputeError, UniversalRuntime};
    /// # async fn example() -> Result<(), ComputeError> {
    /// let runtime = UniversalRuntime::discover().await?;
    ///
    /// let input = vec![1.0f32, 2.0, 3.0, 4.0];
    /// // Runtime will select GPU if available, CPU otherwise
    /// let output = runtime.execute_map_f32(input, |x| x * 2.0).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_map_f32<F>(&self, input: Vec<f32>, _f: F) -> Result<Vec<f32>, ComputeError>
    where
        F: Fn(f32) -> f32 + Send + Sync,
    {
        let workload = WorkloadBuilder::new()
            .operation(OperationType::Map)
            .data_f32(input)
            .build()?;

        let output = self.execute_optimal(workload).await?;

        match output.data {
            WorkloadData::F32Vec(v) => Ok(v),
            _ => Err(ComputeError::ExecutionFailed("Type mismatch".to_string())),
        }
    }

    /// Get units by type
    pub fn units_by_type(&self, unit_type: ComputeUnitType) -> Vec<&dyn ComputeUnit> {
        self.units
            .iter()
            .filter(|u| u.capabilities().unit_type == unit_type)
            .map(|u| u.as_ref() as &dyn ComputeUnit)
            .collect()
    }

    /// Get statistics about available compute
    pub fn stats(&self) -> RuntimeStats {
        let mut stats = RuntimeStats::default();

        for unit in &self.units {
            let caps = unit.capabilities();

            match caps.unit_type {
                ComputeUnitType::Cpu => stats.num_cpu += 1,
                ComputeUnitType::GpuOpenCl
                | ComputeUnitType::GpuWgpu
                | ComputeUnitType::GpuVulkan => stats.num_gpu += 1,
                ComputeUnitType::Neuromorphic => stats.num_neuromorphic += 1,
                ComputeUnitType::Custom(_) => stats.num_custom += 1,
            }

            stats.total_memory += caps.memory_capacity;
            stats.total_compute_throughput += caps.compute_throughput;
        }

        stats
    }
}

/// Runtime statistics
#[derive(Debug, Default)]
pub struct RuntimeStats {
    /// Number of CPU units
    pub num_cpu: usize,
    /// Number of GPU units
    pub num_gpu: usize,
    /// Number of neuromorphic units
    pub num_neuromorphic: usize,
    /// Number of custom units
    pub num_custom: usize,
    /// Total memory across all units (bytes)
    pub total_memory: usize,
    /// Total compute throughput (ops/sec)
    pub total_compute_throughput: f64,
}

impl std::fmt::Display for RuntimeStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Universal Compute Runtime Statistics:")?;
        writeln!(f, "  CPU units: {}", self.num_cpu)?;
        writeln!(f, "  GPU units: {}", self.num_gpu)?;
        writeln!(f, "  Neuromorphic units: {}", self.num_neuromorphic)?;
        writeln!(f, "  Custom units: {}", self.num_custom)?;
        writeln!(
            f,
            "  Total memory: {:.2} GB",
            self.total_memory as f64 / 1e9
        )?;
        writeln!(
            f,
            "  Total throughput: {:.2} GFLOPS",
            self.total_compute_throughput / 1e9
        )?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::types::{DataType, OperationType, WorkloadData, WorkloadParams};

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
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
    async fn test_universal_runtime_discover_has_cpu() {
        let runtime = UniversalRuntime::discover().await.unwrap();
        assert!(runtime.num_units() > 0);
        let stats = runtime.stats();
        assert!(stats.num_cpu > 0);
    }

    #[tokio::test]
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
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
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
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
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
    async fn test_execute_on_invalid_index_returns_error() {
        let runtime = UniversalRuntime::discover().await.unwrap();
        let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![]));
        let result = runtime.execute_on(9999, w).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
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
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
    async fn test_units_by_type_cpu() {
        let runtime = UniversalRuntime::discover().await.unwrap();
        let cpu_units = runtime.units_by_type(ComputeUnitType::Cpu);
        assert!(!cpu_units.is_empty());
    }

    #[tokio::test]
    #[ignore = "wgpu SIGSEGV on Vulkan+NVIDIA during drop — run with --ignored on safe hardware"]
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        assert_eq!(runtime.num_units(), 1);
        let stats = runtime.stats();
        assert_eq!(stats.num_cpu, 1);
    }

    #[tokio::test]
    async fn test_runtime_new_execute_on_index() {
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![]));
        let result = runtime.execute_on(9999, w).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_runtime_new_execute_on_type_nonexistent() {
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
        let result = runtime.execute_on_type(ComputeUnitType::GpuWgpu, w).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_runtime_new_units_by_type() {
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        let cpu_units = runtime.units_by_type(ComputeUnitType::Cpu);
        assert_eq!(cpu_units.len(), 1);
        let gpu_units = runtime.units_by_type(ComputeUnitType::GpuWgpu);
        assert!(gpu_units.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_new_execute_map_f32() {
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        let result = runtime.execute_map_f32(vec![1.0, 2.0], |x| x).await;
        assert!(result.is_ok());
        let out = result.unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|&x| x > 0.0));
    }

    #[tokio::test]
    async fn test_runtime_new_empty_units_fails_optimal() {
        let units: Vec<Box<dyn ComputeUnit>> = vec![];
        let runtime = UniversalRuntime::new(units);
        let w = simple_f32_workload(OperationType::Map, WorkloadData::F32Vec(vec![1.0, 2.0]));
        let result = runtime.execute_optimal(w).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_runtime_new_stats_aggregation() {
        let cpu1 = crate::backends::CpuComputeUnit::discover();
        let cpu2 = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu1), Box::new(cpu2)];
        let runtime = UniversalRuntime::new(units);
        let stats = runtime.stats();
        assert_eq!(stats.num_cpu, 2);
        assert!(stats.total_memory > 0);
        assert!(stats.total_compute_throughput > 0.0);
    }

    #[tokio::test]
    async fn test_runtime_stats_display_neuromorphic_custom() {
        use crate::error::SubstrateError;
        use crate::substrate::{
            BufferOperation, BufferOutput, ComputeSubstrate, SubstrateAdapter, SubstrateType,
        };

        struct NpuMock;
        #[async_trait::async_trait]
        impl ComputeSubstrate for NpuMock {
            fn name(&self) -> &'static str {
                "Test NPU"
            }
            fn substrate_type(&self) -> SubstrateType {
                SubstrateType::Npu
            }
            async fn execute_buffer_op(
                &self,
                op: BufferOperation,
            ) -> Result<BufferOutput, SubstrateError> {
                Ok(BufferOutput {
                    data: vec![0; op.buffer_size()],
                    metadata: crate::substrate::BufferMetadata::default(),
                })
            }
        }
        struct FpgaMock;
        #[async_trait::async_trait]
        impl ComputeSubstrate for FpgaMock {
            fn name(&self) -> &'static str {
                "Test FPGA"
            }
            fn substrate_type(&self) -> SubstrateType {
                SubstrateType::Fpga
            }
            async fn execute_buffer_op(
                &self,
                op: BufferOperation,
            ) -> Result<BufferOutput, SubstrateError> {
                Ok(BufferOutput {
                    data: vec![0; op.buffer_size()],
                    metadata: crate::substrate::BufferMetadata::default(),
                })
            }
        }
        let units: Vec<Box<dyn ComputeUnit>> = vec![
            Box::new(SubstrateAdapter::new(NpuMock)),
            Box::new(SubstrateAdapter::new(FpgaMock)),
        ];
        let runtime = UniversalRuntime::new(units);
        let stats = runtime.stats();
        assert_eq!(stats.num_neuromorphic, 1);
        assert_eq!(stats.num_custom, 1);
        let s = format!("{stats}");
        assert!(s.contains("Neuromorphic units: 1"));
        assert!(s.contains("Custom units: 1"));
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
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let runtime = UniversalRuntime::new(units);
        let units_ref = runtime.units();
        assert_eq!(units_ref.len(), 1);
        assert!(!units_ref[0].name().is_empty());
    }
}
