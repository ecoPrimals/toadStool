// SPDX-License-Identifier: AGPL-3.0-only

//! GPU silicon unit discovery and performance surface types.
//!
//! A modern GPU die has 8+ distinct hardware units, each a special-purpose
//! computer. This module provides the type system for discovering, profiling,
//! and routing work to every functional unit — not just shader cores.
//!
//! See `specs/ALL_SILICON_PIPELINE.md` for the full specification.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A functional unit on a GPU die that can execute science work.
///
/// Each variant represents a distinct piece of silicon with its own
/// throughput characteristics. The rasterizer is a spatial query engine.
/// The depth buffer is a min-reducer. The ROPs are scatter-adders.
/// Every unit is a hidden computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiliconUnit {
    /// Programmable shader cores — FP arithmetic (add, mul, fma).
    /// Already used for compute shaders, DF64 emulation.
    ShaderCore,
    /// Matrix multiply-accumulate units (Volta+: HMMA/IMMA).
    /// Science: CG solver, pairwise distances, convolution, FFT butterfly.
    TensorCore,
    /// BVH traversal + ray-triangle intersection (Turing+).
    /// Science: MD neighbor search, Monte Carlo transport, line-of-sight.
    RtCore,
    /// Texture mapping units — 2D interpolated lookup in one cycle.
    /// Science: EOS tables, activation functions, exp/log at reduced precision.
    TextureUnit,
    /// Render output units / alpha blending — per-pixel scatter-add / min / max.
    /// Science: histograms, particle deposition, Beer-Lambert transmittance.
    Rop,
    /// Fixed-function rasterizer — point-in-polygon + barycentric interpolation.
    /// Science: voxelization, FEM cell assignment, spatial binning (PIC, SPH).
    Rasterizer,
    /// Z-buffer hardware — per-pixel minimum reduction at fill rate.
    /// Science: Voronoi diagrams, distance fields, nearest-neighbor queries.
    DepthBuffer,
    /// Fixed-function tessellation engine — adaptive mesh subdivision.
    /// Science: AMR, FEM mesh refinement, terrain LOD.
    Tessellator,
    /// Hardware video encode/decode (NVENC/VCN/QSV).
    /// Science: simulation frame compression, motion-estimation registration.
    VideoEncoder,
}

impl SiliconUnit {
    /// All silicon unit variants in canonical order.
    pub const ALL: [Self; 9] = [
        Self::ShaderCore,
        Self::TensorCore,
        Self::RtCore,
        Self::TextureUnit,
        Self::Rop,
        Self::Rasterizer,
        Self::DepthBuffer,
        Self::Tessellator,
        Self::VideoEncoder,
    ];

    /// Whether this unit requires the sovereign VFIO pipeline for science use.
    ///
    /// Shader cores are accessible via wgpu compute shaders. All other units
    /// require either the graphics pipeline (rasterizer, depth, ROPs, TMUs,
    /// tessellator) or special dispatch paths (tensor, RT, video encoder).
    #[must_use]
    pub const fn requires_sovereign_pipeline(&self) -> bool {
        !matches!(self, Self::ShaderCore)
    }

    /// Whether this unit is available through the standard wgpu compute path.
    #[must_use]
    pub const fn available_via_wgpu_compute(&self) -> bool {
        matches!(self, Self::ShaderCore)
    }

    /// Whether this unit is part of the graphics pipeline.
    #[must_use]
    pub const fn is_graphics_pipeline_unit(&self) -> bool {
        matches!(
            self,
            Self::Rasterizer
                | Self::DepthBuffer
                | Self::Rop
                | Self::TextureUnit
                | Self::Tessellator
        )
    }

    /// Wire-format name for JSON-RPC and performance surface keys.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ShaderCore => "shader_core",
            Self::TensorCore => "tensor_core",
            Self::RtCore => "rt_core",
            Self::TextureUnit => "texture_unit",
            Self::Rop => "rop",
            Self::Rasterizer => "rasterizer",
            Self::DepthBuffer => "depth_buffer",
            Self::Tessellator => "tessellator",
            Self::VideoEncoder => "video_encoder",
        }
    }
}

