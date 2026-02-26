//! Pure WGSL device - hardware-agnostic compute via WebGPU
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no separate CPU code!)
//! - wgpu handles execution on ANY device (GPU/CPU/NPU/TPU)
//!
//! ## Adapter Selection
//!
//! Set `BARRACUDA_GPU_ADAPTER` environment variable:
//! - `BARRACUDA_GPU_ADAPTER=0` — Select first adapter
//! - `BARRACUDA_GPU_ADAPTER=titan` — Select adapter containing "titan"
//! - `BARRACUDA_GPU_ADAPTER=auto` — Use wgpu HighPerformance (default)

mod buffers;
mod capabilities;
mod creation;

use super::autotune::{GpuCalibration, GLOBAL_TUNER};
use crate::error::Result;
use std::sync::Arc;

/// WebGPU device - executes WGSL on any hardware
#[derive(Debug, Clone)]
pub struct WgpuDevice {
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
    pub(crate) adapter_info: wgpu::AdapterInfo,
    calibration: Option<GpuCalibration>,
    /// Vulkan pipeline cache — avoids re-compiling identical SPIR-V to machine code.
    /// Shared across all pipeline creations on this device.
    pipeline_cache: Option<Arc<wgpu::PipelineCache>>,
}

impl WgpuDevice {
    /// Get device name
    pub fn name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Get device type
    pub fn device_type(&self) -> wgpu::DeviceType {
        self.adapter_info.device_type
    }

    /// Check if running on CPU fallback
    pub fn is_cpu(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }

    /// Check if f64 shaders are enabled for this device.
    ///
    /// Returns `true` when `wgpu::Features::SHADER_F64` was successfully
    /// requested at device creation. F64 shaders panic at validation time
    /// when this is false, so callers should gate any f64 shader dispatch on
    /// this check rather than discovering the missing feature at runtime.
    pub fn has_f64_shaders(&self) -> bool {
        self.device.features().contains(wgpu::Features::SHADER_F64)
    }

    /// Check if the Sovereign Compiler's SPIR-V passthrough path is available.
    ///
    /// Returns `true` when `wgpu::Features::SPIRV_SHADER_PASSTHROUGH` was
    /// granted at device creation — typically available on Vulkan backends
    /// (NVK, RADV, proprietary NVIDIA).
    pub fn has_spirv_passthrough(&self) -> bool {
        self.device
            .features()
            .contains(wgpu::Features::SPIRV_SHADER_PASSTHROUGH)
    }

