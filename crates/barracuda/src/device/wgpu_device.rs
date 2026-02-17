//! Pure WGSL device - hardware-agnostic compute via WebGPU
//!
//! **Pure WGSL Architecture**:
//! - WGSL shaders ONLY (no separate CPU code!)
//! - wgpu handles execution on ANY device (GPU/CPU/NPU/TPU)
//! - Single implementation per operation
//! - Let wgpu experts handle backend optimization
//!
//! ## Adapter Selection
//!
//! Set `BARRACUDA_GPU_ADAPTER` environment variable to control GPU selection:
//!
//! - `BARRACUDA_GPU_ADAPTER=0` — Select first adapter
//! - `BARRACUDA_GPU_ADAPTER=titan` — Select adapter containing "titan" (case-insensitive)
//! - `BARRACUDA_GPU_ADAPTER=auto` — Use wgpu HighPerformance (default)
//!
//! Numeric values that exceed adapter count fall through to name matching.
//! This enables "4070" to match "NVIDIA GeForce RTX 4070" even when parsed as number.
//!
//! ## Multi-GPU Coexistence
//!
//! Multiple GPUs with different drivers (e.g., nvidia proprietary + NVK/nouveau)
//! can coexist and are all visible to wgpu's `enumerate_adapters()`.

use crate::error::{BarracudaError, Result};
use std::sync::Arc;

use super::autotune::{GpuCalibration, GLOBAL_TUNER};

/// Environment variable for adapter selection
const ADAPTER_ENV_VAR: &str = "BARRACUDA_GPU_ADAPTER";

/// WebGPU device - executes WGSL on any hardware
///
/// wgpu automatically selects best backend:
/// - Vulkan (NVIDIA, AMD, Intel GPUs)
/// - Metal (Apple GPUs)
/// - DX12 (Windows GPUs)
/// - Software rasterizer (CPU fallback)
/// - Custom (NPU/TPU if driver available)
///
/// **Auto-Tuning**: Each device can be calibrated at runtime to discover
/// optimal parameters (workgroup size, batch size) for the specific hardware.
/// This handles silicon lottery, generation differences, and unknown cards.
#[derive(Debug, Clone)]
pub struct WgpuDevice {
    pub(crate) device: Arc<wgpu::Device>,
    pub(crate) queue: Arc<wgpu::Queue>,
    adapter_info: wgpu::AdapterInfo,
    /// Cached calibration (lazily populated)
    calibration: Option<GpuCalibration>,
}

impl WgpuDevice {
    /// Create new WebGPU device with auto-discovery
    ///
    /// Prefers GPU, falls back to CPU software rasterizer.
    /// Same WGSL shaders run on any backend - hardware guides its own performance.
    pub async fn new() -> Result<Self> {
        Self::new_with_backend(wgpu::Backends::all()).await
    }

    /// Create device explicitly targeting GPU hardware
    ///
    /// Returns error if no discrete or integrated GPU is available.
    /// Use this when you specifically need GPU acceleration.
    pub async fn new_gpu() -> Result<Self> {
        Self::new_with_filter(wgpu::Backends::all(), |info| {
            matches!(
                info.device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            )
        })
        .await
        .map_err(|_| BarracudaError::device("No GPU adapter found - only CPU available"))
    }

    /// Create device explicitly targeting CPU software rasterizer
    ///
    /// Forces CPU execution even when GPU is available.
    /// Same WGSL shaders execute on CPU via software rasterizer.
    /// Useful for: testing, validation, machines without GPU.
    pub async fn new_cpu() -> Result<Self> {
        Self::new_with_filter(wgpu::Backends::all(), |info| {
            info.device_type == wgpu::DeviceType::Cpu
        })
        .await
        .map_err(|_| BarracudaError::device("No CPU software rasterizer available"))
    }

    /// Create device with high-capacity limits (1GB+ buffers)
    ///
    /// Default wgpu limits cap buffer bindings at 128MB and total buffer at 256MB.
    /// This creates a device with 1GB binding / 2GB buffer limits for large tensors.
    ///
    /// Note: Actual limits depend on hardware - the adapter may support less.
    /// wgpu will negotiate the best available limits.
    ///
    /// # Example
    /// ```rust,ignore
    /// // For tensors larger than 32M elements (128MB at f32)
    /// let device = WgpuDevice::new_high_capacity().await?;
    /// let huge_tensor = Tensor::zeros_on(vec![100_000_000], device).await?;
    /// ```
    pub async fn new_high_capacity() -> Result<Self> {
        Self::new_with_limits(super::tensor_context::high_capacity_limits()).await
    }

