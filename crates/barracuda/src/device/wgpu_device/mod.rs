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
                        log::debug!(
                            "sovereign: {} FMA fusions, {} dead exprs eliminated",
                            stats.fma_fusions,
                            stats.dead_exprs_eliminated,
                        );
                    }
                    return self.compile_shader_spirv(&words, label);
                }
                Err(e) => {
                    log::debug!("sovereign compiler fallback: {e}");
                }
            }
        }

        self.compile_shader(&optimized, label)
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
