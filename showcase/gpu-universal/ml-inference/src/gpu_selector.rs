//! GPU Discovery and Selection
//!
//! Discovers available GPUs and provides capability-based selection.
//! Modern, idiomatic Rust with proper error handling.

use anyhow::Result;
#[cfg(any(feature = "vulkan"))]
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Information about a discovered GPU
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    /// Vendor (NVIDIA, AMD, Intel, etc.)
    pub vendor: String,
    /// Device name (e.g., "GeForce RTX 3090")
    pub name: String,
    /// Total memory in GB
    pub memory_gb: f32,
    /// Number of compute units (CUDA cores, stream processors, etc.)
    pub compute_units: u32,
    /// Backend used to access this GPU
    pub backend: GpuBackend,
    /// Device index (for multi-GPU systems)
    pub device_index: usize,
}

impl fmt::Display for GpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} ({:.1} GB, {} CUs, {:?})",
            self.vendor, self.name, self.memory_gb, self.compute_units, self.backend
        )
    }
}

/// GPU backend/API used for compute
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuBackend {
    /// NVIDIA CUDA (native, highest performance)
    Cuda,
    /// OpenCL (cross-vendor, widely supported)
    OpenCL,
    /// Vulkan Compute (modern, cross-vendor)
    Vulkan,
    /// WebGPU (most portable, future-proof)
    WebGPU,
    /// AMD ROCm/HIP (AMD native)
    ROCm,
}

impl fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cuda => write!(f, "CUDA"),
            Self::OpenCL => write!(f, "OpenCL"),
            Self::Vulkan => write!(f, "Vulkan"),
            Self::WebGPU => write!(f, "WebGPU"),
            Self::ROCm => write!(f, "ROCm"),
        }
    }
}

/// GPU discovery service
pub struct GpuSelector;

impl GpuSelector {
    /// Discover all available GPUs across all backends
    ///
    /// Returns GPUs sorted by compute capability (highest first)
    /// Automatically deduplicates GPUs discovered via multiple backends
    pub fn discover_all() -> Result<Vec<GpuInfo>> {
        let mut all_gpus = Vec::new();

        // Try CUDA (NVIDIA)
        #[cfg(feature = "cuda")]
        if let Ok(cuda_gpus) = Self::discover_cuda() {
            all_gpus.extend(cuda_gpus);
        }

        // Try OpenCL (NVIDIA, AMD, Intel)
        #[cfg(feature = "opencl")]
        if let Ok(opencl_gpus) = Self::discover_opencl() {
            all_gpus.extend(opencl_gpus);
        }

        // Try Vulkan (NVIDIA, AMD, Intel - modern API)
        #[cfg(feature = "vulkan")]
        if let Ok(vulkan_gpus) = Self::discover_vulkan() {
            all_gpus.extend(vulkan_gpus);
        }

        // Try WebGPU (universal)
        if let Ok(webgpu_gpus) = Self::discover_webgpu() {
            all_gpus.extend(webgpu_gpus);
        }

        // Deduplicate: same GPU discovered via different backends
        // Keep the "best" backend for each unique GPU (CUDA > ROCm > OpenCL > WebGPU)
        let gpus = Self::deduplicate_gpus(all_gpus);

        // Sort by compute capability (descending)
        let mut sorted_gpus = gpus;
        sorted_gpus.sort_by(|a, b| {
            b.compute_units
                .cmp(&a.compute_units)
                .then_with(|| b.memory_gb.partial_cmp(&a.memory_gb).unwrap_or(std::cmp::Ordering::Equal))
        });

        if sorted_gpus.is_empty() {
            anyhow::bail!("No GPUs discovered. Check drivers and feature flags.");
        }

        Ok(sorted_gpus)
    }

    /// Deduplicate GPUs found via multiple backends
    ///
    /// Keeps the "best" backend for each unique GPU:
    /// Priority: CUDA > ROCm > OpenCL > Vulkan > WebGPU
    fn deduplicate_gpus(gpus: Vec<GpuInfo>) -> Vec<GpuInfo> {
        use std::collections::HashMap;

        // Group by (vendor, name) - same physical GPU
        let mut gpu_map: HashMap<(String, String), GpuInfo> = HashMap::new();

        for gpu in gpus {
            let key = (gpu.vendor.clone(), gpu.name.clone());
            
            let should_replace = match gpu_map.get(&key) {
                None => true,
                Some(existing) => {
                    // Replace if new backend is higher priority
                    Self::backend_priority(&gpu.backend) > Self::backend_priority(&existing.backend)
                }
            };

            if should_replace {
                gpu_map.insert(key, gpu);
            }
        }

        gpu_map.into_values().collect()
    }

