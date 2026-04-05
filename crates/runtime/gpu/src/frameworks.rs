// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU Framework Implementations

use super::traits::ParallelComputeFramework;
use super::types::{
    CompiledKernel, DataType, DeviceCapabilities, DeviceId, DeviceInfo, DeviceType, DeviceUsage,
    FrameworkHandle, GpuFramework, KernelFormat, KernelInput, KernelOutput,
    PerformanceCharacteristics, ResourceAllocation, UniversalComputeDevice,
};
use async_trait::async_trait;
use std::collections::HashMap;
#[cfg(feature = "webgpu")]
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
#[cfg(feature = "webgpu")]
use tokio::sync::RwLock;
use uuid::Uuid;

/// `WebGPU` adapter wrapper for conditional compilation.
pub struct WebGPUAdapter {
    #[cfg(feature = "webgpu")]
    /// wgpu instance.
    pub instance: wgpu::Instance,
    #[cfg(feature = "webgpu")]
    /// wgpu adapter.
    pub adapter: wgpu::Adapter,
    #[cfg(not(feature = "webgpu"))]
    _private: (),
}

/// `WebGPU` framework implementation.
pub struct WebGpuFramework;

impl WebGpuFramework {
    /// Creates a new `WebGPU` framework (initialization deferred to first use).
    ///
    /// # Errors
    ///
    /// Currently always returns `Ok`; reserved for future validation.
    pub const fn new() -> ToadStoolResult<Self> {
        // Initialize WebGPU adapter with proper error handling
        // Note: WebGPU initialization is async, so we defer to first usage
        Ok(Self)
    }

    /// Initialize `WebGPU` instance and adapter
    ///
    /// Note: Only used when `webgpu` feature is enabled
    async fn initialize_webgpu(&self) -> ToadStoolResult<WebGPUAdapter> {
        #[cfg(feature = "webgpu")]
        {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                dx12_shader_compiler: wgpu::Dx12Compiler::default(),
                flags: wgpu::InstanceFlags::default(),
                gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
            });

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await
                .ok_or_else(|| ToadStoolError::runtime("No WebGPU adapter available"))?;