impl fmt::Display for SiliconUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Tensor core generation — determines MMA precision modes available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TensorCoreGen {
    /// SM 7.0 (Volta) — FP16 MMA only.
    Volta,
    /// SM 7.5 (Turing) — FP16, INT8, INT4.
    Turing,
    /// SM 8.0+ (Ampere) — FP16, BF16, TF32, FP64, INT8.
    Ampere,
    /// SM 8.9 (Ada Lovelace) — FP8 added.
    Ada,
    /// SM 9.0 (Hopper) — FP8, transformer engine.
    Hopper,
}

/// RT core generation — determines BVH/intersection capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RtCoreGen {
    /// 1st gen (Turing) — ray-triangle intersection.
    Turing,
    /// 2nd gen (Ampere) — concurrent RT/shader execution.
    Ampere,
    /// 3rd gen (Ada) — opacity micro-maps, displaced micro-meshes.
    Ada,
}

/// Per-GPU silicon capabilities discovered at runtime.
///
/// Attached to `GpuAdapterInfo` to expose what functional units
/// are available on this specific GPU die.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiliconCapabilities {
    /// Whether tensor cores are present and their generation.
    pub tensor_cores: Option<TensorCoreGen>,
    /// Whether RT cores are present and their generation.
    pub rt_cores: Option<RtCoreGen>,
    /// Whether a hardware video encoder is present (NVENC, VCN, QSV).
    pub has_video_encoder: bool,
    /// Estimated number of texture mapping units.
    pub estimated_tmu_count: u32,
    /// Estimated number of render output units.
    pub estimated_rop_count: u32,
    /// Rasterizer is available (true for all discrete GPUs with display output).
    pub rasterizer_available: bool,
    /// Tessellation engine is available.
    pub tessellator_available: bool,
    /// Which silicon units are confirmed available on this GPU.
    pub available_units: Vec<SiliconUnit>,
}

impl SiliconCapabilities {
    /// Build capabilities with shader cores only (minimum viable GPU).
    #[must_use]
    pub fn shader_only() -> Self {
        Self {
            tensor_cores: None,
            rt_cores: None,
            has_video_encoder: false,
            estimated_tmu_count: 0,
            estimated_rop_count: 0,
            rasterizer_available: false,
            tessellator_available: false,
            available_units: vec![SiliconUnit::ShaderCore],
        }
    }

    /// Build capabilities for a typical discrete GPU (shader + graphics pipeline).
    #[must_use]
    pub fn discrete_gpu_baseline(tmu_count: u32, rop_count: u32) -> Self {
        Self {
            tensor_cores: None,
            rt_cores: None,
            has_video_encoder: false,
            estimated_tmu_count: tmu_count,
            estimated_rop_count: rop_count,
            rasterizer_available: true,
            tessellator_available: true,
            available_units: vec![
                SiliconUnit::ShaderCore,
                SiliconUnit::TextureUnit,
                SiliconUnit::Rop,
                SiliconUnit::Rasterizer,
                SiliconUnit::DepthBuffer,
                SiliconUnit::Tessellator,
            ],
        }
    }

    /// Whether a specific silicon unit is available on this GPU.
    #[must_use]
    pub fn has_unit(&self, unit: SiliconUnit) -> bool {
        self.available_units.contains(&unit)
    }

    /// Count of available silicon units.
    #[must_use]
    pub fn unit_count(&self) -> usize {
        self.available_units.len()
    }
}

impl Default for SiliconCapabilities {
    fn default() -> Self {
        Self::shader_only()
    }
}

/// A single performance measurement from a spring hardware experiment.
///
/// Springs report these via `compute.performance_surface.report` after
/// validating a hardware unit for a specific operation class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMeasurement {
    /// Operation identifier (e.g. `"math.pairwise.yukawa"`).
    pub operation: String,
    /// Which silicon unit was measured.
    pub silicon_unit: SiliconUnit,
    /// Precision mode used (e.g. `"fp16"`, `"fp32"`, `"tf32"`, `"df64"`).
    pub precision_mode: String,
    /// Measured throughput in GFLOPS.
    pub throughput_gflops: f64,
    /// Tolerance achieved (e.g. `1e-7`).
    pub tolerance_achieved: f64,
    /// GPU model identifier (e.g. `"RTX 3090"`).
    pub gpu_model: String,
    /// Which spring/experiment produced this measurement.
    pub measured_by: String,
    /// Measurement timestamp (epoch seconds).
    pub timestamp: u64,
}

