// SPDX-License-Identifier: AGPL-3.0-or-later

//! Platform-agnostic GPU resource handle.
//!
//! [`PortableResourceHandle`] implements [`ResourceHandle`] without relying
//! on VFIO file descriptors or any other OS-specific resource. It tracks
//! logical device ownership via an opaque identifier and atomic liveness
//! flag — suitable for Vulkan/Metal/DX12/WebGPU backends where the GPU
//! runtime manages the actual device handle internally.
//!
//! Phase 2 Silicon Atheism: abstraction over gating.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::resource_handle::ResourceHandle;

/// Error type for portable handle operations.
#[derive(Debug, thiserror::Error)]
pub enum PortableHandleError {
    /// The handle has already been released.
    #[error("handle already released")]
    AlreadyReleased,

    /// Reacquisition is not supported for this handle type.
    #[error("reacquire not supported: {0}")]
    ReacquireUnsupported(String),
}

/// GPU backend type for the portable handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuBackend {
    /// Vulkan (via ash or wgpu)
    Vulkan,
    /// Apple Metal (via wgpu)
    Metal,
    /// DirectX 12 (via wgpu)
    Dx12,
    /// WebGPU / wgpu native
    WebGpu,
    /// Software rasterizer / CPU fallback
    Software,
}

impl fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vulkan => write!(f, "vulkan"),
            Self::Metal => write!(f, "metal"),
            Self::Dx12 => write!(f, "dx12"),
            Self::WebGpu => write!(f, "webgpu"),
            Self::Software => write!(f, "software"),
        }
    }
}

/// Platform-agnostic GPU resource handle.
///
/// Tracks logical ownership of a GPU device without OS-specific file
/// descriptors. The actual device lifecycle (VkDevice creation/destruction,
/// wgpu adapter selection) is managed by the compute runtime; this handle
/// is the ember-side liveness token.
pub struct PortableResourceHandle {
    device_key: String,
    backend: GpuBackend,
    alive: AtomicBool,
}

impl PortableResourceHandle {
    /// Create a new portable handle for a device.
    ///
    /// `device_key` should match the `DeviceId::Platform` string used by
    /// discovery (e.g. `"wgpu:Vulkan:0x10de:0x1b80:GTX 1080"`).
    #[must_use]
    pub fn new(device_key: String, backend: GpuBackend) -> Self {
        Self {
            device_key,
            backend,
            alive: AtomicBool::new(true),
        }
    }

    /// The opaque device key (matches `DeviceId::Platform` from discovery).
    #[must_use]
    pub fn device_key(&self) -> &str {
        &self.device_key
    }

    /// Which GPU backend this handle represents.
    #[must_use]
    pub fn backend(&self) -> GpuBackend {
        self.backend
    }
}

impl fmt::Debug for PortableResourceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PortableResourceHandle")
            .field("device_key", &self.device_key)
            .field("backend", &self.backend)
            .field("alive", &self.alive.load(Ordering::Relaxed))
            .finish()
    }
}

impl ResourceHandle for PortableResourceHandle {
    type Error = PortableHandleError;

    fn handle_type(&self) -> &str {
        match self.backend {
            GpuBackend::Vulkan => "vulkan",
            GpuBackend::Metal => "metal",
            GpuBackend::Dx12 => "dx12",
            GpuBackend::WebGpu => "webgpu",
            GpuBackend::Software => "software",
        }
    }

    fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Acquire)
    }

    fn release(&mut self) -> Result<(), Self::Error> {
        self.alive.store(false, Ordering::Release);
        Ok(())
    }

    fn reacquire(&mut self) -> Result<bool, Self::Error> {
        if self.alive.load(Ordering::Acquire) {
            return Ok(true);
        }
        self.alive.store(true, Ordering::Release);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handle() -> PortableResourceHandle {
        PortableResourceHandle::new(
            "wgpu:Vulkan:0x10de:0x1b80:GTX 1080".into(),
            GpuBackend::Vulkan,
        )
    }

    #[test]
    fn new_handle_is_alive() {
        let h = test_handle();
        assert!(h.is_alive());
        assert_eq!(h.handle_type(), "vulkan");
    }

    #[test]
    fn release_marks_dead() {
        let mut h = test_handle();
        h.release().unwrap();
        assert!(!h.is_alive());
    }

    #[test]
    fn release_is_idempotent() {
        let mut h = test_handle();
        h.release().unwrap();
        h.release().unwrap();
        assert!(!h.is_alive());
    }

    #[test]
    fn reacquire_after_release() {
        let mut h = test_handle();
        h.release().unwrap();
        assert!(!h.is_alive());

        let ok = h.reacquire().unwrap();
        assert!(ok);
        assert!(h.is_alive());
    }

    #[test]
    fn reacquire_while_alive_is_noop() {
        let mut h = test_handle();
        let ok = h.reacquire().unwrap();
        assert!(ok);
        assert!(h.is_alive());
    }

    #[test]
    fn device_key_matches_discovery() {
        let h = test_handle();
        assert_eq!(h.device_key(), "wgpu:Vulkan:0x10de:0x1b80:GTX 1080");
    }

    #[test]
    fn backend_accessor() {
        let h = test_handle();
        assert_eq!(h.backend(), GpuBackend::Vulkan);
    }

    #[test]
    fn all_backend_handle_types() {
        for (backend, expected) in [
            (GpuBackend::Vulkan, "vulkan"),
            (GpuBackend::Metal, "metal"),
            (GpuBackend::Dx12, "dx12"),
            (GpuBackend::WebGpu, "webgpu"),
            (GpuBackend::Software, "software"),
        ] {
            let h = PortableResourceHandle::new("test".into(), backend);
            assert_eq!(h.handle_type(), expected);
        }
    }

    #[test]
    fn debug_format() {
        let h = test_handle();
        let dbg = format!("{h:?}");
        assert!(dbg.contains("PortableResourceHandle"));
        assert!(dbg.contains("Vulkan"));
        assert!(dbg.contains("alive: true"));
    }

    #[test]
    fn gpu_backend_display() {
        assert_eq!(GpuBackend::Vulkan.to_string(), "vulkan");
        assert_eq!(GpuBackend::Metal.to_string(), "metal");
        assert_eq!(GpuBackend::Dx12.to_string(), "dx12");
        assert_eq!(GpuBackend::WebGpu.to_string(), "webgpu");
        assert_eq!(GpuBackend::Software.to_string(), "software");
    }

    #[test]
    fn gpu_backend_serde_roundtrip() {
        let json = serde_json::to_string(&GpuBackend::Vulkan).unwrap();
        let back: GpuBackend = serde_json::from_str(&json).unwrap();
        assert_eq!(back, GpuBackend::Vulkan);
    }

    #[test]
    fn held_resource_integration() {
        use crate::HeldResource;

        let h = test_handle();
        let held = HeldResource::new(h);
        assert!(held.handle().is_alive());
        assert_eq!(held.handle().handle_type(), "vulkan");
    }
}
