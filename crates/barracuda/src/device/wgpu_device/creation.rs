//! Device creation and adapter selection

use super::WgpuDevice;
use crate::device::probe;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;

/// Environment variable for adapter selection
pub const ADAPTER_ENV_VAR: &str = "BARRACUDA_GPU_ADAPTER";

impl WgpuDevice {
    /// Install error handler that flags device-lost without panicking.
    ///
    /// Device-lost is a recoverable condition: the test pool recreates the
    /// device and subsequent operations continue. Panicking on device-lost
    /// kills test threads under parallel load, causing false failures on
    /// machines with real GPUs.
    ///
    /// Non-lost errors still panic — they indicate real bugs.
    fn install_error_handler(
        device: &wgpu::Device,
        lost_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) {
        let flag = Arc::clone(lost_flag);
        device.on_uncaptured_error(Box::new(move |error| {
            let msg = error.to_string();
            if msg.contains("lost") || msg.contains("Lost") || msg.contains("Parent device") {
                flag.store(true, std::sync::atomic::Ordering::Release);
                tracing::warn!("GPU device lost (flagged for pool recovery): {msg}");
                return;
            }
            panic!("wgpu uncaptured error: {msg}");
        }));
    }

    fn make_pipeline_cache(device: &wgpu::Device) -> Option<Arc<wgpu::PipelineCache>> {
        if !device.features().contains(wgpu::Features::PIPELINE_CACHE) {
            return None;
        }
        // SAFETY: `data: None` means empty initial cache — no previous blob to validate.
        // The wgpu unsafe contract applies when `data: Some(...)` contains serialized
        // cache from disk; corrupted data could cause driver UB. With `data: None`, we
        // create a fresh cache with no untrusted input. Violation: if we ever passed
        // `data: Some(blob)` from untrusted source, malformed blob could cause UB.
        #[allow(unsafe_code)]
        Some(Arc::new(unsafe {
            device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
                label: Some("barraCuda pipeline cache"),
                data: None,
                fallback: true,
            })
        }))
    }

    /// Create new WebGPU device with auto-discovery
    ///
    /// Prefers discrete GPU via `HighPerformance` power preference.
    /// Falls back to integrated GPU, then software rasterizer.
    pub async fn new() -> Result<Self> {
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

        Self::from_adapter(adapter).await
    }

    /// Create device explicitly targeting GPU hardware
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
    pub async fn new_cpu() -> Result<Self> {
        Self::new_with_filter(wgpu::Backends::all(), |info| {
            info.device_type == wgpu::DeviceType::Cpu
        })
        .await
        .map_err(|_| BarracudaError::device("No CPU software rasterizer available"))
    }

    /// Create a CPU software-rasterizer device using the adapter's own supported limits.
    ///
    /// `new_cpu()` requests `science_limits()` (512 MB storage buffer binding), which
    /// llvmpipe and other software rasterizers cannot satisfy (capped at 128 MB), causing
    /// device creation to fail.  `new_cpu_relaxed()` instead asks for only
    /// `wgpu::Limits::downlevel_defaults()`, which every compliant adapter supports.
    ///
    /// Use this constructor in tests and any pipeline that runs on CPU/llvmpipe.
    pub async fn new_cpu_relaxed() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        let adapter = adapters
            .into_iter()
            .find(|a| a.get_info().device_type == wgpu::DeviceType::Cpu)
            .ok_or_else(|| BarracudaError::device("No CPU software rasterizer available"))?;

        let adapter_info = adapter.get_info();
        tracing::info!(
            "barraCuda (cpu-relaxed): {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );

        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        for feature in [
            wgpu::Features::SHADER_F64,
            wgpu::Features::SHADER_F16,
            wgpu::Features::PIPELINE_CACHE,
            wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
        ] {
            if adapter_features.contains(feature) {
                required_features |= feature;
            }
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("barraCuda cpu-relaxed"),
                    required_features,
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create CPU device: {e}")))?;

        let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::install_error_handler(&device, &lost);

        let pipeline_cache = Self::make_pipeline_cache(&device);
        let budget = super::concurrency_budget(adapter_info.device_type);
        let wgpu_device = Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
            calibration: None,
            pipeline_cache,
            lost,
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);
        Ok(wgpu_device)
    }

    /// Create device with high-capacity limits (1GB+ buffers)
    pub async fn new_high_capacity() -> Result<Self> {
        Self::new_with_limits(super::super::tensor_context::high_capacity_limits()).await
    }

    /// Create device with custom limits
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
        tracing::info!(
            "BarraCuda (high-capacity): {} ({:?})",
            info.name,
            info.device_type
        );

        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        for feature in [
            wgpu::Features::SHADER_F64,
            wgpu::Features::SHADER_F16,
            wgpu::Features::TIMESTAMP_QUERY,
            wgpu::Features::PIPELINE_CACHE,
            wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
        ] {
            if adapter_features.contains(feature) {
                required_features |= feature;
            }
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BarraCuda high-capacity device"),
                    required_features,
                    required_limits: limits,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        let actual_limits = device.limits();
        tracing::info!(
            "Limits: max_binding={}MB, max_buffer={}MB",
            actual_limits.max_storage_buffer_binding_size / (1 << 20),
            actual_limits.max_buffer_size / (1 << 20),
        );

        let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::install_error_handler(&device, &lost);

        let pipeline_cache = Self::make_pipeline_cache(&device);
        let budget = super::concurrency_budget(info.device_type);
        let wgpu_device = Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info: info,
            calibration: None,
            pipeline_cache,
            lost,
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);
        Ok(wgpu_device)
    }

    /// Create device from ToadStool hardware selection
    pub async fn from_selection(
        selection: super::super::toadstool_integration::DeviceSelection,
    ) -> Result<Self> {
        use super::super::toadstool_integration::DeviceSelection;
        match selection {
            DeviceSelection::Gpu => Self::new_gpu().await,
            DeviceSelection::Cpu => Self::new_cpu().await,
            DeviceSelection::Npu => {
                tracing::info!(
                    "NPU selected but WGSL not supported on NPU; falling back to best WGPU adapter"
                );
                Self::new().await
            }
        }
    }

    /// List all available WGPU adapters (raw, may include duplicates)
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
    pub fn enumerate_physical_devices() -> Vec<super::super::registry::PhysicalDevice> {
        super::super::registry::DeviceRegistry::global()
            .physical_devices()
            .cloned()
            .collect()
    }

    /// Get the global device registry
    pub fn registry() -> &'static super::super::registry::DeviceRegistry {
        super::super::registry::DeviceRegistry::global()
    }

    /// Create device from a physical device index (using preferred backend)
    pub async fn from_physical_device(index: usize) -> Result<Self> {
        let registry = super::super::registry::DeviceRegistry::global();
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
    pub async fn from_physical_device_with_backend(
        device_index: usize,
        backend: wgpu::Backend,
    ) -> Result<Self> {
        let registry = super::super::registry::DeviceRegistry::global();
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
                    "Backend {backend:?} not available for device {device_index} (available: {backends:?})"
                ))
            })?;
        Self::from_adapter_index(adapter_index).await
    }

    /// Create device for the first f64-capable GPU (using preferred backend)
    pub async fn new_f64_capable() -> Result<Self> {
        let registry = super::super::registry::DeviceRegistry::global();
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
    pub async fn from_env() -> Result<Self> {
        let selector = std::env::var(ADAPTER_ENV_VAR).unwrap_or_else(|_| "auto".to_string());
        Self::with_adapter_selector(&selector).await
    }

    /// Create device with explicit adapter selector
    pub async fn with_adapter_selector(selector: &str) -> Result<Self> {
        let selector = selector.trim().to_lowercase();

        if selector == "auto" || selector.is_empty() {
            tracing::info!("Adapter selection: auto (HighPerformance)");
            return Self::new().await;
        }

        let adapters = Self::enumerate_adapters();
        if adapters.is_empty() {
            return Err(BarracudaError::device("No adapters available"));
        }

        if let Ok(index) = selector.parse::<usize>() {
            if index < adapters.len() {
                tracing::info!(
                    "Adapter selection: index {} → {}",
                    index,
                    adapters[index].name
                );
                return Self::from_adapter_index(index).await;
            }
            tracing::debug!(
                "Adapter index {} out of bounds ({}), trying name match",
                index,
                adapters.len()
            );
        }

        for (index, info) in adapters.iter().enumerate() {
            if info.name.to_lowercase().contains(&selector) {
                tracing::info!(
                    "Adapter selection: '{}' → {} (index {})",
                    selector,
                    info.name,
                    index
                );
                return Self::from_adapter_index(index).await;
            }
        }

        let available: Vec<_> = adapters.iter().map(|a| a.name.as_str()).collect();
        Err(BarracudaError::device(format!(
            "No adapter matches '{selector}'. Available: {available:?}"
        )))
    }

    /// Create with specific backend (for testing/multi-GPU)
    ///
    /// Prefers `HighPerformance` adapter within the specified backends.
    pub async fn new_with_backend(backends: wgpu::Backends) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| BarracudaError::device("No WGPU adapter found for requested backend"))?;

        Self::from_adapter(adapter).await
    }

    /// Create device from a specific adapter index
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
        tracing::info!(
            "Selecting adapter {index}: {} ({:?})",
            info.name,
            info.device_type
        );

        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        for feature in [
            wgpu::Features::SHADER_F64,
            wgpu::Features::SHADER_F16,
            wgpu::Features::TIMESTAMP_QUERY,
            wgpu::Features::PIPELINE_CACHE,
            wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
        ] {
            if adapter_features.contains(feature) {
                required_features |= feature;
            }
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BarraCuda device"),
                    required_features,
                    required_limits: super::super::tensor_context::science_limits(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::install_error_handler(&device, &lost);

        let pipeline_cache = Self::make_pipeline_cache(&device);
        let budget = super::concurrency_budget(info.device_type);
        let wgpu_device = Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info: info,
            calibration: None,
            pipeline_cache,
            lost,
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);
        Ok(wgpu_device)
    }

    /// Create with custom filter (for specific GPU selection)
    pub async fn new_with_filter<F>(backends: wgpu::Backends, filter: F) -> Result<Self>
    where
        F: Fn(&wgpu::AdapterInfo) -> bool,
    {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(backends);
        if adapters.is_empty() {
            return Err(BarracudaError::device(
                "No WGPU adapters found (need GPU or CPU software rasterizer)",
            ));
        }

        let adapter = adapters
            .into_iter()
            .find(|a: &wgpu::Adapter| filter(&a.get_info()))
            .ok_or_else(|| BarracudaError::device("No adapter matching requested hardware type"))?;

        Self::from_adapter(adapter).await
    }

    /// Create device from a pre-selected adapter (shared helper)
    async fn from_adapter(adapter: wgpu::Adapter) -> Result<Self> {
        let adapter_info = adapter.get_info();
        tracing::info!(
            "barraCuda initialized: {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );

        let adapter_features = adapter.features();
        let mut required_features = wgpu::Features::empty();
        for feature in [
            wgpu::Features::SHADER_F64,
            wgpu::Features::SHADER_F16,
            wgpu::Features::TIMESTAMP_QUERY,
            wgpu::Features::PIPELINE_CACHE,
            wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
        ] {
            if adapter_features.contains(feature) {
                required_features |= feature;
            }
        }
        if required_features.contains(wgpu::Features::SHADER_F64) {
            tracing::info!("  SHADER_F64: enabled");
        }
        if required_features.contains(wgpu::Features::SPIRV_SHADER_PASSTHROUGH) {
            tracing::info!("  SPIRV_SHADER_PASSTHROUGH: enabled (sovereign compiler active)");
        }

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("barraCuda Device"),
                    required_features,
                    required_limits: super::super::tensor_context::science_limits(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        let lost = Arc::new(std::sync::atomic::AtomicBool::new(false));
        Self::install_error_handler(&device, &lost);

        let pipeline_cache = Self::make_pipeline_cache(&device);
        let budget = super::concurrency_budget(adapter_info.device_type);

        let wgpu_device = Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
            calibration: None,
            pipeline_cache,
            lost,
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);

        if wgpu_device
            .device
            .features()
            .contains(wgpu::Features::SHADER_F64)
        {
            let caps = probe::probe_f64_builtins(&wgpu_device).await;
            if !caps.can_compile_f64() {
                tracing::warn!(
                    "SHADER_F64 advertised but basic f64 probe FAILED — all f64 shaders will use DF64 fallback"
                );
            } else {
                tracing::info!("f64 probe: {}/{} builtins native", caps.native_count(), 9);
            }
        }

        Ok(wgpu_device)
    }

    /// Create WgpuDevice from existing wgpu device and queue
    pub fn from_existing(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        adapter_info: wgpu::AdapterInfo,
    ) -> Self {
        let pipeline_cache = Self::make_pipeline_cache(&device);
        let budget = super::concurrency_budget(adapter_info.device_type);
        let wgpu_device = Self {
            device,
            queue,
            adapter_info,
            calibration: None,
            pipeline_cache,
            lost: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);
        wgpu_device
    }

    /// Create WgpuDevice from existing device/queue with synthetic adapter info.
    ///
    /// # Deprecated
    ///
    /// Use [`from_existing`](Self::from_existing) instead and pass the real
    /// `AdapterInfo` from the adapter that created the device. Synthetic
    /// `AdapterInfo` with `DeviceType::Other` breaks Ada Lovelace (RTX 40xx)
    /// capability detection and f64 feature probing.
    #[deprecated(
        since = "0.3.0",
        note = "Use from_existing() with real AdapterInfo; synthetic info breaks driver detection"
    )]
    pub fn from_existing_simple(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        let pipeline_cache = Self::make_pipeline_cache(&device);
        let device_type = wgpu::DeviceType::Other;
        let budget = super::concurrency_budget(device_type);
        let wgpu_device = Self {
            device,
            queue,
            adapter_info: wgpu::AdapterInfo {
                name: "External Device".to_string(),
                vendor: 0,
                device: 0,
                device_type,
                driver: "external".to_string(),
                driver_info: "wrapped from existing wgpu resources".to_string(),
                backend: wgpu::Backend::Vulkan,
            },
            calibration: None,
            pipeline_cache,
            lost: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            gpu_lock: Arc::new(std::sync::Mutex::new(())),
            dispatch_semaphore: Arc::new(super::DispatchSemaphore::new(budget)),
        };
        probe::seed_cache_from_heuristics(&wgpu_device);
        wgpu_device
    }
}