    /// Backend priority for deduplication
    ///
    /// Higher number = higher priority (native > portable)
    fn backend_priority(backend: &GpuBackend) -> u8 {
        match backend {
            GpuBackend::Cuda => 5,    // NVIDIA native
            GpuBackend::ROCm => 4,    // AMD native
            GpuBackend::OpenCL => 3,  // Cross-vendor
            GpuBackend::Vulkan => 2,  // Modern cross-vendor
            GpuBackend::WebGPU => 1,  // Most portable
        }
    }

    /// Discover NVIDIA GPUs via CUDA
    #[cfg(feature = "cuda")]
    fn discover_cuda() -> Result<Vec<GpuInfo>> {
        use anyhow::Context;
        use cudarc::driver::CudaDevice;

        let device_count = CudaDevice::count()
            .context("Failed to query CUDA device count")?;

        if device_count == 0 {
            return Ok(Vec::new());
        }

        let mut gpus = Vec::new();

        for i in 0..device_count {
            let device_index = i as usize;
            match CudaDevice::new(device_index) {
                Ok(_device) => {
                    // Note: cudarc 0.11 wraps device in Arc and doesn't expose property query methods
                    // For now, use descriptive naming. Full property query requires unsafe CUDA API calls.
                    // This is a known limitation - see ToadStool's cuda_impl.rs for full implementation.
                    
                    let info = GpuInfo {
                        vendor: "NVIDIA".to_string(),
                        name: format!("CUDA Device {} (via CUDA API)", i),
                        memory_gb: 0.0, // Query not exposed by cudarc wrapper
                        compute_units: 0, // Query not exposed by cudarc wrapper
                        backend: GpuBackend::Cuda,
                        device_index,
                    };
                    
                    tracing::info!(
                        "Discovered CUDA GPU: {} (device ordinal: {})",
                        info.name,
                        device_index
                    );
                    
                    gpus.push(info);
                }
                Err(e) => {
                    tracing::warn!("Failed to initialize CUDA device {}: {}", i, e);
                }
            }
        }

        Ok(gpus)
    }

    /// Discover GPUs via OpenCL
    #[cfg(feature = "opencl")]
    fn discover_opencl() -> Result<Vec<GpuInfo>> {
        use ocl::{Device, Platform};

        let platforms = Platform::list();
        if platforms.is_empty() {
            return Ok(Vec::new());
        }

        let mut gpus = Vec::new();
        let mut device_index = 0;

        for platform in platforms {
            let platform_name = platform.name().unwrap_or_else(|_| "Unknown".to_string());

            if let Ok(devices) = Device::list_all(platform) {
                for device in devices {
                    // Only include GPU devices (not CPU)
                    if let Ok(device_type) = device.info(ocl::core::DeviceInfo::Type) {
                        use ocl::core::{DeviceInfoResult, DeviceType};

                        if let DeviceInfoResult::Type(DeviceType::GPU) = device_type {
                            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
                            let vendor = device.vendor().unwrap_or_else(|_| "Unknown".to_string());

                            // Get compute units
                            let compute_units = if let Ok(ocl::core::DeviceInfoResult::MaxComputeUnits(cu)) =
                                device.info(ocl::core::DeviceInfo::MaxComputeUnits)
                            {
                                cu
                            } else {
                                1
                            };

                            // Get memory
                            let memory_bytes = if let Ok(ocl::core::DeviceInfoResult::GlobalMemSize(mem)) =
                                device.info(ocl::core::DeviceInfo::GlobalMemSize)
                            {
                                mem
                            } else {
                                0
                            };
                            let memory_gb = memory_bytes as f32 / (1024.0 * 1024.0 * 1024.0);

                            let info = GpuInfo {
                                vendor: vendor.clone(),
                                name,
                                memory_gb,
                                compute_units,
                                backend: GpuBackend::OpenCL,
                                device_index,
                            };

                            tracing::info!(
                                "Discovered OpenCL GPU: {} on platform {}",
                                info.name,
                                platform_name
                            );

                            gpus.push(info);
                            device_index += 1;
                        }
                    }
                }
            }
        }

        Ok(gpus)
    }