    /// Compile a pre-built SPIR-V binary into a shader module.
    ///
    /// Requires `SPIRV_SHADER_PASSTHROUGH` — check with `has_spirv_passthrough()`.
    ///
    /// # Safety
    ///
    /// The SPIR-V binary is passed to the driver as-is. The caller must
    /// ensure the binary was produced by a trusted source (our own naga
    /// backend) and has been validated by `naga::valid::Validator`.
    #[allow(unsafe_code)]
    pub fn compile_shader_spirv(
        &self,
        spirv_words: &[u32],
        label: Option<&str>,
    ) -> wgpu::ShaderModule {
        // SAFETY: SPIR-V was emitted by naga::back::spv::Writer from a
        // naga::valid::Validator-approved module. No external/untrusted data.
        unsafe {
            self.device
                .create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
                    label,
                    source: std::borrow::Cow::Borrowed(spirv_words),
                })
        }
    }

    /// Access underlying wgpu device
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get Arc-wrapped device (for shared ownership)
    pub fn device_arc(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    /// Get adapter info (for capability detection)
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Get the device's pipeline cache for `create_compute_pipeline` calls.
    ///
    /// Returns `None` only when the Vulkan driver does not support pipeline caching.
    pub fn pipeline_cache(&self) -> Option<&wgpu::PipelineCache> {
        self.pipeline_cache.as_deref()
    }

    /// Access command queue
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Get Arc to command queue
    pub fn queue_arc(&self) -> Arc<wgpu::Queue> {
        self.queue.clone()
    }

    /// Compile WGSL shader
    pub fn compile_shader(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    /// Compile an f64 WGSL shader with automatic driver-aware patching and ILP optimization.
    ///
    /// Pipeline:
    /// 1. `ShaderTemplate::for_driver_auto` — patches exp/log for drivers that lack native f64
    /// 2. `WgslOptimizer::optimize` — reorders `@ilp_region` blocks + unrolls `@unroll_hint` loops
    ///    (Phase 3 SOVEREIGN_COMPUTE_EVOLUTION; only active when annotations are present)
    /// 3. `SovereignCompiler::compile` — Phase 4: naga IR optimization (FMA fusion, dead expr
    ///    elimination) + SPIR-V emission via `SPIRV_SHADER_PASSTHROUGH` (when available).
    ///
    /// The optimizer is keyed to the actual GPU arch detected at device-creation time,
    /// so the ILP fill width matches the hardware (8 cy on SM70, 4 cy on RDNA2, etc.).
    pub fn compile_shader_f64(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        // Step 1: driver-specific exp/log patching.
        let patched = crate::shaders::precision::ShaderTemplate::for_driver_auto(
            source,
            self.needs_f64_exp_log_workaround(),
        );

        // Step 2: ILP optimizer — fast-path skip when no annotations present.
        let profile = crate::device::driver_profile::GpuDriverProfile::from_device(self);
        let optimized = if patched.contains("@ilp_region") || patched.contains("@unroll_hint") {
            use crate::shaders::optimizer::WgslOptimizer;
            let optimizer = WgslOptimizer::new(profile.latency_model());
            optimizer.optimize(&patched)
        } else {
            patched
        };

        // Step 3: Sovereign compiler — Phase 4 naga IR path.
        // Try SPIR-V passthrough first; fall back to WGSL text if unavailable or on error.
        if self.has_spirv_passthrough() {
            use crate::shaders::sovereign::{SovereignCompiler, SovereignOutput};
            let sovereign = SovereignCompiler::new(profile);
            match sovereign.compile(&optimized) {
                Ok((SovereignOutput::Spirv(words), stats)) => {
                    if stats.fma_fusions > 0 || stats.dead_exprs_eliminated > 0 {
                        tracing::debug!(
                            "sovereign: {} FMA fusions, {} dead exprs eliminated",
                            stats.fma_fusions,
                            stats.dead_exprs_eliminated,
                        );
                    }
                    return self.compile_shader_spirv(&words, label);
                }
                Err(e) => {
                    tracing::debug!("sovereign compiler fallback: {e}");
                }
            }
        }

        self.compile_shader(&optimized, label)
    }

    /// Compile a DF64 (double-float, f32-pair) WGSL shader.
    ///
    /// Prepends `df64_core.wgsl` + `df64_transcendentals.wgsl` to the source,
    /// providing the full DF64 arithmetic library: `Df64`, `df64_add`, `df64_mul`,
    /// `df64_div`, `sqrt_df64`, `exp_df64`, `log_df64`, `sin_df64`, `cos_df64`,
    /// `pow_df64`, `tanh_df64`.
    ///
    /// DF64 shaders run entirely on FP32 cores (no f64 hardware needed), achieving
    /// ~48-bit mantissa (~14 decimal digits) at up to 9.9× the throughput of native
    /// f64 on consumer GPUs (Ampere/Ada fp64:fp32 ≈ 1:64).
    ///
    /// Pipeline mirrors [`compile_shader_f64`] minus the f64 driver patching:
    /// 1. Prepend DF64 preamble (core + transcendentals)
    /// 2. ILP optimizer (when `@ilp_region`/`@unroll_hint` annotations present)
    /// 3. Sovereign compiler SPIR-V path (when available)
    pub fn compile_shader_df64(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        const DF64_CORE: &str = include_str!("../../shaders/math/df64_core.wgsl");
        const DF64_TRANSCENDENTALS: &str =
            include_str!("../../shaders/math/df64_transcendentals.wgsl");

        let combined = format!("{DF64_CORE}\n{DF64_TRANSCENDENTALS}\n{source}");

        let profile = crate::device::driver_profile::GpuDriverProfile::from_device(self);
        let optimized =
            if combined.contains("@ilp_region") || combined.contains("@unroll_hint") {
                use crate::shaders::optimizer::WgslOptimizer;
                let optimizer = WgslOptimizer::new(profile.latency_model());
                optimizer.optimize(&combined)
            } else {
                combined
            };

        if self.has_spirv_passthrough() {
            use crate::shaders::sovereign::{SovereignCompiler, SovereignOutput};
            let sovereign = SovereignCompiler::new(profile);
            match sovereign.compile(&optimized) {
                Ok((SovereignOutput::Spirv(words), stats)) => {
                    if stats.fma_fusions > 0 || stats.dead_exprs_eliminated > 0 {
                        tracing::debug!(
                            "sovereign df64: {} FMA fusions, {} dead exprs eliminated",
                            stats.fma_fusions,
                            stats.dead_exprs_eliminated,
                        );
                    }
                    return self.compile_shader_spirv(&words, label);
                }
                Err(e) => {
                    tracing::debug!("sovereign df64 fallback: {e}");
                }
            }
        }

        self.compile_shader(&optimized, label)
    }

    /// Compile a shader written as universal math, specialized to the requested precision.
    ///
    /// **Math is universal, precision is silicon.** The same algorithm written once
    /// in f64 (the conceptually true math) is compiled for any target precision:
    ///
    /// - `Precision::F32` — downcast f64 types to f32, compile via standard path
    /// - `Precision::F64` — full `compile_shader_f64()` pipeline (polyfills + sovereign compiler)
    /// - `Precision::Df64` — downcast f64 to DF64 types + transcendentals, compile via
    ///   `compile_shader_df64()` which auto-injects the DF64 core library
    /// - `Precision::F16` — downcast f64 types to f16, compile via standard path
    ///
    /// Pass the f64-canonical source (the "true math") for ALL precisions.
    /// The pipeline handles the rest.
    ///
    /// **DF64 coverage**: Full coverage via two complementary layers:
    ///
    /// 1. **Text-based downcast** — handles types, constructors, transcendentals,
    ///    storage conversions (fast, always available)
    /// 2. **Naga-guided rewrite** — parses with naga for type analysis, rewrites
    ///    f64 infix operators (`+`, `-`, `*`, `/`) to df64 function calls.
    ///    Falls back to text-only downcast if naga rewrite fails.
    ///
    /// Shaders using `op_add`/`op_mul`/etc. work at all precisions without
    /// either layer — the operation preamble provides implementations directly.
    pub fn compile_shader_universal(
        &self,
        source: &str,
        precision: crate::shaders::precision::Precision,
        label: Option<&str>,
    ) -> wgpu::ShaderModule {
        use crate::shaders::precision::{downcast_f64_to_df64, downcast_f64_to_f32, Precision};
        match precision {
            Precision::F32 => {
                let f32_source = downcast_f64_to_f32(source);
                self.compile_shader(&f32_source, label)
            }
            Precision::F64 => self.compile_shader_f64(source, label),
            Precision::Df64 => {
                // Two-layer DF64 compilation:
                //
                // Layer 1 (naga-guided): Parse f64 WGSL with naga, identify f64
                //   infix operators by type, replace with bridge functions that
                //   route computation through DF64 while keeping f64 types.
                //
                // Layer 2 (text-based): downcast_f64_to_df64 handles types,
                //   constructors, transcendentals, and storage conversions.
                //
                // Naga is tried first. If it fails (e.g., source uses polyfill
                // functions naga can't validate), fall back to text-only downcast.
                let df64_source = crate::shaders::sovereign::df64_rewrite::rewrite_f64_infix_full(source)
                    .unwrap_or_else(|_| downcast_f64_to_df64(source));
                self.compile_shader_df64(&df64_source, label)
            }
            Precision::F16 => {
                let f16_source = source
                    .replace("array<f64>", "array<f16>")
                    .replace("array<f64,", "array<f16,")
                    .replace(": f64", ": f16")
                    .replace("-> f64", "-> f16")
                    .replace("f64(", "f16(")
                    .replace("<f64>", "<f16>");
                self.compile_shader(&f16_source, label)
            }
        }
    }

    /// Compile a universal shader that uses `op_add`/`op_mul`/etc. operations.
    ///
    /// This is the ultimate "math is universal" entry point. The shader uses
    /// abstract operation functions (`op_add`, `op_mul`, `op_pack`, `op_unpack`,
    /// etc.) and `Scalar` as the type alias. The pipeline:
    ///
    /// 1. Injects the precision-specific operation preamble (trivial wrappers
    ///    for f32/f64, DF64 library calls for Df64)
    /// 2. Routes through the appropriate compilation pipeline
    ///
    /// Shaders written this way work at ALL precisions without naga IR rewriting.
    pub fn compile_op_shader(
        &self,
        source: &str,
        precision: crate::shaders::precision::Precision,
        label: Option<&str>,
    ) -> wgpu::ShaderModule {
        use crate::shaders::precision::Precision;
        let preamble = precision.op_preamble();
        let combined = format!("{preamble}\n{source}");
        match precision {
            Precision::F64 => self.compile_shader_f64(&combined, label),
            Precision::Df64 => self.compile_shader_df64(&combined, label),
            _ => self.compile_shader(&combined, label),
        }
    }

    /// Compile a `{{SCALAR}}`-templated shader at the given precision.
    ///
    /// Renders the template via [`ShaderTemplate::render`], then routes through
    /// the appropriate compilation pipeline for the target precision.
    pub fn compile_template(
        &self,
        template: &crate::shaders::precision::ShaderTemplate,
        precision: crate::shaders::precision::Precision,
        label: Option<&str>,
    ) -> wgpu::ShaderModule {
        use crate::shaders::precision::Precision;
        let rendered = template.render(precision);
        match precision {
            Precision::F64 => self.compile_shader_f64(&rendered, label),
            Precision::Df64 => self.compile_shader_df64(&rendered, label),
            _ => self.compile_shader(&rendered, label),
        }
    }

    /// Execute WGSL compute shader
    pub fn execute_compute(
        &self,
        shader_source: &str,
        bind_groups: &[&wgpu::BindGroup],
        workgroups: (u32, u32, u32),
    ) -> Result<()> {
        let shader = self.compile_shader(shader_source, Some("barraCuda Operation"));
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("barraCuda Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
                cache: self.pipeline_cache(),
                compilation_options: Default::default(),
            });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("barraCuda Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("barraCuda Compute"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            for (i, bg) in bind_groups.iter().enumerate() {
                pass.set_bind_group(i as u32, bg, &[]);
            }
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }

        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }

    /// Get calibration for this device
    pub fn get_calibration(&self) -> GpuCalibration {
        GLOBAL_TUNER.get_or_calibrate(&self.device, &self.queue, &self.adapter_info.name)
    }

    /// Force recalibration
    pub fn recalibrate(&self) -> GpuCalibration {
        GLOBAL_TUNER.recalibrate(&self.device, &self.queue, &self.adapter_info.name)
    }

    /// Get optimal workgroup size for this device
    pub fn optimal_workgroup_size(&self) -> u32 {
        self.calibration
            .as_ref()
            .map(|c| c.optimal_workgroup_size)
            .unwrap_or_else(|| {
                GLOBAL_TUNER
                    .get_or_calibrate(&self.device, &self.queue, &self.adapter_info.name)
                    .optimal_workgroup_size
            })
    }

    /// Get measured peak bandwidth for this device (GB/s)
    pub fn peak_bandwidth_gbps(&self) -> f64 {
        self.get_calibration().peak_bandwidth_gbps
    }

    /// Get measured dispatch overhead for this device (μs)
    pub fn dispatch_overhead_us(&self) -> f64 {
        self.get_calibration().dispatch_overhead_us
    }

    /// Create calibrated device (runs calibration immediately)
    pub async fn new_calibrated() -> Result<Self> {
        let mut device = Self::new().await?;
        let cal = device.get_calibration();
        device.calibration = Some(cal);
        Ok(device)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wgpu_device_creation() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };
        println!("barraCuda device: {}", device.name());
        if device.is_cpu() {
            println!("  Using CPU software rasterizer");
        } else {
            println!("  Using GPU acceleration");
        }
    }

    #[tokio::test]
    async fn test_enumerate_adapters() {
        let adapters = WgpuDevice::enumerate_adapters();
        assert!(
            !adapters.is_empty(),
            "WGPU should find at least one adapter"
        );
    }

    #[tokio::test]
    async fn test_buffer_operations() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };
        let buffer = device.create_buffer_f32(10).unwrap();
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        device.write_buffer_f32(&buffer, &data).unwrap();
        let read_data = device.read_buffer_f32(&buffer, 10).unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_from_selection_gpu() {
        use super::super::toadstool_integration::DeviceSelection;
        if let Ok(device) = WgpuDevice::from_selection(DeviceSelection::Gpu).await {
            assert!(!device.is_cpu());
        }
    }

    #[tokio::test]
    async fn test_from_selection_cpu() {
        use super::super::toadstool_integration::DeviceSelection;
        if let Ok(device) = WgpuDevice::from_selection(DeviceSelection::Cpu).await {
            assert!(device.is_cpu());
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_auto() {
        let _ = WgpuDevice::with_adapter_selector("auto").await;
    }

    #[tokio::test]
    async fn test_adapter_selector_index() {
        let adapters = WgpuDevice::enumerate_adapters();
        if adapters.is_empty() {
            return;
        }
        if let Ok(device) = WgpuDevice::with_adapter_selector("0").await {
            assert_eq!(device.name(), adapters[0].name);
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_name_match() {
        let adapters = WgpuDevice::enumerate_adapters();
        if adapters.is_empty() {
            return;
        }
        let partial = adapters[0]
            .name
            .chars()
            .take(4)
            .collect::<String>()
            .to_lowercase();
        let _ = WgpuDevice::with_adapter_selector(&partial).await;
    }

    #[tokio::test]
    async fn test_adapter_selector_fallthrough() {
        let adapters = WgpuDevice::enumerate_adapters();
        let large_index = (adapters.len() + 1000).to_string();
        if let Err(e) = WgpuDevice::with_adapter_selector(&large_index).await {
            assert!(e.to_string().contains("No adapter matches"));
        }
    }

    #[tokio::test]
    async fn test_from_env_default() {
        std::env::remove_var(super::creation::ADAPTER_ENV_VAR);
        let _ = WgpuDevice::from_env().await;
    }

    #[tokio::test]
    async fn test_driver_detection_apis() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };
        let _ = device.is_nvk();
        let _ = device.is_radv();
        let _ = device.is_nvidia_proprietary();
    }

    #[test]
    fn test_driver_detection_logic() {
        fn contains_nvk_markers(driver: &str) -> bool {
            let lower = driver.to_lowercase();
            lower.contains("nvk") || lower.contains("nouveau") || lower.contains("mesa")
        }
        fn contains_radv_markers(driver: &str) -> bool {
            driver.to_lowercase().contains("radv")
        }
        assert!(contains_nvk_markers("NVK"));
        assert!(contains_nvk_markers("nouveau"));
        assert!(!contains_nvk_markers("NVIDIA"));
        assert!(contains_radv_markers("RADV"));
        assert!(!contains_radv_markers("NVIDIA"));
    }
}
