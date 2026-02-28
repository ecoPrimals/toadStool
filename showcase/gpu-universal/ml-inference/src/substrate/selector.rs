//! Processing Substrate Selector
//!
//! Modern, async selector for choosing processing substrates.
//! Replaces brittle environment variable approach with explicit API.

use super::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Substrate selector - choose where computation happens
///
/// This is the main entry point for selecting processing substrates.
/// It provides:
/// - Async discovery
/// - Caching of available devices
/// - Validation
/// - Explicit control (no environment variables!)
pub struct SubstrateSelector {
    /// Cache of discovered devices
    cache: Arc<RwLock<Option<DiscoveredDevices>>>,
}

#[derive(Debug, Clone)]
struct DiscoveredDevices {
    gpus: Vec<(usize, wgpu::AdapterInfo)>,
    timestamp: std::time::Instant,
}

impl SubstrateSelector {
    /// Create a new selector
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(None)),
        }
    }

    /// Discover all available processing substrates
    ///
    /// Returns a list of all GPUs and processing options available on this system.
    pub async fn discover_all(&self) -> Result<Vec<ProcessingSubstrate>> {
        let mut substrates = Vec::new();

        // Always have CPU
        substrates.push(ProcessingSubstrate::Cpu(CpuTarget::auto()));

        // Discover GPUs
        let gpus = self.discover_gpus().await?;
        for (idx, info) in &gpus {
            let vendor = Self::detect_vendor(info);
            substrates.push(ProcessingSubstrate::Gpu(GpuTarget {
                vendor: Some(vendor),
                device_index: Some(*idx),
                backend: Self::detect_backend(info.backend),
                power_preference: PowerPreference::HighPerformance,
            }));
        }

        Ok(substrates)
    }

    /// Select substrate based on criteria
    ///
    /// This is the robust, explicit way to select a processing substrate.
    /// No environment variables, no magic!
    pub async fn select(&self, target: ProcessingSubstrate) -> Result<ProcessingSubstrate> {
        // Validate that target is available
        if !target.is_available().await {
            anyhow::bail!("Selected substrate is not available: {target}");
        }

        Ok(target)
    }

    /// Select GPU by vendor (first matching)
    pub async fn select_gpu_by_vendor(&self, vendor: GpuVendor) -> Result<ProcessingSubstrate> {
        let gpus = self.discover_gpus().await?;

        for (_idx, info) in &gpus {
            if vendor.matches(info) {
                return Ok(ProcessingSubstrate::Gpu(GpuTarget {
                    vendor: Some(vendor),
                    device_index: None,
                    backend: Self::detect_backend(info.backend),
                    power_preference: PowerPreference::HighPerformance,
                }));
            }
        }

        anyhow::bail!("No {vendor:?} GPU found")
    }

    /// Select GPU by index
    pub async fn select_gpu_by_index(&self, index: usize) -> Result<ProcessingSubstrate> {
        let gpus = self.discover_gpus().await?;

        if index >= gpus.len() {
            anyhow::bail!(
                "GPU index {} out of range (found {} GPUs)",
                index,
                gpus.len()
            );
        }

        let (_, info) = &gpus[index];
        let vendor = Self::detect_vendor(info);

        Ok(ProcessingSubstrate::Gpu(GpuTarget {
            vendor: Some(vendor),
            device_index: Some(index),
            backend: Self::detect_backend(info.backend),
            power_preference: PowerPreference::HighPerformance,
        }))
    }

    /// Get default substrate (best available GPU, or CPU)
    pub async fn default_substrate(&self) -> Result<ProcessingSubstrate> {
        let gpus = self.discover_gpus().await?;

        if let Some((idx, info)) = gpus.first() {
            let vendor = Self::detect_vendor(info);
            return Ok(ProcessingSubstrate::Gpu(GpuTarget {
                vendor: Some(vendor),
                device_index: Some(*idx),
                backend: Self::detect_backend(info.backend),
                power_preference: PowerPreference::HighPerformance,
            }));
        }

        // Fallback to CPU
        Ok(ProcessingSubstrate::Cpu(CpuTarget::auto()))
    }

    /// Discover all GPUs (with caching)
    async fn discover_gpus(&self) -> Result<Vec<(usize, wgpu::AdapterInfo)>> {
        // Check cache
        {
            let cache = self.cache.read().await;
            if let Some(ref devices) = *cache {
                // Cache valid for 60 seconds
                if devices.timestamp.elapsed().as_secs() < 60 {
                    return Ok(devices.gpus.clone());
                }
            }
        }

        // Discover fresh
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        let mut gpus: Vec<(usize, wgpu::AdapterInfo)> = adapters
            .iter()
            .enumerate()
            .map(|(idx, adapter)| (idx, adapter.get_info()))
            .collect();

        // Sort by preference (discrete GPUs first)
        gpus.sort_by_key(|(_, info)| match info.device_type {
            wgpu::DeviceType::DiscreteGpu => 0,
            wgpu::DeviceType::IntegratedGpu => 1,
            wgpu::DeviceType::VirtualGpu => 2,
            wgpu::DeviceType::Cpu => 3,
            wgpu::DeviceType::Other => 4,
        });

        // Update cache
        {
            let mut cache = self.cache.write().await;
            *cache = Some(DiscoveredDevices {
                gpus: gpus.clone(),
                timestamp: std::time::Instant::now(),
            });
        }

        Ok(gpus)
    }

    fn detect_vendor(info: &wgpu::AdapterInfo) -> GpuVendor {
        for vendor in [
            GpuVendor::Nvidia,
            GpuVendor::Amd,
            GpuVendor::Intel,
            GpuVendor::Apple,
            GpuVendor::Qualcomm,
            GpuVendor::Arm,
        ] {
            if vendor.matches(info) {
                return vendor;
            }
        }

        // Fallback based on vendor ID
        match info.vendor {
            0x10DE => GpuVendor::Nvidia,
            0x1002 => GpuVendor::Amd,
            0x8086 => GpuVendor::Intel,
            _ => GpuVendor::Nvidia, // Default
        }
    }

    fn detect_backend(backend: wgpu::Backend) -> GpuBackend {
        match backend {
            wgpu::Backend::Vulkan => GpuBackend::Vulkan,
            wgpu::Backend::Metal => GpuBackend::Metal,
            wgpu::Backend::Dx12 => GpuBackend::Dx12,
            wgpu::Backend::Gl => GpuBackend::Gl,
            _ => GpuBackend::Auto,
        }
    }

    /// List all available devices (for debugging/selection)
    pub async fn list_devices(&self) -> Result<Vec<String>> {
        let gpus = self.discover_gpus().await?;

        let mut devices = vec!["CPU (native, all cores)".to_string()];

        for (idx, info) in &gpus {
            let vendor = Self::detect_vendor(info);
            devices.push(format!(
                "[{}] {:?} {} ({:?}, {:?})",
                idx, vendor, info.name, info.backend, info.device_type
            ));
        }

        Ok(devices)
    }
}

