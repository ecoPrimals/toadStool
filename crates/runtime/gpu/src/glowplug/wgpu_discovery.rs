// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cross-platform GPU discovery via wgpu adapter enumeration.
//!
//! Unlike [`GpuDiscovery`](super::GpuDiscovery) which scans Linux sysfs,
//! this backend uses wgpu's adapter enumeration — which wraps Vulkan,
//! Metal, DX12, and WebGPU — to discover GPUs on any platform.
//!
//! Phase 2 Silicon Atheism: abstraction over gating.

use toadstool_glowplug::device_id::DeviceId;
use toadstool_glowplug::discovery::DeviceDiscovery;

/// Cross-platform GPU discovery via wgpu (Vulkan/Metal/DX12/WebGPU).
///
/// Each discovered adapter is identified by `DeviceId::Platform` using
/// a stable key of `"wgpu:<backend>:<vendor_id>:<device_id>:<name>"`.
#[derive(Debug, Default)]
pub struct WgpuGpuDiscovery {
    backends: wgpu::Backends,
}

impl WgpuGpuDiscovery {
    /// Create a discovery instance that probes all available backends.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            backends: wgpu::Backends::all(),
        }
    }

    /// Create a discovery instance limited to specific backends.
    #[must_use]
    pub const fn with_backends(backends: wgpu::Backends) -> Self {
        Self { backends }
    }

    fn adapter_to_device_id(info: &wgpu::AdapterInfo) -> DeviceId {
        DeviceId::Platform(format!(
            "wgpu:{:?}:{:#06x}:{:#06x}:{}",
            info.backend, info.vendor, info.device, info.name,
        ))
    }
}

impl DeviceDiscovery for WgpuGpuDiscovery {
    type Error = std::io::Error;

    fn hardware_class(&self) -> &str {
        "gpu"
    }

    async fn discover(&self) -> Result<Vec<DeviceId>, Self::Error> {
        if !crate::vulkan_loader_available() {
            return Ok(Vec::new());
        }

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: self.backends,
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(self.backends).await;

        let mut devices: Vec<DeviceId> = adapters
            .iter()
            .map(|a| Self::adapter_to_device_id(&a.get_info()))
            .collect();
        devices.sort_by_key(std::string::ToString::to_string);
        Ok(devices)
    }

    async fn is_present(&self, id: &DeviceId) -> Result<bool, Self::Error> {
        let DeviceId::Platform(key) = id else {
            return Ok(false);
        };
        if !key.starts_with("wgpu:") {
            return Ok(false);
        }
        let all = self.discover().await?;
        Ok(all.iter().any(|d| d == id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_to_device_id_format() {
        let info = wgpu::AdapterInfo {
            name: "Test GPU".into(),
            vendor: 0x10DE,
            device: 0x1B80,
            device_type: wgpu::DeviceType::DiscreteGpu,
            device_pci_bus_id: String::new(),
            driver: String::new(),
            driver_info: String::new(),
            backend: wgpu::Backend::Vulkan,
            subgroup_min_size: 32,
            subgroup_max_size: 32,
            transient_saves_memory: false,
        };
        let id = WgpuGpuDiscovery::adapter_to_device_id(&info);
        match &id {
            DeviceId::Platform(key) => {
                assert!(key.starts_with("wgpu:Vulkan:"));
                assert!(key.contains("0x10de"));
                assert!(key.contains("0x1b80"));
                assert!(key.ends_with("Test GPU"));
            }
            other => panic!("expected Platform, got {other:?}"),
        }
    }

    #[test]
    fn short_label_for_wgpu_id() {
        let id = DeviceId::Platform("wgpu:Vulkan:0x10de:0x1b80:GTX 1080".into());
        let label = id.short_label();
        assert!(label.starts_with("platform:"));
    }

    #[tokio::test]
    async fn discover_does_not_panic() {
        let disc = WgpuGpuDiscovery::new();
        let _result = disc.discover().await;
        // Ok with GPU, Err without backend — either is acceptable
    }

    #[tokio::test]
    async fn nonexistent_device_not_present() {
        let disc = WgpuGpuDiscovery::new();
        let id = DeviceId::Platform("wgpu:Vulkan:0xffff:0xffff:Nonexistent".into());
        let result = disc.is_present(&id).await;
        // Without a wgpu backend, discover returns Err which propagates here
        match result {
            Ok(present) => assert!(!present),
            Err(_) => {} // no backend available — acceptable
        }
    }

    #[tokio::test]
    async fn pci_id_returns_false() {
        let disc = WgpuGpuDiscovery::new();
        let id = DeviceId::PciBdf("0000:01:00.0".into());
        assert!(!disc.is_present(&id).await.unwrap());
    }

    #[test]
    fn default_uses_all_backends() {
        let disc = WgpuGpuDiscovery::default();
        assert_eq!(disc.backends, wgpu::Backends::all());
    }

    #[test]
    fn with_backends_limits_scope() {
        let disc = WgpuGpuDiscovery::with_backends(wgpu::Backends::VULKAN);
        assert_eq!(disc.backends, wgpu::Backends::VULKAN);
    }
}