/// A routing recommendation from the performance surface.
///
/// Given an operation and tolerance requirement, this is what toadStool
/// recommends based on measured data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSurfaceEntry {
    /// Operation identifier.
    pub operation: String,
    /// Tolerance requirement that this entry satisfies.
    pub tolerance_required: f64,
    /// Recommended silicon unit for best throughput at this tolerance.
    pub recommended_unit: SiliconUnit,
    /// Recommended precision mode.
    pub recommended_precision: String,
    /// Estimated throughput in GFLOPS on the recommended unit.
    pub estimated_throughput_gflops: f64,
    /// Fallback silicon unit if recommended is unavailable.
    pub fallback_unit: SiliconUnit,
    /// Estimated throughput on the fallback unit.
    pub fallback_throughput_gflops: f64,
}

/// A routed operation within a multi-unit dispatch plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedOperation {
    /// Operation identifier.
    pub operation: String,
    /// Silicon unit selected for this operation.
    pub silicon_unit: SiliconUnit,
    /// Precision mode selected.
    pub precision_mode: String,
    /// Estimated throughput in GFLOPS.
    pub estimated_throughput_gflops: f64,
    /// Human-readable reason for this routing decision.
    pub reason: String,
    /// Fallback if the selected unit is unavailable.
    pub fallback: Option<Box<RoutedOperation>>,
}