    /// Create device with custom limits
    ///
    /// Allows requesting specific wgpu limits for your workload.
    /// The adapter will negotiate actual limits based on hardware support.
    pub async fn new_with_limits(limits: wgpu::Limits) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| BarracudaError::device("No WGPU adapter found"))?;

        let info = adapter.get_info();
        log::info!(
            "BarraCUDA (high-capacity): {} ({:?})",
            info.name,
            info.device_type
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BarraCUDA high-capacity device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: limits,
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        // Log actual limits achieved
        let actual_limits = device.limits();
        log::info!(
            "Limits: max_binding={}MB, max_buffer={}MB",
            actual_limits.max_storage_buffer_binding_size / (1 << 20),
            actual_limits.max_buffer_size / (1 << 20),
        );

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info: info,
            calibration: None,
        })
    }

    /// Create device from ToadStool hardware selection
    ///
    /// ToadStool discovers hardware, BarraCUDA creates the right device.
    /// This is the production path: ToadStool decides what hardware,
    /// BarraCUDA runs the same WGSL on it.
    pub async fn from_selection(
        selection: super::toadstool_integration::DeviceSelection,
    ) -> Result<Self> {
        use super::toadstool_integration::DeviceSelection;
        match selection {
            DeviceSelection::Gpu => Self::new_gpu().await,
            DeviceSelection::Cpu => Self::new_cpu().await,
            DeviceSelection::Npu => {
                // NPU doesn't run WGSL - fall back to GPU, then CPU
                log::info!(
                    "NPU selected but WGSL not supported on NPU; falling back to best WGPU adapter"
                );
                Self::new().await
            }
        }
    }

    /// List all available WGPU adapters (raw, may include duplicates)
    ///
    /// Returns adapter info for every compute device WGPU can see.
    /// **Note**: The same physical GPU may appear multiple times through
    /// different backends (Vulkan, OpenCL, etc.). For deduplicated devices,
    /// use `enumerate_physical_devices()` instead.
    ///
    /// ToadStool uses this to understand what BarraCUDA can target.
    pub fn enumerate_adapters() -> Vec<wgpu::AdapterInfo> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        instance
            .enumerate_adapters(wgpu::Backends::all())
            .iter()
            .map(|a| a.get_info())
            .collect()
    }

    /// List unique physical devices (deduplicated by hardware)
    ///
    /// Returns one entry per physical device, regardless of how many backends
    /// can access it. For example, an RTX 3090 accessible via both Vulkan and
    /// OpenGL appears as ONE device.
    ///
    /// **Preferred over `enumerate_adapters()`** for device selection and
    /// workload distribution to avoid sending duplicate work to the same GPU.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use barracuda::device::WgpuDevice;
    ///
    /// let devices = WgpuDevice::enumerate_physical_devices();
    /// for device in &devices {
    ///     println!("{}: {:?} (f64: {})",
    ///         device.name,
    ///         device.vendor,
    ///         device.capabilities.f64_shaders
    ///     );
    /// }
    /// ```
    pub fn enumerate_physical_devices() -> Vec<super::registry::PhysicalDevice> {
        super::registry::DeviceRegistry::global()
            .physical_devices()
            .cloned()
            .collect()
    }

    /// Get the global device registry
    ///
    /// The registry provides detailed information about physical devices,
    /// their backends, and capabilities. It deduplicates devices that
    /// appear through multiple backends.
    pub fn registry() -> &'static super::registry::DeviceRegistry {
        super::registry::DeviceRegistry::global()
    }

    /// Create device from a physical device index (using preferred backend)
    ///
    /// Uses the device registry to select the best backend (Vulkan preferred).
    /// The index corresponds to `enumerate_physical_devices()` order.
    ///
    /// **Preferred over `from_adapter_index()`** as it uses deduplicated
    /// physical devices and selects the optimal backend automatically.
    pub async fn from_physical_device(index: usize) -> Result<Self> {
        let registry = super::registry::DeviceRegistry::global();

        let adapter_index = registry.get_preferred_adapter_index(index).ok_or_else(|| {
            BarracudaError::device(format!(
                "Physical device index {} out of bounds (only {} devices available)",
                index,
                registry.device_count()
            ))
        })?;

        Self::from_adapter_index(adapter_index).await
    }

    /// Create device from a physical device with explicit backend
    ///
    /// Allows specifying which backend to use for a physical device.
    /// Returns error if the device doesn't support the requested backend.
    pub async fn from_physical_device_with_backend(
        device_index: usize,
        backend: wgpu::Backend,
    ) -> Result<Self> {
        let registry = super::registry::DeviceRegistry::global();

        let adapter_index = registry
            .get_adapter_for_backend(device_index, backend)
            .ok_or_else(|| {
                let device = registry.get_device(device_index);
                let backends: Vec<_> = device
                    .map(|d| {
                        d.backends
                            .iter()
                            .map(|b| format!("{:?}", b.backend))
                            .collect()
                    })
                    .unwrap_or_default();

                BarracudaError::device(format!(
                    "Backend {:?} not available for device {} (available: {:?})",
                    backend, device_index, backends
                ))
            })?;

        Self::from_adapter_index(adapter_index).await
    }

    /// Create device for the first f64-capable GPU (using preferred backend)
    ///
    /// Returns error if no GPU with f64 shader support is available.
    /// Prefers discrete GPUs over integrated.
    pub async fn new_f64_capable() -> Result<Self> {
        let registry = super::registry::DeviceRegistry::global();

        for (idx, device) in registry.physical_devices().enumerate() {
            if device.capabilities.f64_shaders
                && matches!(
                    device.device_type,
                    wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
                )
            {
                return Self::from_physical_device(idx).await;
            }
        }

        Err(BarracudaError::device(
            "No f64-capable GPU found (NVIDIA Pascal+, AMD GCN+, or Intel required)",
        ))
    }

    /// Create device using `BARRACUDA_GPU_ADAPTER` environment variable
    ///
    /// Selection modes:
    /// - Numeric index: `BARRACUDA_GPU_ADAPTER=0` selects first adapter
    /// - Name match: `BARRACUDA_GPU_ADAPTER=titan` matches "TITAN V" (case-insensitive)
    /// - Auto: `BARRACUDA_GPU_ADAPTER=auto` or unset uses HighPerformance
    ///
    /// **Key insight**: Numeric values that exceed adapter count fall through to
    /// name matching. This allows "4070" to match "NVIDIA GeForce RTX 4070".
    ///
    /// # Example
    /// ```rust,ignore
    /// // In shell: export BARRACUDA_GPU_ADAPTER=titan
    /// let device = WgpuDevice::from_env().await?;
    /// // → selects "NVIDIA TITAN V" if available
    /// ```
    pub async fn from_env() -> Result<Self> {
        let selector = std::env::var(ADAPTER_ENV_VAR).unwrap_or_else(|_| "auto".to_string());
        Self::with_adapter_selector(&selector).await
    }

    /// Create device with explicit adapter selector
    ///
    /// Selector modes:
    /// - `"auto"` — wgpu HighPerformance power preference (default)
    /// - `"0"`, `"1"`, etc. — adapter index from `enumerate_adapters()`
    /// - `"titan"`, `"4070"`, etc. — case-insensitive name substring match
    ///
    /// Numeric selectors that exceed adapter count fall through to name matching.
    ///
    /// # Example
    /// ```rust,ignore
    /// // By index
    /// let device = WgpuDevice::with_adapter_selector("0").await?;
    ///
    /// // By name
    /// let device = WgpuDevice::with_adapter_selector("titan").await?;
    /// ```
    pub async fn with_adapter_selector(selector: &str) -> Result<Self> {
        let selector = selector.trim().to_lowercase();

        if selector == "auto" || selector.is_empty() {
            log::info!("Adapter selection: auto (HighPerformance)");
            return Self::new().await;
        }

        let adapters = Self::enumerate_adapters();

        if adapters.is_empty() {
            return Err(BarracudaError::device("No adapters available"));
        }

        // Try numeric index first
        if let Ok(index) = selector.parse::<usize>() {
            if index < adapters.len() {
                log::info!(
                    "Adapter selection: index {} → {}",
                    index,
                    adapters[index].name
                );
                return Self::from_adapter_index(index).await;
            }
            // Fall through to name matching if index out of bounds
            // This allows "4070" to match "NVIDIA GeForce RTX 4070"
            log::debug!(
                "Adapter index {} out of bounds ({}), trying name match",
                index,
                adapters.len()
            );
        }

        // Name substring match (case-insensitive)
        for (index, info) in adapters.iter().enumerate() {
            if info.name.to_lowercase().contains(&selector) {
                log::info!(
                    "Adapter selection: '{}' → {} (index {})",
                    selector,
                    info.name,
                    index
                );
                return Self::from_adapter_index(index).await;
            }
        }

        // No match found
        let available: Vec<_> = adapters.iter().map(|a| a.name.as_str()).collect();
        Err(BarracudaError::device(format!(
            "No adapter matches '{}'. Available: {:?}",
            selector, available
        )))
    }

    /// Create with specific backend (for testing/multi-GPU)
    pub async fn new_with_backend(backends: wgpu::Backends) -> Result<Self> {
        Self::new_with_filter(backends, |_| true).await
    }

    /// Create device from a specific adapter index
    ///
    /// Uses the index from `enumerate_adapters()` to select a specific GPU.
    /// Useful for multi-GPU setups where you want explicit control.
    pub async fn from_adapter_index(index: usize) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters: Vec<wgpu::Adapter> = instance.enumerate_adapters(wgpu::Backends::all());

        if index >= adapters.len() {
            return Err(BarracudaError::device(format!(
                "Adapter index {index} out of bounds (only {} adapters available)",
                adapters.len()
            )));
        }

        let adapter = &adapters[index];
        let info = adapter.get_info();

        log::info!(
            "Selecting adapter {index}: {} ({:?})",
            info.name,
            info.device_type
        );

        // BUG FIX (Feb 16 2026 — hotSpring finding):
        // Must request SHADER_F64 when adapter supports it, otherwise all f64
        // WGSL shaders fail with "Using f64 values requires FLOAT64 flag".
        // Also request SHADER_F16 and TIMESTAMP_QUERY if available.
        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        if adapter_features.contains(wgpu::Features::SHADER_F64) {
            required_features |= wgpu::Features::SHADER_F64;
            log::info!("  SHADER_F64: enabled");
        }
        if adapter_features.contains(wgpu::Features::SHADER_F16) {
            required_features |= wgpu::Features::SHADER_F16;
            log::info!("  SHADER_F16: enabled");
        }
        if adapter_features.contains(wgpu::Features::TIMESTAMP_QUERY) {
            required_features |= wgpu::Features::TIMESTAMP_QUERY;
        }

        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BarraCUDA device"),
                    required_features,
                    required_limits: super::tensor_context::science_limits(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info: info,
            calibration: None,
        })
    }

    /// Create with custom filter (for specific GPU selection)
    pub async fn new_with_filter<F>(backends: wgpu::Backends, filter: F) -> Result<Self>
    where
        F: Fn(&wgpu::AdapterInfo) -> bool,
    {
        // Create instance (pure Rust runtime discovery)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Enumerate adapters
        let adapters = instance.enumerate_adapters(backends);

        if adapters.is_empty() {
            return Err(BarracudaError::device(
                "No WGPU adapters found (need GPU or CPU software rasterizer)",
            ));
        }

        // Find matching adapter
        let adapter = adapters
            .into_iter()
            .find(|adapter: &wgpu::Adapter| filter(&adapter.get_info()))
            .ok_or_else(|| BarracudaError::device("No adapter matching requested hardware type"))?;

        let adapter_info = adapter.get_info();

        // Request device with science-grade limits (runtime capability negotiation)
        // Default limits raised from 128 MiB to 512 MiB for storage buffer binding,
        // and from 256 MiB to 1 GiB for max buffer size.
        // Validated by hotSpring nuclear EOS study (169/169 acceptance checks).
        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("barraCUDA Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: super::tensor_context::science_limits(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {}", e)))?;

        // Log what device we're using
        log::info!(
            "barraCUDA initialized: {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
            calibration: None,
        })
    }

    /// Create WgpuDevice from existing wgpu device and queue
    ///
    /// This allows integration with code that already has wgpu::Device/Queue,
    /// such as PppmGpu or external libraries.
    ///
    /// # Arguments
    /// * `device` - Existing wgpu device
    /// * `queue` - Existing wgpu queue
    /// * `adapter_info` - Adapter info (for device metadata)
    ///
    /// # Example
    /// ```rust,ignore
    /// // If you have existing wgpu resources
    /// let wgpu_dev = WgpuDevice::from_existing(device, queue, info);
    /// // Now can use with Tensor, FFT, etc.
    /// ```
    pub fn from_existing(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        adapter_info: wgpu::AdapterInfo,
    ) -> Self {
        Self {
            device,
            queue,
            adapter_info,
            calibration: None,
        }
    }

    /// Create WgpuDevice from existing device/queue with synthetic adapter info
    ///
    /// Use when you don't have the original adapter info. Creates synthetic
    /// metadata that marks this as an "external" device.
    pub fn from_existing_simple(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            adapter_info: wgpu::AdapterInfo {
                name: "External Device".to_string(),
                vendor: 0,
                device: 0,
                device_type: wgpu::DeviceType::Other,
                driver: "external".to_string(),
                driver_info: "wrapped from existing wgpu resources".to_string(),
                backend: wgpu::Backend::Vulkan, // Reasonable default
            },
            calibration: None,
        }
    }

    /// Get device name (e.g., "NVIDIA RTX 4090", "llvmpipe (CPU)")
    pub fn name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Get device type (DiscreteGpu, IntegratedGpu, Cpu, etc.)
    pub fn device_type(&self) -> wgpu::DeviceType {
        self.adapter_info.device_type
    }

    /// Check if running on CPU fallback
    pub fn is_cpu(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }

    /// Access underlying wgpu device
    ///
    /// **Deep Debt**: Enables external consumers to use barraCUDA infrastructure
    /// for custom operations (e.g., homomorphic computing, neuromorphic, etc.)
    ///
    /// # Safety
    /// External users must ensure proper synchronization with the queue.
    /// Use `queue()` for command submission.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Get Arc-wrapped device (for shared ownership in TensorContext)
    ///
    /// Returns a clone of the internal `Arc<wgpu::Device>` for use cases
    /// that need shared ownership, like buffer pools and tensor contexts.
    pub fn device_arc(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    /// Get adapter info (for capability detection)
    ///
    /// **Deep Debt**: Runtime device information for capability-based execution
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Access command queue
    ///
    /// **Deep Debt**: Enables external consumers to submit custom compute passes
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Create storage buffer (convenience helper)
    ///
    /// **Deep Debt**: Reduces boilerplate for external barraCUDA users
    ///
    /// # Example
    /// ```rust,no_run
    /// # use barracuda::prelude::*;
    /// # async fn example() -> Result<()> {
    /// let device = WgpuDevice::new().await?;
    /// let buffer = device.create_storage_buffer("my_data", &[1u8, 2, 3, 4]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_storage_buffer(&self, label: &str, data: &[u8]) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: data,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            })
    }

    /// Create uniform buffer (convenience helper)
    ///
    /// **Deep Debt**: Type-safe uniform buffer creation
    ///
    /// # Example
    /// ```rust,no_run
    /// # use barracuda::prelude::*;
    /// # use bytemuck::{Pod, Zeroable};
    /// # async fn example() -> Result<()> {
    /// #[repr(C)]
    /// #[derive(Copy, Clone, Pod, Zeroable)]
    /// struct Params {
    ///     width: u32,
    ///     height: u32,
    /// }
    ///
    /// let device = WgpuDevice::new().await?;
    /// let params = Params { width: 1920, height: 1080 };
    /// let buffer = device.create_uniform_buffer("params", &params);
    /// # Ok(())
    /// # }
    /// ```
    pub fn create_uniform_buffer<T: bytemuck::Pod>(&self, label: &str, data: &T) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::bytes_of(data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
    }

    /// Allocate buffer for f32 data
    pub fn create_buffer_f32(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    /// Allocate buffer for u32 data
    pub fn create_buffer_u32(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA U32 Buffer"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    /// Allocate zero-initialized buffer for u32 data
    pub fn create_buffer_u32_zeros(&self, size: usize) -> Result<wgpu::Buffer> {
        use wgpu::util::DeviceExt;
        let zeros = vec![0u32; size];
        Ok(self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("barraCUDA U32 Zeros Buffer"),
                contents: bytemuck::cast_slice(&zeros),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }))
    }

    /// Allocate buffer for f64 data
    ///
    /// **Deep Debt Evolution (Feb 16, 2026)**:
    /// Science-grade f64 precision for scientific computing.
    /// Native Vulkan fp64 support via WebGPU.
    pub fn create_buffer_f64(&self, size: usize) -> Result<wgpu::Buffer> {
        Ok(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("barraCUDA F64 Buffer"),
            size: (size * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }))
    }

    /// Compile WGSL shader
    pub fn compile_shader(&self, source: &str, label: Option<&str>) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    /// Execute WGSL compute shader
    pub fn execute_compute(
        &self,
        shader_source: &str,
        bind_groups: &[&wgpu::BindGroup],
        workgroups: (u32, u32, u32),
    ) -> Result<()> {
        // Compile WGSL
        let shader = self.compile_shader(shader_source, Some("barraCUDA Operation"));

        // Create pipeline
        let pipeline = self
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("barraCUDA Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Encode and submit
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("barraCUDA Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("barraCUDA Compute"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            for (i, bind_group) in bind_groups.iter().enumerate() {
                pass.set_bind_group(i as u32, bind_group, &[]);
            }
            pass.dispatch_workgroups(workgroups.0, workgroups.1, workgroups.2);
        }

        self.queue.submit(Some(encoder.finish()));

        Ok(())
    }
}

