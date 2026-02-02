//! WGPU Executor - Main GPU compute coordinator
//!
//! Pure Rust GPU executor using WebGPU standard.
//! Zero FFI, zero unsafe code - modern idiomatic Rust!
//!
//! This is the main coordinator that delegates to specialized operation modules.
//! Deep Debt: Runtime capability discovery, no hardcoded GPU requirements.

use anyhow::{Context, Result};

/// Pure Rust GPU executor using wgpu (WebGPU)
///
/// No FFI, no unsafe code - just modern idiomatic Rust!
///
/// Design Philosophy:
/// - Thin coordinator layer
/// - Delegates to specialized operation modules
/// - Runtime GPU capability discovery
/// - Zero hardcoded GPU requirements
pub struct WgpuExecutor {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) adapter_info: wgpu::AdapterInfo,
}

impl WgpuExecutor {
    /// Create a new wgpu executor with runtime GPU discovery
    ///
    /// This is pure Rust - no FFI, no unsafe!
    ///
    /// Deep Debt Principles:
    /// - Discovers available GPU at runtime
    /// - No hardcoded GPU vendor/model requirements
    /// - Self-knowledge: knows own capabilities only
    pub async fn new() -> Result<Self> {
        Self::new_with_backend(wgpu::Backends::all()).await
    }

    /// Create executor with specific backend (for testing/capability-based selection)
    ///
    /// Allows runtime selection of GPU backend based on discovered capabilities.
    pub async fn new_with_backend(backends: wgpu::Backends) -> Result<Self> {
        Self::new_with_backend_and_filter(backends, |_| true).await
    }

    /// Create executor selecting a specific GPU by filter
    ///
    /// Allows precise GPU selection for multi-GPU systems and research.
    pub async fn new_with_backend_and_filter<F>(
        backends: wgpu::Backends,
        adapter_filter: F,
    ) -> Result<Self>
    where
        F: Fn(&wgpu::AdapterInfo) -> bool,
    {
        // Create instance (pure Rust, runtime discovery)
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        // Enumerate all adapters (returns Vec already)
        let adapters = instance.enumerate_adapters(backends);

        // Find matching adapter
        let adapter = adapters
            .into_iter()
            .find(|adapter: &wgpu::Adapter| adapter_filter(&adapter.get_info()))
            .context("No GPU matching filter found")?;

        let adapter_info = adapter.get_info();

        // Request device (runtime capability negotiation)
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ToadStool GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .context("Failed to create GPU device from discovered adapter")?;

        Ok(Self {
            device,
            queue,
            adapter_info,
        })
    }

    /// Create executor with AMD GPU (for research/testing)
    pub async fn new_amd() -> Result<Self> {
        Self::new_with_backend_and_filter(wgpu::Backends::all(), |info| {
            info.vendor == 0x1002 || // AMD vendor ID
            info.name.to_lowercase().contains("amd") || 
            info.name.to_lowercase().contains("radeon")
        })
        .await
        .context("No AMD GPU found")
    }

    /// Create executor with NVIDIA GPU (for research/testing)
    pub async fn new_nvidia() -> Result<Self> {
        Self::new_with_backend_and_filter(wgpu::Backends::all(), |info| {
            info.vendor == 0x10DE || // NVIDIA vendor ID
            info.name.to_lowercase().contains("nvidia") ||
            info.name.to_lowercase().contains("geforce")
        })
        .await
        .context("No NVIDIA GPU found")
    }

    /// Create executor with Intel GPU (for research/testing)
    pub async fn new_intel() -> Result<Self> {
        Self::new_with_backend_and_filter(wgpu::Backends::all(), |info| {
            info.vendor == 0x8086 || // Intel vendor ID
            info.name.to_lowercase().contains("intel") ||
            info.name.to_lowercase().contains("iris")
        })
        .await
        .context("No Intel GPU found")
    }

    /// Create executor with Apple GPU (for research/testing)
    pub async fn new_apple() -> Result<Self> {
        Self::new_with_backend_and_filter(wgpu::Backends::all(), |info| {
            info.name.to_lowercase().contains("apple")
                || info.name.to_lowercase().contains("m1")
                || info.name.to_lowercase().contains("m2")
                || info.name.to_lowercase().contains("m3")
        })
        .await
        .context("No Apple GPU found")
    }

    /// List all available GPUs (for research/multi-GPU systems)
    pub async fn list_gpus() -> Vec<String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());

        adapters
            .iter()
            .map(|adapter: &wgpu::Adapter| {
                let info = adapter.get_info();
                format!("{} {} ({:?})", info.vendor, info.name, info.backend)
            })
            .collect()
    }

    /// Get GPU information (runtime-discovered capabilities)
    ///
    /// Returns self-knowledge about discovered GPU, not hardcoded info.
    pub fn gpu_info(&self) -> String {
        format!(
            "{} {} ({})",
            self.adapter_info.vendor,
            self.adapter_info.name,
            self.adapter_info.backend.to_str()
        )
    }

    /// Get detailed GPU capabilities (Deep Debt: self-knowledge only)
    pub fn capabilities(&self) -> GpuCapabilities {
        GpuCapabilities {
            vendor: self.adapter_info.vendor as usize,
            name: self.adapter_info.name.clone(),
            backend: self.adapter_info.backend.to_str().to_string(),
            device_type: format!("{:?}", self.adapter_info.device_type),
        }
    }
}

/// GPU capabilities discovered at runtime
///
/// Deep Debt: This is self-knowledge, discovered at runtime, never hardcoded.
#[derive(Debug, Clone)]
pub struct GpuCapabilities {
    pub vendor: usize,
    pub name: String,
    pub backend: String,
    pub device_type: String,
}
