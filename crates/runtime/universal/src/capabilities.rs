//! Capability discovery and management
//!
//! This module implements runtime discovery of compute capabilities,
//! following the principle: "Discover, don't hardcode"

use crate::types::*;

/// Capability discovery engine
pub struct CapabilityDiscovery;

impl CapabilityDiscovery {
    /// Discover all available compute units
    ///
    /// This function discovers compute resources at runtime:
    /// - CPU cores and capabilities
    /// - GPU devices (OpenCL, wgpu, etc.)
    /// - Neuromorphic processors (future)
    ///
    /// No hardcoded assumptions - everything is discovered!
    pub async fn discover_all() -> Vec<Box<dyn ComputeUnit>> {
        let mut units: Vec<Box<dyn ComputeUnit>> = Vec::new();

        // Discover CPU
        if let Some(cpu) = Self::discover_cpu() {
            units.push(cpu);
        }

        // Discover GPU (OpenCL) - DEPRECATED, use wgpu instead
        // OpenCL support is legacy - kept for compatibility but returns empty Vec
        #[cfg(feature = "opencl")]
        {
            #[allow(deprecated)]
            units.extend(Self::discover_opencl().await);
        }

        // Discover GPU (wgpu)
        #[cfg(feature = "wgpu-backend")]
        {
            units.extend(Self::discover_wgpu().await);
        }

        // Future: Discover neuromorphic
        // units.extend(Self::discover_neuromorphic().await);

        units
    }

    /// Discover CPU capabilities
    fn discover_cpu() -> Option<Box<dyn ComputeUnit>> {
        #[cfg(feature = "cpu")]
        {
            Some(Box::new(crate::backends::CpuComputeUnit::discover()))
        }

        #[cfg(not(feature = "cpu"))]
        {
            None
        }
    }

    /// Discover OpenCL devices
    ///
    /// **DEPRECATED**: OpenCL support is legacy. Use wgpu (barraCuda) instead.
    ///
    /// **Why Deprecated**:
    /// - OpenCL requires C bindings (FFI complexity)
    /// - ocl crate API has breaking changes
    /// - wgpu provides pure Rust alternative
    /// - wgpu is vendor-agnostic (NVIDIA, AMD, Intel, Apple)
    /// - barraCuda framework built on wgpu
    ///
    /// **Migration**: Use `discover_wgpu()` for GPU compute
    #[cfg(feature = "opencl")]
    #[deprecated(
        since = "3.0.0",
        note = "Use wgpu (barraCuda) instead - pure Rust, vendor-agnostic"
    )]
    async fn discover_opencl() -> Vec<Box<dyn ComputeUnit>> {
        // Returning empty Vec - OpenCL discovery deprecated in favor of wgpu
        // Applications should use discover_wgpu() for GPU compute
        Vec::new()
    }

    /// Discover wgpu adapters
    #[cfg(feature = "wgpu-backend")]
    async fn discover_wgpu() -> Vec<Box<dyn ComputeUnit>> {
        use crate::backends::WgpuComputeUnit;

        let mut units = Vec::new();

        // Create wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Enumerate all adapters
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());

        for adapter in adapters {
            if let Ok(unit) = WgpuComputeUnit::from_adapter(adapter).await {
                units.push(Box::new(unit) as Box<dyn ComputeUnit>);
            }
        }

        units
    }
}

/// Workload profile for analysis
pub struct WorkloadProfile {
    /// Size category
    pub size: WorkloadSize,

    /// Latency requirement
    pub latency: LatencyRequirement,

    /// Power constraint
    pub power: PowerConstraint,

    /// Throughput requirement
    pub throughput: ThroughputRequirement,
}

/// Workload size categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadSize {
    Small,  // < 1K operations
    Medium, // 1K - 1M operations
    Large,  // > 1M operations
}

/// Latency requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyRequirement {
    Critical,  // < 1ms
    Important, // < 10ms
    Relaxed,   // > 10ms
}

/// Power constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerConstraint {
    UltraLow, // < 1W
    Low,      // < 10W
    Medium,   // < 100W
    Unconstrained,
}

/// Throughput requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThroughputRequirement {
    Low,    // < 1 GFLOPS
    Medium, // 1-10 GFLOPS
    High,   // > 10 GFLOPS
}

impl WorkloadProfile {
    /// Analyze a workload and create a profile
    pub fn from_workload(workload: &Workload) -> Self {
        let size = match workload.num_operations {
            0..=1_000 => WorkloadSize::Small,
            1_001..=1_000_000 => WorkloadSize::Medium,
            _ => WorkloadSize::Large,
        };

        // Default profiles (can be extended with workload hints)
        Self {
            size,
            latency: LatencyRequirement::Relaxed,
            power: PowerConstraint::Unconstrained,
            throughput: ThroughputRequirement::Medium,
        }
    }

    /// Select best compute unit for this profile
    pub fn select_best_unit<'a>(
        &self,
        units: &'a [Box<dyn ComputeUnit>],
        workload: &Workload,
    ) -> Option<&'a dyn ComputeUnit> {
        let mut best_unit: Option<&dyn ComputeUnit> = None;
        let mut best_score = 0.0;

        for unit in units {
            // Skip if unit can't execute this workload
            if !unit.can_execute(workload) {
                continue;
            }

            let score = unit.capabilities().score_for_workload(workload);

            if score > best_score {
                best_score = score;
                best_unit = Some(unit.as_ref());
            }
        }

        best_unit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_workload(n: usize) -> Workload {
        Workload {
            operation: OperationType::Map,
            data_type: DataType::F32,
            num_operations: n,
            required_memory: 0,
            input: WorkloadData::F32Vec(vec![]),
            params: WorkloadParams::default(),
        }
    }

    #[test]
    fn test_workload_size_small() {
        let w = make_workload(500);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Small);
    }

    #[test]
    fn test_workload_size_medium() {
        let w = make_workload(50_000);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Medium);
    }

    #[test]
    fn test_workload_size_large() {
        let w = make_workload(2_000_000);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Large);
    }

    #[test]
    fn test_select_best_unit_empty_returns_none() {
        let w = make_workload(10);
        let profile = WorkloadProfile::from_workload(&w);
        let units: Vec<Box<dyn ComputeUnit>> = vec![];
        assert!(profile.select_best_unit(&units, &w).is_none());
    }

    #[test]
    fn test_select_best_unit_with_cpu() {
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let w = make_workload(100);
        let profile = WorkloadProfile::from_workload(&w);
        let best = profile.select_best_unit(&units, &w);
        // CPU supports Map/F32, so we should get a result
        assert!(best.is_some());
    }

    #[test]
    fn test_latency_requirement_variants() {
        let _ = LatencyRequirement::Critical;
        let _ = LatencyRequirement::Important;
        let _ = LatencyRequirement::Relaxed;
    }

    #[test]
    fn test_power_constraint_variants() {
        let _ = PowerConstraint::UltraLow;
        let _ = PowerConstraint::Low;
        let _ = PowerConstraint::Medium;
        let _ = PowerConstraint::Unconstrained;
    }

    #[test]
    fn test_throughput_requirement_variants() {
        let _ = ThroughputRequirement::Low;
        let _ = ThroughputRequirement::Medium;
        let _ = ThroughputRequirement::High;
    }
}