impl WgpuDevice {
    /// Read buffer to host memory
    pub fn read_buffer_f32(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<f32>> {
        if size == 0 {
            return Ok(Vec::new());
        }
        // Create staging buffer
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy GPU -> staging
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);

        // Wait for mapping
        futures::executor::block_on(receiver)
            .map_err(|_| BarracudaError::gpu("Failed to map buffer"))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        // Copy data
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Write data to buffer
    pub fn write_buffer_f32(&self, buffer: &wgpu::Buffer, data: &[f32]) -> Result<()> {
        self.queue
            .write_buffer(buffer, 0, bytemuck::cast_slice(data));
        Ok(())
    }

    /// Read f64 buffer to host memory
    ///
    /// Used for high-precision operations (PPPM, FFT f64, sparse solvers)
    pub fn read_buffer_f64(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<f64>> {
        if size == 0 {
            return Ok(Vec::new());
        }
        // Create staging buffer
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer f64"),
            size: (size * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy GPU -> staging
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<f64>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);

        // Wait for mapping
        futures::executor::block_on(receiver)
            .map_err(|_| BarracudaError::gpu("Failed to map buffer"))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        // Copy data
        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    /// Read u32 buffer to host memory
    pub fn read_buffer_u32(&self, buffer: &wgpu::Buffer, size: usize) -> Result<Vec<u32>> {
        // Create staging buffer
        let staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging Buffer"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy GPU -> staging
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
            buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<u32>()) as u64,
        );
        self.queue.submit(Some(encoder.finish()));

        // Map and read
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::Maintain::Wait);

        // Wait for mapping
        futures::executor::block_on(receiver)
            .map_err(|_| BarracudaError::gpu("Failed to map buffer"))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        // Copy data
        let data = buffer_slice.get_mapped_range();
        let result: Vec<u32> = bytemuck::cast_slice(&data).to_vec();

        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }

    // =========================================================================
    // AUTO-TUNING API
    // =========================================================================

    /// Get calibration for this device (from cache or run calibration)
    ///
    /// Calibration discovers optimal workgroup size, batch size, and measures
    /// actual bandwidth for this specific hardware.
    ///
    /// Results are cached globally and persisted to disk, so calibration only
    /// runs once per unique GPU.
    pub fn get_calibration(&self) -> GpuCalibration {
        GLOBAL_TUNER.get_or_calibrate(&self.device, &self.queue, &self.adapter_info.name)
    }

    /// Force recalibration (use after driver updates or hardware changes)
    pub fn recalibrate(&self) -> GpuCalibration {
        GLOBAL_TUNER.recalibrate(&self.device, &self.queue, &self.adapter_info.name)
    }

    /// Get optimal workgroup size for this device
    ///
    /// This is the primary tuned parameter - affects all compute dispatches.
    /// Uses cached calibration or falls back to safe default (256).
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
    ///
    /// Use this for production workloads to ensure optimal settings from start.
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
        // Should always succeed (wgpu has CPU fallback)
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };
        println!("barraCUDA device: {}", device.name());
        println!("Device type: {:?}", device.device_type());

        if device.is_cpu() {
            println!("  Using CPU software rasterizer");
        } else {
            println!("  Using GPU acceleration");
        }
    }

    #[tokio::test]
    async fn test_enumerate_adapters() {
        let adapters = WgpuDevice::enumerate_adapters();
        println!("Found {} WGPU adapters:", adapters.len());
        for info in &adapters {
            println!(
                "  - {} ({:?}, {:?})",
                info.name, info.device_type, info.backend
            );
        }
        // Should find at least one adapter (GPU or CPU)
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

        // Create buffer
        let buffer = device.create_buffer_f32(10).unwrap();

        // Write data
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        device.write_buffer_f32(&buffer, &data).unwrap();

        // Read back
        let read_data = device.read_buffer_f32(&buffer, 10).unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_from_selection_gpu() {
        use super::super::toadstool_integration::DeviceSelection;
        // Try GPU selection - may fail if no GPU, that's ok
        match WgpuDevice::from_selection(DeviceSelection::Gpu).await {
            Ok(device) => {
                assert!(
                    !device.is_cpu(),
                    "GPU selection should not return CPU device"
                );
                println!("  GPU device: {}", device.name());
            }
            Err(_) => {
                println!("  No GPU available (expected on CI/headless)");
            }
        }
    }

    #[tokio::test]
    async fn test_from_selection_cpu() {
        use super::super::toadstool_integration::DeviceSelection;
        // CPU selection - may not have software rasterizer on all systems
        match WgpuDevice::from_selection(DeviceSelection::Cpu).await {
            Ok(device) => {
                assert!(device.is_cpu(), "CPU selection should return CPU device");
                println!("  CPU device: {}", device.name());
            }
            Err(_) => {
                println!("  No CPU software rasterizer available");
            }
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_auto() {
        // "auto" should work like the default new()
        match WgpuDevice::with_adapter_selector("auto").await {
            Ok(device) => {
                println!("Auto-selected: {} ({:?})", device.name(), device.device_type());
            }
            Err(e) => {
                println!("No adapter available: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_index() {
        let adapters = WgpuDevice::enumerate_adapters();
        if adapters.is_empty() {
            println!("No adapters to test index selection");
            return;
        }

        // Select first adapter by index
        match WgpuDevice::with_adapter_selector("0").await {
            Ok(device) => {
                println!("Index 0: {} ({:?})", device.name(), device.device_type());
                // Should match first adapter
                assert_eq!(device.name(), adapters[0].name);
            }
            Err(e) => {
                panic!("Index 0 should succeed when adapters exist: {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_name_match() {
        let adapters = WgpuDevice::enumerate_adapters();
        if adapters.is_empty() {
            println!("No adapters to test name matching");
            return;
        }

        // Get first 3 chars of first adapter name (lowercase)
        let first_name = &adapters[0].name;
        let partial = first_name.chars().take(4).collect::<String>().to_lowercase();

        println!("Testing name match with partial: '{partial}'");

        match WgpuDevice::with_adapter_selector(&partial).await {
            Ok(device) => {
                println!("Name match: {} ({:?})", device.name(), device.device_type());
                // Should match an adapter containing the partial name
                assert!(device.name().to_lowercase().contains(&partial));
            }
            Err(e) => {
                // Might fail if partial doesn't uniquely match
                println!("Name match failed (expected if ambiguous): {e}");
            }
        }
    }

    #[tokio::test]
    async fn test_adapter_selector_fallthrough() {
        // Test that large numeric indices fall through to name matching
        // e.g., "4070" as index (probably > adapter count) should match "RTX 4070"
        let adapters = WgpuDevice::enumerate_adapters();

        // Use a number larger than adapter count
        let large_index = (adapters.len() + 1000).to_string();

        // This should fail with "no adapter matches"
        match WgpuDevice::with_adapter_selector(&large_index).await {
            Ok(_) => {
                // Only succeeds if there's an adapter with this name substring
                println!("Unexpectedly found adapter matching '{large_index}'");
            }
            Err(e) => {
                // Expected: should report available adapters
                println!("Expected error: {e}");
                assert!(e.to_string().contains("No adapter matches"));
            }
        }
    }

    #[tokio::test]
    async fn test_from_env_default() {
        // With no env var set, should default to auto
        std::env::remove_var(ADAPTER_ENV_VAR);

        match WgpuDevice::from_env().await {
            Ok(device) => {
                println!("Env default: {} ({:?})", device.name(), device.device_type());
            }
            Err(e) => {
                println!("No adapter available: {e}");
            }
        }
    }
}