/// A complete multi-unit routing plan for a compound workload.
///
/// Phase C: toadStool splits a workload across multiple silicon units
/// on the same GPU, routing each sub-operation to the cheapest unit
/// that meets its tolerance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiUnitRoutingPlan {
    /// Individual routed operations.
    pub operations: Vec<RoutedOperation>,
    /// Combined estimated throughput across all units.
    pub total_estimated_throughput_gflops: f64,
    /// Target GPU for this plan.
    pub gpu_target: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silicon_unit_all_variants() {
        assert_eq!(SiliconUnit::ALL.len(), 9);
        for unit in &SiliconUnit::ALL {
            assert!(!unit.as_str().is_empty());
        }
    }

    #[test]
    fn shader_core_does_not_require_sovereign() {
        assert!(!SiliconUnit::ShaderCore.requires_sovereign_pipeline());
        assert!(SiliconUnit::ShaderCore.available_via_wgpu_compute());
    }

    #[test]
    fn fixed_function_units_require_sovereign() {
        for unit in &SiliconUnit::ALL {
            if *unit != SiliconUnit::ShaderCore {
                assert!(
                    unit.requires_sovereign_pipeline(),
                    "{unit} should require sovereign pipeline"
                );
            }
        }
    }

    #[test]
    fn graphics_pipeline_units() {
        assert!(SiliconUnit::Rasterizer.is_graphics_pipeline_unit());
        assert!(SiliconUnit::DepthBuffer.is_graphics_pipeline_unit());
        assert!(SiliconUnit::Rop.is_graphics_pipeline_unit());
        assert!(SiliconUnit::TextureUnit.is_graphics_pipeline_unit());
        assert!(SiliconUnit::Tessellator.is_graphics_pipeline_unit());
        assert!(!SiliconUnit::ShaderCore.is_graphics_pipeline_unit());
        assert!(!SiliconUnit::TensorCore.is_graphics_pipeline_unit());
        assert!(!SiliconUnit::RtCore.is_graphics_pipeline_unit());
        assert!(!SiliconUnit::VideoEncoder.is_graphics_pipeline_unit());
    }

    #[test]
    fn silicon_capabilities_shader_only() {
        let caps = SiliconCapabilities::shader_only();
        assert_eq!(caps.unit_count(), 1);
        assert!(caps.has_unit(SiliconUnit::ShaderCore));
        assert!(!caps.has_unit(SiliconUnit::TensorCore));
    }

    #[test]
    fn silicon_capabilities_discrete_gpu() {
        let caps = SiliconCapabilities::discrete_gpu_baseline(128, 96);
        assert_eq!(caps.unit_count(), 6);
        assert!(caps.has_unit(SiliconUnit::ShaderCore));
        assert!(caps.has_unit(SiliconUnit::Rasterizer));
        assert!(caps.has_unit(SiliconUnit::DepthBuffer));
        assert!(caps.has_unit(SiliconUnit::Rop));
        assert!(caps.has_unit(SiliconUnit::TextureUnit));
        assert!(caps.has_unit(SiliconUnit::Tessellator));
        assert!(!caps.has_unit(SiliconUnit::TensorCore));
        assert_eq!(caps.estimated_tmu_count, 128);
        assert_eq!(caps.estimated_rop_count, 96);
    }

    #[test]
    fn silicon_unit_display() {
        assert_eq!(SiliconUnit::ShaderCore.to_string(), "shader_core");
        assert_eq!(SiliconUnit::TensorCore.to_string(), "tensor_core");
        assert_eq!(SiliconUnit::RtCore.to_string(), "rt_core");
        assert_eq!(SiliconUnit::Rop.to_string(), "rop");
    }

    #[test]
    fn silicon_unit_serde_roundtrip() {
        for unit in &SiliconUnit::ALL {
            let json = serde_json::to_string(unit).expect("serialize");
            let back: SiliconUnit = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*unit, back);
        }
    }

    #[test]
    fn performance_measurement_serde() {
        let m = PerformanceMeasurement {
            operation: String::from("math.pairwise.yukawa"),
            silicon_unit: SiliconUnit::RtCore,
            precision_mode: String::from("fp32"),
            throughput_gflops: 5400.0,
            tolerance_achieved: 1e-7,
            gpu_model: String::from("RTX 3090"),
            measured_by: String::from("hotSpring exp076"),
            timestamp: 1_710_000_000,
        };
        let json = serde_json::to_string(&m).expect("serialize");
        let back: PerformanceMeasurement = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(&*back.operation, "math.pairwise.yukawa");
        assert_eq!(back.silicon_unit, SiliconUnit::RtCore);
    }

    #[test]
    fn routed_operation_with_fallback() {
        let fallback = RoutedOperation {
            operation: String::from("neighbor_search"),
            silicon_unit: SiliconUnit::ShaderCore,
            precision_mode: String::from("fp32"),
            estimated_throughput_gflops: 540.0,
            reason: String::from("compute BVH fallback"),
            fallback: None,
        };
        let primary = RoutedOperation {
            operation: String::from("neighbor_search"),
            silicon_unit: SiliconUnit::RtCore,
            precision_mode: String::from("fp32"),
            estimated_throughput_gflops: 5400.0,
            reason: String::from("spatial query, 10x over compute"),
            fallback: Some(Box::new(fallback)),
        };
        assert!(primary.fallback.is_some());
        let fb = primary.fallback.as_ref().unwrap();
        assert_eq!(fb.silicon_unit, SiliconUnit::ShaderCore);
    }

    #[test]
    fn multi_unit_routing_plan() {
        let plan = MultiUnitRoutingPlan {
            operations: vec![
                RoutedOperation {
                    operation: String::from("neighbor_search"),
                    silicon_unit: SiliconUnit::RtCore,
                    precision_mode: String::from("fp32"),
                    estimated_throughput_gflops: 5400.0,
                    reason: String::from("spatial query"),
                    fallback: None,
                },
                RoutedOperation {
                    operation: String::from("force_eval"),
                    silicon_unit: SiliconUnit::ShaderCore,
                    precision_mode: String::from("df64"),
                    estimated_throughput_gflops: 3240.0,
                    reason: String::from("14-digit tolerance"),
                    fallback: None,
                },
                RoutedOperation {
                    operation: String::from("accumulation"),
                    silicon_unit: SiliconUnit::Rop,
                    precision_mode: String::from("fp32"),
                    estimated_throughput_gflops: 2700.0,
                    reason: String::from("additive scatter"),
                    fallback: None,
                },
            ],
            total_estimated_throughput_gflops: 11_340.0,
            gpu_target: String::from("RTX 3090"),
        };
        assert_eq!(plan.operations.len(), 3);
        assert_eq!(&*plan.gpu_target, "RTX 3090");
    }

    #[test]
    fn tensor_core_gen_serde() {
        let tc_gen = TensorCoreGen::Ampere;
        let json = serde_json::to_string(&tc_gen).expect("serialize");
        let back: TensorCoreGen = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(tc_gen, back);
    }

    #[test]
    fn rt_core_gen_serde() {
        let rt_gen = RtCoreGen::Ada;
        let json = serde_json::to_string(&rt_gen).expect("serialize");
        let back: RtCoreGen = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(rt_gen, back);
    }
}
