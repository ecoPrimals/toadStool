//! Device creation and adapter selection

use super::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;

/// Environment variable for adapter selection
pub const ADAPTER_ENV_VAR: &str = "BARRACUDA_GPU_ADAPTER";

impl WgpuDevice {
    /// Create new WebGPU device with auto-discovery
    pub async fn new() -> Result<Self> {
        Self::new_with_backend(wgpu::Backends::all()).await
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
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

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
    pub async fn from_selection(
        selection: super::super::toadstool_integration::DeviceSelection,
    ) -> Result<Self> {
        use super::super::toadstool_integration::DeviceSelection;
        match selection {
            DeviceSelection::Gpu => Self::new_gpu().await,
            DeviceSelection::Cpu => Self::new_cpu().await,
            DeviceSelection::Npu => {
                log::info!(
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
                    "Backend {:?} not available for device {} (available: {:?})",
                    backend, device_index, backends
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
            log::info!("Adapter selection: auto (HighPerformance)");
            return Self::new().await;
        }

        let adapters = Self::enumerate_adapters();
        if adapters.is_empty() {
            return Err(BarracudaError::device("No adapters available"));
        }

        if let Ok(index) = selector.parse::<usize>() {
            if index < adapters.len() {
                log::info!(
                    "Adapter selection: index {} → {}",
                    index,
                    adapters[index].name
                );
                return Self::from_adapter_index(index).await;
            }
            log::debug!(
                "Adapter index {} out of bounds ({}), trying name match",
                index,
                adapters.len()
            );
        }

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

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("BarraCUDA device"),
                    required_features,
                    required_limits: super::super::tensor_context::science_limits(),
                    memory_hints: Default::default(),
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

        let adapter_info = adapter.get_info();
        log::info!(
            "barraCUDA initialized: {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("barraCUDA Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: super::super::tensor_context::science_limits(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .map_err(|e| BarracudaError::device(format!("Failed to create device: {e}")))?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            adapter_info,
            calibration: None,
        })
    }

    /// Create WgpuDevice from existing wgpu device and queue
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
                backend: wgpu::Backend::Vulkan,
            },
            calibration: None,
        }
    }
}