impl Default for SubstrateSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_all() {
        let selector = SubstrateSelector::new();
        let devices = selector.discover_all().await.unwrap();

        println!("Discovered {} devices:", devices.len());
        for device in &devices {
            println!("  {}", device);
        }

        assert!(!devices.is_empty(), "Should find at least CPU");
    }

    #[tokio::test]
    async fn test_list_devices() {
        let selector = SubstrateSelector::new();
        let devices = selector.list_devices().await.unwrap();

        println!("Available devices:");
        for device in &devices {
            println!("  {}", device);
        }

        assert!(!devices.is_empty());
    }

    #[tokio::test]
    async fn test_default_substrate() {
        let selector = SubstrateSelector::new();
        let substrate = selector.default_substrate().await.unwrap();

        println!("Default substrate: {}", substrate);

        assert!(substrate.is_available().await);
    }

    #[tokio::test]
    async fn test_caching() {
        let selector = SubstrateSelector::new();

        let start = std::time::Instant::now();
        let _devices1 = selector.discover_gpus().await.unwrap();
        let first_duration = start.elapsed();

        let start = std::time::Instant::now();
        let _devices2 = selector.discover_gpus().await.unwrap();
        let second_duration = start.elapsed();

        println!("First discovery: {:?}", first_duration);
        println!("Cached discovery: {:?}", second_duration);

        // Second should be much faster (cached)
        assert!(second_duration < first_duration / 10);
    }
}
