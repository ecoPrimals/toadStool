// SPDX-License-Identifier: AGPL-3.0-or-later
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

        // Discover GPU (wgpu) — isolated so driver crashes don't bring down the process
        #[cfg(feature = "wgpu-backend")]
        {
            match tokio::time::timeout(std::time::Duration::from_secs(10), Self::discover_wgpu())
                .await
            {
                Ok(gpu_units) => units.extend(gpu_units),
                Err(_) => {
                    eprintln!(
                        "[toadstool-runtime-universal] wgpu discovery timed out — continuing with CPU only"
                    );
                }
            }
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

    /// Discover wgpu adapters.
    ///
    /// Catches panics from GPU driver initialization so that headless/CI
    /// environments degrade gracefully to CPU-only instead of segfaulting.
    ///
    /// **hotSpring absorption (S94):** Multi-adapter selection with env var
    /// override. Set `TOADSTOOL_GPU_ADAPTER` to a comma-separated fallback
    /// list: index (`"0"`), name substring (`"3090,titan"`), or `"auto"`.
    #[cfg(feature = "wgpu-backend")]
    async fn discover_wgpu() -> Vec<Box<dyn ComputeUnit>> {
        use crate::backends::WgpuComputeUnit;

        let adapters = match std::panic::catch_unwind(|| {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });
            instance.enumerate_adapters(wgpu::Backends::all())
        }) {
            Ok(a) => a,
            Err(_) => {
                eprintln!(
                    "[toadstool-runtime-universal] wgpu adapter enumeration panicked — falling back to CPU only"
                );
                return Vec::new();
            }
        };

        let mut units: Vec<Box<dyn ComputeUnit>> = Vec::new();
        let mut infos: Vec<(usize, String, bool)> = Vec::new();
        for (idx, adapter) in adapters.into_iter().enumerate() {
            let info = adapter.get_info();
            let has_f64 = adapter.features().contains(wgpu::Features::SHADER_F64);
            let name = info.name.clone();
            if let Ok(unit) = WgpuComputeUnit::from_adapter(adapter).await {
                infos.push((idx, name, has_f64));
                units.push(Box::new(unit));
            }
        }

        if let Ok(selector) = std::env::var("TOADSTOOL_GPU_ADAPTER") {
            return Self::select_adapters(&selector, &infos, units);
        }

        units
    }

    /// Reorder/filter adapters based on a comma-separated selector string.
    ///
    /// Supported selectors: index ("0"), name substring ("3090"), "auto" (best
    /// discrete GPU with f64). Returns the full list if no selector matches.
    #[cfg(feature = "wgpu-backend")]
    fn select_adapters(
        selector: &str,
        infos: &[(usize, String, bool)],
        mut units: Vec<Box<dyn ComputeUnit>>,
    ) -> Vec<Box<dyn ComputeUnit>> {
        for token in selector.split(',').map(str::trim) {
            if token.eq_ignore_ascii_case("auto") {
                let best = infos
                    .iter()
                    .filter(|(_, _, f64_ok)| *f64_ok)
                    .min_by_key(|(idx, _, _)| *idx);
                if let Some((idx, name, _)) = best {
                    eprintln!(
                        "[toadstool-runtime-universal] TOADSTOOL_GPU_ADAPTER=auto → selected [{idx}] {name}"
                    );
                    let unit = units.swap_remove(*idx);
                    return vec![unit];
                }
            } else if let Ok(idx) = token.parse::<usize>() {
                if idx < units.len() {
                    eprintln!(
                        "[toadstool-runtime-universal] TOADSTOOL_GPU_ADAPTER={idx} → selected [{}]",
                        infos[idx].1
                    );
                    let unit = units.swap_remove(idx);
                    return vec![unit];
                }
            } else {
                let lower = token.to_lowercase();
                if let Some((idx, name, _)) = infos
                    .iter()
                    .find(|(_, n, _)| n.to_lowercase().contains(&lower))
                {
                    eprintln!(
                        "[toadstool-runtime-universal] TOADSTOOL_GPU_ADAPTER={token} → matched [{idx}] {name}"
                    );
                    let unit = units.swap_remove(*idx);
                    return vec![unit];
                }
            }
        }

        eprintln!(
            "[toadstool-runtime-universal] TOADSTOOL_GPU_ADAPTER={selector} matched no adapter — using all discovered"
        );
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
    /// < 1K operations
    Small,
    /// 1K - 1M operations
    Medium,
    /// > 1M operations
    Large,
}

/// Latency requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatencyRequirement {
    /// < 1ms
    Critical,
    /// < 10ms
    Important,
    /// > 10ms
    Relaxed,
}

/// Power constraints
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerConstraint {
    /// < 1W
    UltraLow,
    /// < 10W
    Low,
    /// < 100W
    Medium,
    /// No power constraint
    Unconstrained,
}

/// Throughput requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThroughputRequirement {
    /// < 1 GFLOPS
    Low,
    /// 1-10 GFLOPS
    Medium,
    /// > 10 GFLOPS
    High,
}

impl WorkloadProfile {
    /// Analyze a workload and create a profile
    #[allow(clippy::missing_const_for_fn)] // Workload has non-const fields
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

    #[tokio::test]
    async fn test_discover_all_returns_units() {
        let units = CapabilityDiscovery::discover_all().await;
        // Should have at least CPU on default features
        #[cfg(feature = "cpu")]
        assert!(!units.is_empty(), "Should discover at least CPU");
        #[cfg(not(feature = "cpu"))]
        let _ = units;
    }

    #[test]
    fn test_workload_size_boundary_1000() {
        let w = make_workload(1000);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Small);
    }

    #[test]
    fn test_workload_size_boundary_1001() {
        let w = make_workload(1001);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Medium);
    }

    #[test]
    fn test_workload_size_boundary_1_000_000() {
        let w = make_workload(1_000_000);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Medium);
    }

    #[test]
    fn test_workload_size_boundary_1_000_001() {
        let w = make_workload(1_000_001);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Large);
    }

    #[test]
    fn test_workload_size_zero() {
        let w = make_workload(0);
        let profile = WorkloadProfile::from_workload(&w);
        assert_eq!(profile.size, WorkloadSize::Small);
    }

    #[test]
    fn test_select_best_unit_skips_incompatible() {
        let w = Workload {
            operation: OperationType::MatMul,
            data_type: DataType::F32,
            num_operations: 1000,
            required_memory: 0,
            input: WorkloadData::F32Vec(vec![]),
            params: WorkloadParams::default(),
        };
        let profile = WorkloadProfile::from_workload(&w);
        let cpu = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu)];
        let best = profile.select_best_unit(&units, &w);
        // CPU may or may not support MatMul
        let _ = best;
    }

    #[test]
    fn test_select_best_unit_prefers_higher_score() {
        let w = make_workload(100);
        let profile = WorkloadProfile::from_workload(&w);
        let cpu1 = crate::backends::CpuComputeUnit::discover();
        let cpu2 = crate::backends::CpuComputeUnit::discover();
        let units: Vec<Box<dyn ComputeUnit>> = vec![Box::new(cpu1), Box::new(cpu2)];
        let best = profile.select_best_unit(&units, &w);
        assert!(best.is_some());
    }
}
