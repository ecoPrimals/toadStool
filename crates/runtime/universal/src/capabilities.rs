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

        // Discover GPU (OpenCL)
        #[cfg(feature = "opencl")]
        {
            units.extend(Self::discover_opencl().await);
        }

        // Discover GPU (wgpu)
        #[cfg(feature = "wgpu")]
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
    #[cfg(feature = "opencl")]
    async fn discover_opencl() -> Vec<Box<dyn ComputeUnit>> {
        // TODO: Update OpenCL implementation for new ocl crate API
        // The ocl crate API has changed - Platform::list() now returns Vec directly
        // Device info() methods have also changed significantly
        // Recommended: Use wgpu (pure Rust) as primary path, OpenCL as legacy
        Vec::new()
    }

    /// Discover wgpu adapters
    #[cfg(feature = "wgpu")]
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
    ) -> Option<&'a Box<dyn ComputeUnit>> {
        let mut best_unit: Option<&Box<dyn ComputeUnit>> = None;
        let mut best_score = 0.0;

        for unit in units {
            // Skip if unit can't execute this workload
            if !unit.can_execute(workload) {
                continue;
            }

            let score = unit.capabilities().score_for_workload(workload);

            if score > best_score {
                best_score = score;
                best_unit = Some(unit);
            }
        }

        best_unit
    }
}