            Ok(WebGPUAdapter { instance, adapter })
        }
        #[cfg(not(feature = "webgpu"))]
        {
            Err(ToadStoolError::runtime("WebGPU feature not enabled"))
        }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ParallelComputeFramework for WebGpuFramework {
    fn framework_type(&self) -> GpuFramework {
        GpuFramework::WebGpu
    }

    async fn discover_devices(&self) -> ToadStoolResult<Vec<UniversalComputeDevice>> {
        #[cfg(feature = "webgpu")]
        {
            match self.initialize_webgpu().await {
                Ok(webgpu_adapter) => {
                    let info = webgpu_adapter.adapter.get_info();
                    let limits = webgpu_adapter.adapter.limits();

                    let mut extensions = HashMap::new();
                    extensions.insert("WebGPU".to_string(), true);

                    // Estimate performance from device type and memory (wgpu doesn't expose these)
                    let (memory_bandwidth_gbps, peak_gflops_fp32) = match info.device_type {
                        wgpu::DeviceType::DiscreteGpu => (448.0, 12_000.0), // Typical mid-range discrete
                        wgpu::DeviceType::IntegratedGpu => (76.8, 1_200.0), // Typical integrated
                        wgpu::DeviceType::VirtualGpu => (100.0, 2_000.0), // Conservative for virtual
                        wgpu::DeviceType::Cpu => (25.6, 100.0),           // Software renderer
                        _ => (100.0, 1_000.0),
                    };

                    let device_name = info.name;
                    let handle_name = device_name.clone();
                    let device = UniversalComputeDevice {
                        id: DeviceId {
                            framework: GpuFramework::WebGpu,
                            device_index: 0,
                            uuid: Uuid::new_v4().to_string(),
                        },
                        info: DeviceInfo {
                            name: device_name,
                            vendor: info.vendor.to_string(),
                            device_type: match info.device_type {
                                wgpu::DeviceType::DiscreteGpu => DeviceType::DiscreteGpu,
                                wgpu::DeviceType::IntegratedGpu => DeviceType::IntegratedGpu,
                                wgpu::DeviceType::VirtualGpu => DeviceType::VirtualGpu,
                                wgpu::DeviceType::Cpu => DeviceType::Other("CPU".to_string()),
                                _ => DeviceType::Other("Unknown".to_string()),
                            },
                            driver_version: info.driver_info,
                            architecture: "WebGPU".to_string(),
                            physical_location: None,
                        },
                        capabilities: DeviceCapabilities {
                            compute_capability: "WebGPU".to_string(),
                            total_memory_bytes: limits.max_buffer_size,
                            memory_bandwidth_gbps,
                            compute_units: limits.max_compute_workgroups_per_dimension,
                            max_work_group_size: (
                                limits.max_compute_workgroup_size_x,
                                limits.max_compute_workgroup_size_y,
                                limits.max_compute_workgroup_size_z,
                            ),
                            supported_data_types: vec![
                                DataType::Float32,
                                DataType::Int32,
                                DataType::UInt32,
                                DataType::Float16,
                            ],
                            extensions,
                            performance: PerformanceCharacteristics {
                                peak_gflops_fp32,
                                peak_gflops_fp64: None,
                                peak_gflops_fp16: None,
                                peak_memory_bandwidth_utilization: 0.8,
                                typical_power_watts: 65.0,
                                max_power_watts: 250.0,
                            },
                        },
                        usage: Arc::new(RwLock::new(DeviceUsage::default())),
                        framework_handle: Some(FrameworkHandle::Unavailable {
                            name: handle_name,
                            reason: "wgpu::Device handle deferred until create_session".to_string(),
                        }),
                    };

                    Ok(vec![device])
                }
                Err(e) => {
                    tracing::warn!("WebGPU device discovery failed: {}", e);
                    Ok(vec![])
                }
            }
        }
        #[cfg(not(feature = "webgpu"))]
        {
            tracing::info!("WebGPU feature not enabled, no devices discovered");
            Ok(vec![])
        }
    }

    async fn create_session(&self, _device_id: &DeviceId) -> ToadStoolResult<Uuid> {
        #[cfg(feature = "webgpu")]
        {
            match self.initialize_webgpu().await {
                Ok(webgpu_adapter) => {
                    let (_device, _queue) = webgpu_adapter
                        .adapter
                        .request_device(
                            &wgpu::DeviceDescriptor {
                                label: Some(&format!(
                                    "ToadStool WebGPU Session {}",
                                    _device_id.uuid
                                )),
                                required_features: wgpu::Features::empty(),
                                required_limits: wgpu::Limits::default(),
                                memory_hints: wgpu::MemoryHints::default(),
                            },
                            None,
                        )
                        .await
                        .map_err(|e| {
                            ToadStoolError::runtime(format!("Failed to create WebGPU device: {e}"))
                        })?;

                    let session_id = Uuid::new_v4();
                    tracing::info!(
                        "Created WebGPU session {} for device {}",
                        session_id,
                        _device_id.uuid
                    );
                    Ok(session_id)
                }
                Err(e) => Err(e),
            }
        }
        #[cfg(not(feature = "webgpu"))]
        {
            Err(ToadStoolError::runtime("WebGPU feature not enabled"))
        }
    }

    async fn compile_kernel(
        &self,
        session_id: Uuid,
        kernel_source: &str,
        format: KernelFormat,
    ) -> ToadStoolResult<CompiledKernel> {
        match format {
            KernelFormat::Glsl => {
                // For GLSL compute shaders, we could compile to SPIR-V
                tracing::info!("Compiling GLSL kernel for WebGPU session {}", session_id);

                // Basic validation
                if kernel_source.is_empty() {
                    return Err(ToadStoolError::runtime("Empty kernel source"));
                }

                if !kernel_source.contains("main") {
                    return Err(ToadStoolError::runtime("Kernel must contain main function"));
                }

                Ok(CompiledKernel {
                    id: Uuid::new_v4().to_string(),
                    binary: bytes::Bytes::copy_from_slice(kernel_source.as_bytes()),
                    framework: GpuFramework::WebGpu,
                    compiled_at: std::time::Instant::now(),
                    optimization_level: super::config::OptimizationLevel::Basic,
                    resource_requirements: ResourceAllocation {
                        memory_bytes: 1024 * 1024,
                        compute_units: 1,
                        priority: 1,
                    },
                })
            }
            KernelFormat::Spirv => {
                // SPIR-V is directly supported by WebGPU
                tracing::info!("Using SPIR-V kernel for WebGPU session {}", session_id);
                Ok(CompiledKernel {
                    id: Uuid::new_v4().to_string(),
                    binary: bytes::Bytes::copy_from_slice(kernel_source.as_bytes()),
                    framework: GpuFramework::WebGpu,
                    compiled_at: std::time::Instant::now(),
                    optimization_level: super::config::OptimizationLevel::Basic,
                    resource_requirements: ResourceAllocation {
                        memory_bytes: 1024 * 1024,
                        compute_units: 1,
                        priority: 1,
                    },
                })
            }
            _ => Err(ToadStoolError::runtime(format!(
                "Unsupported kernel format {format:?} for WebGPU"
            ))),
        }
    }

    async fn execute_kernel(
        &self,
        session_id: Uuid,
        kernel: &CompiledKernel,
        inputs: &[KernelInput],
    ) -> ToadStoolResult<KernelOutput> {
        tracing::info!(
            "Executing kernel {} on WebGPU session {}",
            kernel.id,
            session_id
        );

        // Validate inputs
        if inputs.is_empty() {
            return Err(ToadStoolError::runtime(
                "No inputs provided for kernel execution",
            ));
        }

        let start_time = std::time::Instant::now();

        // WebGPU kernel execution goes through the network / shader compute `ComputeDispatch`.
        // This trait-based path exists for the generic runtime orchestrator;
        // direct use of the compute stack (Tensor ops, ComputeDispatch) is preferred.
        let execution_time = start_time.elapsed();

        Err(ToadStoolError::runtime(format!(
            "WebGPU kernel execution via generic framework trait is not yet wired to the network-service ComputeDispatch. \
             Use Tensor operations on the compute stack directly. Kernel: {}, session: {}, inputs: {}, elapsed: {:?}",
            kernel.id,
            session_id,
            inputs.len(),
            execution_time,
        )))
    }

    async fn destroy_session(&self, session_id: Uuid) -> ToadStoolResult<()> {
        tracing::info!("Destroying WebGPU session {}", session_id);
        // In a real implementation, this would cleanup WebGPU resources
        Ok(())
    }

    async fn get_device_usage(&self, _device_id: &DeviceId) -> ToadStoolResult<DeviceUsage> {
        // In a real implementation, this would query actual WebGPU device usage
        Ok(DeviceUsage {
            gpu_utilization_percent: 0.0,
            memory_utilization_percent: 0.0,
            memory_used_bytes: 0,
            temperature_celsius: None,
            power_usage_watts: None,
            active_sessions: 0,
        })
    }

    fn supports_recursion(&self) -> bool {
        // WebGPU supports compute shader dispatch from host
        true
    }

    async fn spawn_recursive_session(
        &self,
        parent_session: Uuid,
        device_id: &DeviceId,
    ) -> ToadStoolResult<Uuid> {
        tracing::info!(
            "Spawning recursive WebGPU session from parent {}",
            parent_session
        );
        self.create_session(device_id).await
    }
}

