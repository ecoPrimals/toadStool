//! Processing Substrate Abstraction
//!
//! Modern, idiomatic, async abstraction for all processing substrates:
//! - GPU (AMD, NVIDIA, Intel, Apple)
//! - CPU (native, SIMD, multithreaded)
//! - Neuromorphic (future)
//! - Custom accelerators (future)
//!
//! Design Principles:
//! - Explicit selection (no environment variables!)
//! - Runtime discovery
//! - Async throughout
//! - Fully concurrent
//! - Granular control for validation
//! - Zero deep debt

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

pub mod selector;
pub use selector::*;

/// Processing substrate - where computation happens
///
/// This abstraction allows granular control over execution targets
/// for validation, benchmarking, and optimization.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProcessingSubstrate {
    /// GPU compute (WGPU/Vulkan/Metal/DX12)
    Gpu(GpuTarget),

    /// CPU compute (native, SIMD, multithreaded)
    Cpu(CpuTarget),

    /// Neuromorphic compute (future)
    #[serde(skip)]
    Neuromorphic(NeuromorphicTarget),

    /// Custom accelerator (TPU, NPU, etc.)
    #[serde(skip)]
    Custom(String),
}

impl ProcessingSubstrate {
    /// Get human-readable name
    pub fn name(&self) -> String {
        match self {
            Self::Gpu(target) => format!("GPU:{}", target.name()),
            Self::Cpu(target) => format!("CPU:{}", target.name()),
            Self::Neuromorphic(target) => format!("Neuromorphic:{}", target.name()),
            Self::Custom(name) => format!("Custom:{name}"),
        }
    }

    /// Check if this substrate is available on the current system
    pub async fn is_available(&self) -> bool {
        match self {
            Self::Gpu(target) => target.is_available().await,
            Self::Cpu(target) => target.is_available(),
            Self::Neuromorphic(_) => false, // Future
            Self::Custom(_) => false,       // Future
        }
    }

    /// Get detailed capabilities
    pub async fn capabilities(&self) -> Result<SubstrateCapabilities> {
        match self {
            Self::Gpu(target) => target.capabilities().await,
            Self::Cpu(target) => Ok(target.capabilities()),
            Self::Neuromorphic(_) => anyhow::bail!("Neuromorphic not yet implemented"),
            Self::Custom(name) => {
                anyhow::bail!("Custom accelerator '{name}' not yet implemented")
            }
        }
    }
}

impl fmt::Display for ProcessingSubstrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// GPU compute target - explicit, granular control
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GpuTarget {
    /// Vendor preference (None = any available)
    pub vendor: Option<GpuVendor>,

    /// Device index (None = first matching)
    pub device_index: Option<usize>,

    /// Backend preference (Vulkan, Metal, DX12, etc.)
    pub backend: GpuBackend,

    /// Power preference (high performance vs low power)
    pub power_preference: PowerPreference,
}

impl GpuTarget {
    /// Default: Any available GPU, high performance
    pub fn any() -> Self {
        Self {
            vendor: None,
            device_index: None,
            backend: GpuBackend::Auto,
            power_preference: PowerPreference::HighPerformance,
        }
    }

    /// Target AMD GPU
    pub fn amd() -> Self {
        Self {
            vendor: Some(GpuVendor::Amd),
            ..Self::any()
        }
    }

    /// Target NVIDIA GPU
    pub fn nvidia() -> Self {
        Self {
            vendor: Some(GpuVendor::Nvidia),
            ..Self::any()
        }
    }

    /// Target Intel GPU
    pub fn intel() -> Self {
        Self {
            vendor: Some(GpuVendor::Intel),
            ..Self::any()
        }
    }

    /// Target Apple GPU
    pub fn apple() -> Self {
        Self {
            vendor: Some(GpuVendor::Apple),
            ..Self::any()
        }
    }

    /// Target specific device by index
    pub fn device(mut self, index: usize) -> Self {
        self.device_index = Some(index);
        self
    }

    /// Use specific backend
    pub fn with_backend(mut self, backend: GpuBackend) -> Self {
        self.backend = backend;
        self
    }

    /// Use low power mode
    pub fn low_power(mut self) -> Self {
        self.power_preference = PowerPreference::LowPower;
        self
    }

    fn name(&self) -> String {
        let vendor = self
            .vendor
            .as_ref()
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "Any".to_string());
        let device = self
            .device_index
            .map(|i| format!("#{i}"))
            .unwrap_or_default();
        format!("{vendor}{device}")
    }

    async fn is_available(&self) -> bool {
        // Check if matching GPU is available
        match self.enumerate_matching().await {
            Ok(devices) => !devices.is_empty(),
            Err(_) => false,
        }
    }

    async fn capabilities(&self) -> Result<SubstrateCapabilities> {
        let devices = self.enumerate_matching().await?;
        let device = devices.first().context("No matching GPU found")?;

        Ok(SubstrateCapabilities {
            name: device.name.clone(),
            compute_capability: format!("{:?}", device.device_type),
            memory_bytes: None, // WGPU doesn't expose this
            backend: format!("{:?}", device.backend),
            features: vec![], // TODO: Parse wgpu features
        })
    }

    async fn enumerate_matching(&self) -> Result<Vec<wgpu::AdapterInfo>> {
        let backends = self.backend.to_wgpu_backends();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(backends);
        let mut matching: Vec<wgpu::AdapterInfo> = adapters
            .iter()
            .map(|a| a.get_info())
            .filter(|info| self.matches_adapter(info))
            .collect();

        // Sort by preference (discrete GPUs first)
        matching.sort_by_key(|info| match info.device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            wgpu::DeviceType::VirtualGpu => 2,
            wgpu::DeviceType::Cpu => 3,
            wgpu::DeviceType::Other => 4,
        });

        Ok(matching)
    }

    fn matches_adapter(&self, info: &wgpu::AdapterInfo) -> bool {
        // Check vendor if specified
        if let Some(vendor) = &self.vendor {
            if !vendor.matches(info) {
                return false;
            }
        }

        // Check backend if specified
        if !self.backend.matches(info.backend) {
            return false;
        }

        true
    }
}

/// GPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Qualcomm,
    Arm,
}

impl GpuVendor {
    /// Check if adapter matches this vendor
    fn matches(&self, info: &wgpu::AdapterInfo) -> bool {
        let name_lower = info.name.to_lowercase();
        match self {
            Self::Nvidia => {
                info.vendor == 0x10DE || // NVIDIA vendor ID
                name_lower.contains("nvidia") || 
                name_lower.contains("geforce") ||
                name_lower.contains("quadro")
            }
            Self::Amd => {
                info.vendor == 0x1002 || // AMD vendor ID
                name_lower.contains("amd") || 
                name_lower.contains("radeon") ||
                name_lower.contains("rx ")
            }
            Self::Intel => {
                info.vendor == 0x8086 || // Intel vendor ID
                name_lower.contains("intel") ||
                name_lower.contains("iris")
            }
            Self::Apple => {
                name_lower.contains("apple")
                    || name_lower.contains("m1")
                    || name_lower.contains("m2")
                    || name_lower.contains("m3")
            }
            Self::Qualcomm => name_lower.contains("qualcomm") || name_lower.contains("adreno"),
            Self::Arm => name_lower.contains("mali") || name_lower.contains("arm"),
        }
    }
}

/// GPU backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuBackend {
    Auto,   // Let WGPU choose
    Vulkan, // Cross-platform, modern
    Metal,  // Apple
    Dx12,   // Windows
    Gl,     // Legacy OpenGL
}

impl GpuBackend {
    fn to_wgpu_backends(&self) -> wgpu::Backends {
        match self {
            Self::Auto => wgpu::Backends::all(),
            Self::Vulkan => wgpu::Backends::VULKAN,
            Self::Metal => wgpu::Backends::METAL,
            Self::Dx12 => wgpu::Backends::DX12,
            Self::Gl => wgpu::Backends::GL,
        }
    }

    fn matches(&self, backend: wgpu::Backend) -> bool {
        match self {
            Self::Auto => true,
            Self::Vulkan => backend == wgpu::Backend::Vulkan,
            Self::Metal => backend == wgpu::Backend::Metal,
            Self::Dx12 => backend == wgpu::Backend::Dx12,
            Self::Gl => backend == wgpu::Backend::Gl,
        }
    }
}

/// Power preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PowerPreference {
    HighPerformance,
    LowPower,
}

/// CPU compute target
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CpuTarget {
    /// Number of threads (None = all cores)
    pub threads: Option<usize>,

    /// SIMD level (SSE, AVX, AVX512, NEON)
    pub simd: SimdLevel,
}

impl CpuTarget {
    pub fn auto() -> Self {
        Self {
            threads: None,
            simd: SimdLevel::Auto,
        }
    }

    pub fn threads(mut self, n: usize) -> Self {
        self.threads = Some(n);
        self
    }

    fn name(&self) -> String {
        let threads = self
            .threads
            .map(|t| format!("{t}t"))
            .unwrap_or_else(|| "all".to_string());
        format!("{}:{:?}", threads, self.simd)
    }

    fn is_available(&self) -> bool {
        true // CPU always available
    }

    fn capabilities(&self) -> SubstrateCapabilities {
        SubstrateCapabilities {
            name: "CPU".to_string(),
            compute_capability: format!("{:?}", self.simd),
            memory_bytes: None,
            backend: "Native".to_string(),
            features: vec![],
        }
    }
}

/// SIMD instruction set
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SimdLevel {
    Auto,   // Detect at runtime
    Scalar, // No SIMD
    Sse,    // x86 SSE
    Avx,    // x86 AVX
    Avx512, // x86 AVX-512
    Neon,   // ARM NEON
}

/// Neuromorphic target (future)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NeuromorphicTarget {
    pub device: String,
}

impl NeuromorphicTarget {
    fn name(&self) -> String {
        self.device.clone()
    }
}

/// Substrate capabilities discovered at runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    pub name: String,
    pub compute_capability: String,
    pub memory_bytes: Option<usize>,
    pub backend: String,
    pub features: Vec<String>,
}

impl fmt::Display for SubstrateCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {})",
            self.name, self.compute_capability, self.backend
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gpu_enumeration() {
        let target = GpuTarget::any();
        let available = target.is_available().await;
        println!("GPU available: {}", available);

        if available {
            let caps = target.capabilities().await.unwrap();
            println!("GPU capabilities: {}", caps);
        }
    }

    #[tokio::test]
    async fn test_vendor_specific() {
        // Test NVIDIA
        let nvidia = GpuTarget::nvidia();
        if nvidia.is_available().await {
            println!("NVIDIA GPU found");
            let caps = nvidia.capabilities().await.unwrap();
            println!("  {}", caps);
        }

        // Test AMD
        let amd = GpuTarget::amd();
        if amd.is_available().await {
            println!("AMD GPU found");
            let caps = amd.capabilities().await.unwrap();
            println!("  {}", caps);
        }
    }

    #[test]
    fn test_cpu_always_available() {
        let cpu = CpuTarget::auto();
        assert!(cpu.is_available());
    }
}