    /// Discover GPUs via Vulkan
    #[cfg(feature = "vulkan")]
    fn discover_vulkan() -> Result<Vec<GpuInfo>> {
        use ash::vk;

        let mut gpus = Vec::new();

        unsafe {
            // Load Vulkan
            let entry = ash::Entry::load().context("Failed to load Vulkan library")?;

            // Create instance
            let app_name = std::ffi::CString::new("ToadStool GPU Discovery").unwrap();
            let app_info = vk::ApplicationInfo {
                p_application_name: app_name.as_ptr(),
                application_version: vk::make_api_version(0, 1, 0, 0),
                api_version: vk::API_VERSION_1_2,
                ..Default::default()
            };

            let create_info = vk::InstanceCreateInfo {
                p_application_info: &app_info,
                ..Default::default()
            };

            let instance = entry
                .create_instance(&create_info, None)
                .context("Failed to create Vulkan instance")?;

            // Enumerate physical devices
            let physical_devices = instance
                .enumerate_physical_devices()
                .context("Failed to enumerate Vulkan devices")?;

            for (idx, &device) in physical_devices.iter().enumerate() {
                let properties = instance.get_physical_device_properties(device);
                let memory_properties = instance.get_physical_device_memory_properties(device);

                // Get device name
                let name = std::ffi::CStr::from_ptr(properties.device_name.as_ptr())
                    .to_string_lossy()
                    .to_string();

                // Determine vendor
                let vendor = match properties.vendor_id {
                    0x10DE => "NVIDIA",
                    0x1002 => "AMD",
                    0x8086 => "Intel",
                    _ => "Unknown",
                };

                // Calculate total memory (DEVICE_LOCAL heaps)
                let total_memory: u64 = memory_properties
                    .memory_heaps
                    .iter()
                    .take(memory_properties.memory_heap_count as usize)
                    .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
                    .map(|heap| heap.size)
                    .sum();

                let memory_gb = total_memory as f32 / (1024.0 * 1024.0 * 1024.0);

                // Get compute units (work group count is a proxy)
                let compute_units = properties.limits.max_compute_work_group_count[0];

                gpus.push(GpuInfo {
                    vendor: vendor.to_string(),
                    name,
                    memory_gb,
                    compute_units,
                    backend: GpuBackend::Vulkan,
                    device_index: idx,
                });
            }

            // Cleanup
            instance.destroy_instance(None);
        }

        Ok(gpus)
    }

    /// Discover GPUs via WebGPU (vendor-agnostic)
    ///
    /// **Deep Debt Compliance**:
    /// - Runtime GPU detection (no hardcoding)
    /// - Vendor-agnostic (NVIDIA, AMD, Intel, Apple)
    /// - Works with existing tokio runtime or creates temporary one
    /// - Graceful degradation (returns empty on failure)
    fn discover_webgpu() -> Result<Vec<GpuInfo>> {
        // Try to use existing tokio runtime, or create a temporary one
        let gpus = if let Ok(handle) = tokio::runtime::Handle::try_current() {
            // We're already in a tokio runtime - use it
            handle.block_on(Self::discover_webgpu_async())?
        } else {
            // No runtime available - create temporary one
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(Self::discover_webgpu_async())?
        };
        
        Ok(gpus)
    }
    