/// Fallback framework for unsupported platforms
pub struct FallbackFramework {
    framework_type: GpuFramework,
}

impl FallbackFramework {
    /// Creates a fallback framework for unsupported platform.
    #[must_use]
    pub const fn new(framework_type: GpuFramework) -> Self {
        Self { framework_type }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl ParallelComputeFramework for FallbackFramework {
    fn framework_type(&self) -> GpuFramework {
        self.framework_type.clone()
    }

    async fn discover_devices(&self) -> ToadStoolResult<Vec<UniversalComputeDevice>> {
        // Framework requested but not available on this platform — no GPU devices
        tracing::debug!(
            "Framework {} not available on this platform, no devices discovered",
            self.framework_type.name()
        );
        Ok(vec![])
    }

    async fn create_session(&self, _device_id: &DeviceId) -> ToadStoolResult<Uuid> {
        Err(ToadStoolError::runtime(format!(
            "Framework {} not supported on this platform",
            self.framework_type.name()
        )))
    }

    async fn compile_kernel(
        &self,
        _session_id: Uuid,
        _kernel_source: &str,
        _format: KernelFormat,
    ) -> ToadStoolResult<CompiledKernel> {
        Err(ToadStoolError::runtime(format!(
            "Kernel compilation not supported for {}",
            self.framework_type.name()
        )))
    }

    async fn execute_kernel(
        &self,
        _session_id: Uuid,
        _kernel: &CompiledKernel,
        _inputs: &[KernelInput],
    ) -> ToadStoolResult<KernelOutput> {
        Err(ToadStoolError::runtime(format!(
            "Kernel execution not supported for {}",
            self.framework_type.name()
        )))
    }

    async fn destroy_session(&self, _session_id: Uuid) -> ToadStoolResult<()> {
        Ok(())
    }

    async fn get_device_usage(&self, _device_id: &DeviceId) -> ToadStoolResult<DeviceUsage> {
        Ok(DeviceUsage::default())
    }

    fn supports_recursion(&self) -> bool {
        false
    }

    async fn spawn_recursive_session(
        &self,
        _parent_session: Uuid,
        _device_id: &DeviceId,
    ) -> ToadStoolResult<Uuid> {
        Err(ToadStoolError::runtime(format!(
            "Recursive execution not supported for {}",
            self.framework_type.name()
        )))
    }
}
