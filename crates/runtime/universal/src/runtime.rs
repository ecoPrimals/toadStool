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
    /// # use toadstool_runtime_universal::UniversalRuntime;
    /// # async fn example() -> anyhow::Result<()> {
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
    /// # use toadstool_runtime_universal::{UniversalRuntime, WorkloadBuilder, OperationType};
    /// # async fn example() -> anyhow::Result<()> {
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
    /// # use toadstool_runtime_universal::UniversalRuntime;
    /// # async fn example() -> anyhow::Result<()> {
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
        // Note: In a full implementation, we'd serialize/compile the function
        // For now, this is a placeholder showing the API design

        let workload = WorkloadBuilder::new()
            .operation(OperationType::Map)
            .data_f32(input.clone())
            .build()?;

        let output = self.execute_optimal(workload).await?;

        match output.data {
            WorkloadData::F32Vec(v) => Ok(v),
            _ => Err(ComputeError::ExecutionFailed("Type mismatch".to_string())),
        }
    }

    /// Get units by type
    pub fn units_by_type(&self, unit_type: ComputeUnitType) -> Vec<&Box<dyn ComputeUnit>> {
        self.units
            .iter()
            .filter(|u| u.capabilities().unit_type == unit_type)
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