    /// Async WebGPU discovery implementation
    async fn discover_webgpu_async() -> Result<Vec<GpuInfo>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), // ALL vendors
            ..Default::default()
        });
        
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        let mut gpu_infos = Vec::new();
        
        for (idx, adapter) in adapters.iter().enumerate() {
            let info = adapter.get_info();
            
            // Only include discrete/integrated GPUs (not CPU/virtual)
            if matches!(
                info.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            ) {
                let backend = match info.backend {
                    wgpu::Backend::Vulkan => GpuBackend::Vulkan,
                    wgpu::Backend::Metal => GpuBackend::Metal,
                    wgpu::Backend::Dx12 => GpuBackend::Dx12,
                    wgpu::Backend::Gl => GpuBackend::OpenGl,
                    _ => continue, // Skip unsupported backends
                };
                
                let vendor = if info.name.contains("NVIDIA") {
                    "NVIDIA"
                } else if info.name.contains("AMD") || info.name.contains("Radeon") {
                    "AMD"
                } else if info.name.contains("Intel") {
                    "Intel"
                } else if info.name.contains("Apple") {
                    "Apple"
                } else {
                    "Unknown"
                };
                
                gpu_infos.push(GpuInfo {
                    vendor: vendor.to_string(),
                    name: info.name.clone(),
                    memory_gb: 0.0, // WebGPU doesn't expose memory info
                    compute_units: 0, // Not exposed by WebGPU
                    backend,
                    device_index: idx,
                });
            }
        }
        
        Ok(gpu_infos)
    }

    /// Find NVIDIA GPU (prefers CUDA, falls back to OpenCL)
    pub fn find_nvidia(gpus: &[GpuInfo]) -> Option<&GpuInfo> {
        // Prefer CUDA backend
        gpus.iter()
            .find(|gpu| gpu.vendor.contains("NVIDIA") && gpu.backend == GpuBackend::Cuda)
            .or_else(|| {
                // Fallback to OpenCL
                gpus.iter()
                    .find(|gpu| gpu.vendor.contains("NVIDIA"))
            })
    }

    /// Find AMD GPU (prefers ROCm, then Vulkan, falls back to OpenCL)
    pub fn find_amd(gpus: &[GpuInfo]) -> Option<&GpuInfo> {
        // Prefer ROCm backend (native AMD)
        gpus.iter()
            .find(|gpu| gpu.vendor.contains("AMD") && gpu.backend == GpuBackend::ROCm)
            .or_else(|| {
                // Fallback to Vulkan (works well on AMD via Mesa RADV)
                gpus.iter()
                    .find(|gpu| (gpu.vendor.contains("AMD") || gpu.vendor.contains("Advanced Micro Devices")) 
                        && gpu.backend == GpuBackend::Vulkan)
            })
            .or_else(|| {
                // Last resort: OpenCL
                gpus.iter()
                    .find(|gpu| gpu.vendor.contains("AMD") || gpu.vendor.contains("Advanced Micro Devices"))
            })
    }

    /// Find best GPU by compute capability
    pub fn find_best(gpus: &[GpuInfo]) -> Option<&GpuInfo> {
        gpus.first() // Already sorted by compute capability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_info_display() {
        let gpu = GpuInfo {
            vendor: "NVIDIA".to_string(),
            name: "GeForce RTX 3090".to_string(),
            memory_gb: 24.0,
            compute_units: 10752,
            backend: GpuBackend::Cuda,
            device_index: 0,
        };

        let display = format!("{}", gpu);
        assert!(display.contains("NVIDIA"));
        assert!(display.contains("RTX 3090"));
        assert!(display.contains("24.0 GB"));
    }

    #[test]
    fn test_find_nvidia() {
        let gpus = vec![
            GpuInfo {
                vendor: "AMD".to_string(),
                name: "RX 6950 XT".to_string(),
                memory_gb: 16.0,
                compute_units: 80,
                backend: GpuBackend::OpenCL,
                device_index: 0,
            },
            GpuInfo {
                vendor: "NVIDIA".to_string(),
                name: "RTX 3090".to_string(),
                memory_gb: 24.0,
                compute_units: 10752,
                backend: GpuBackend::Cuda,
                device_index: 1,
            },
        ];

        let nvidia = GpuSelector::find_nvidia(&gpus);
        assert!(nvidia.is_some());
        assert_eq!(nvidia.unwrap().vendor, "NVIDIA");
    }

    #[test]
    fn test_find_amd() {
        let gpus = vec![
            GpuInfo {
                vendor: "NVIDIA".to_string(),
                name: "RTX 3090".to_string(),
                memory_gb: 24.0,
                compute_units: 10752,
                backend: GpuBackend::Cuda,
                device_index: 0,
            },
            GpuInfo {
                vendor: "AMD".to_string(),
                name: "RX 6950 XT".to_string(),
                memory_gb: 16.0,
                compute_units: 80,
                backend: GpuBackend::OpenCL,
                device_index: 1,
            },
        ];

        let amd = GpuSelector::find_amd(&gpus);
        assert!(amd.is_some());
        assert!(amd.unwrap().vendor.contains("AMD"));
    }
}

