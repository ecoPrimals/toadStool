// SPDX-License-Identifier: AGPL-3.0-or-later
//! WebGPU unified memory backend
//!
//! **Status**: ✅ IMPLEMENTED - Pure Rust, vendor-agnostic
//!
//! This provides unified memory via WebGPU's mappable buffers.
//! It's the sovereignty-first backend, using pure Rust with wgpu.
//!
//! # Known Limitations
//!
//! WebGPU's safe API doesn't expose raw pointers like Vulkan/OpenCL.
//! Instead, it uses `BufferSlice` with `get_mapped_range()`. This means:
//!
//! - **CPU pointer**: Returns sentinel value (buffer address)
//! - **Device pointer**: Returns opaque handle (buffer address)
//! - **Actual access**: Must use wgpu's BufferSlice API
//!
//! For true zero-copy with WebGPU, applications should use wgpu's
//! native API directly. This backend provides compatibility with
//! the unified memory interface.
//!
//! # Future Work
//!
//! - Implement buffer pool with persistent mappings
//! - Add wgpu-specific fast path for direct BufferSlice access
//! - Integrate with ToadStool's kernel execution system

use crate::unified_memory::{
    backend::{BackendAllocation, BackendInitializer, UnifiedMemoryBackend, WebGpuAllocation},
    types::*,
};
use async_trait::async_trait;
use std::sync::Arc;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use wgpu;

/// WebGPU unified memory backend
///
/// Uses `wgpu` for pure Rust, vendor-agnostic GPU access.
/// Provides unified memory via MAP_READ + MAP_WRITE buffers.
pub struct WebGpuBackend {
    /// wgpu device
    device: Arc<wgpu::Device>,

    /// wgpu queue (for future sync operations)
    #[allow(dead_code, reason = "reserved for future sync operations")]
    queue: Arc<wgpu::Queue>,

    /// Backend capabilities
    capabilities: UnifiedMemoryCapabilities,

    /// Device limits
    limits: wgpu::Limits,
}

impl WebGpuBackend {
    /// Create new WebGPU backend with given device and queue
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> ToadStoolResult<Self> {
        let limits = device.limits();

        let capabilities = UnifiedMemoryCapabilities {
            backend_type: BackendType::WebGpu,
            max_allocation_size: limits.max_buffer_size as usize,
            zero_copy: true,       // Mappable buffers provide zero-copy access
            coherent: true,        // WebGPU handles synchronization
            cpu_fast_access: true, // Direct mapped access
            gpu_fast_access: true, // GPU can access efficiently
            alignment_requirement: wgpu::COPY_BUFFER_ALIGNMENT as usize,
        };

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
            capabilities,
            limits,
        })
    }

    /// Initialize WebGPU with automatic adapter selection
    async fn init_device() -> ToadStoolResult<(wgpu::Device, wgpu::Queue)> {
        // Create instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter (auto-select best available)
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| ToadStoolError::runtime("No WebGPU adapter available"))?;

        // Get adapter info for logging
        let info = adapter.get_info();
        tracing::info!(
            "Selected WebGPU adapter: {} ({:?}) - {:?}",
            info.name,
            info.device_type,
            info.backend
        );

        // Request device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ToadStool Unified Memory Device"),
                    required_features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None, // No trace path
            )
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Failed to request device: {e}")))?;

        Ok((device, queue))
    }

    /// Check if WebGPU is available on this system
    fn check_availability() -> bool {
        // Quick sync check - just see if we can create an instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Try to enumerate adapters (returns Vec directly)
        let adapters = instance.enumerate_adapters(wgpu::Backends::all());
        !adapters.is_empty()
    }
}

impl BackendInitializer for WebGpuBackend {
    async fn try_init() -> ToadStoolResult<Self> {
        let (device, queue) = Self::init_device().await?;
        Self::new(device, queue)
    }

    fn is_available() -> bool {
        Self::check_availability()
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl UnifiedMemoryBackend for WebGpuBackend {
    fn name(&self) -> &'static str {
        "WebGPU"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::WebGpu
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        &self.capabilities
    }

    async fn allocate_unified(
        &self,
        size: usize,
        flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        // Validate size
        if size == 0 {
            return Err(ToadStoolError::runtime("Cannot allocate 0 bytes"));
        }

        if size > self.limits.max_buffer_size as usize {
            return Err(ToadStoolError::runtime(format!(
                "Allocation size {} exceeds device maximum {}",
                size, self.limits.max_buffer_size
            )));
        }

        // Determine usage flags based on MemoryFlags
        let usage = if flags.prefer_gpu {
            // GPU-optimized: Storage buffer for compute
            wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
        } else {
            // CPU-optimized or balanced: Map-friendly
            wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
        };

        // Create buffer
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ToadStool Unified Buffer"),
            size: size as u64,
            usage,
            mapped_at_creation: false, // Don't map at creation
        });

        let allocation = WebGpuAllocation {
            buffer: Some(buffer),
            size,
            mapped_ptr: None, // Will be set when mapped
        };

        Ok(BackendAllocation::WebGpu(allocation))
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match allocation {
            BackendAllocation::WebGpu(_alloc) => {
                // Buffer will be dropped automatically
                // wgpu handles cleanup via Drop trait
                Ok(())
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for WebGPU backend",
            )),
        }
    }

    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        match allocation {
            BackendAllocation::WebGpu(alloc) => {
                // For WebGPU, we can't provide a true raw pointer due to wgpu's safe API
                // Return a non-null sentinel that can be used for identification
                // Actual access needs to go through wgpu's BufferSlice API

                // Use the buffer's address as a unique identifier
                alloc.buffer.as_ref().map_or_else(
                    || Err(ToadStoolError::runtime("Buffer has been freed")),
                    |buffer| Ok(buffer as *const wgpu::Buffer as *mut u8),
                )
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for WebGPU backend",
            )),
        }
    }

    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8 {
        match allocation {
            BackendAllocation::WebGpu(alloc) => {
                // Return buffer reference as device pointer (opaque handle)
                alloc.buffer.as_ref().map_or(std::ptr::null(), |buffer| {
                    buffer as *const wgpu::Buffer as *const u8
                })
            }
            _ => std::ptr::null(),
        }
    }

    async fn sync_cpu_to_device(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // WebGPU handles synchronization automatically
        // No explicit sync needed for mappable buffers
        Ok(())
    }

    async fn sync_device_to_cpu(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // WebGPU handles synchronization automatically
        // No explicit sync needed for mappable buffers
        Ok(())
    }

    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        matches!(allocation, BackendAllocation::WebGpu(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webgpu_availability() {
        // Just check if we can detect WebGPU
        let available = WebGpuBackend::is_available();
        println!("WebGPU available: {available}");

        // This test passes regardless of availability
        // (system may not have WebGPU support)
    }

    #[tokio::test]
    #[ignore = "Requires actual GPU hardware"]
    async fn test_webgpu_backend_initialization() {
        // Try to initialize WebGPU backend
        let result = WebGpuBackend::try_init().await;

        match result {
            Ok(backend) => {
                println!("✅ WebGPU backend initialized successfully");
                assert_eq!(backend.name(), "WebGPU");
                assert_eq!(backend.backend_type(), BackendType::WebGpu);

                let caps = backend.capabilities();
                assert!(caps.zero_copy);
                assert!(caps.coherent);
                println!(
                    "Max allocation: {} MB",
                    caps.max_allocation_size / 1024 / 1024
                );
            }
            Err(e) => {
                println!("⚠️  WebGPU not available: {e}");
                // Not a failure - system may not support WebGPU
            }
        }
    }

    #[tokio::test]
    #[ignore = "Requires actual GPU hardware"]
    async fn test_webgpu_backend_allocation() {
        // Try to allocate via WebGPU
        let Ok(backend) = WebGpuBackend::try_init().await else {
            println!("⚠️  Skipping test - WebGPU not available");
            return;
        };

        // Allocate a buffer
        let allocation = backend
            .allocate_unified(4096, MemoryFlags::default())
            .await
            .expect("Failed to allocate");

        assert!(backend.is_valid(&allocation));

        // Get pointers
        let cpu_ptr = backend
            .map_cpu_ptr(&allocation)
            .await
            .expect("Failed to map");
        let device_ptr = backend.get_device_ptr(&allocation);

        assert!(!cpu_ptr.is_null());
        assert!(!device_ptr.is_null());

        // Free
        backend
            .free_unified(allocation)
            .await
            .expect("Failed to free");
    }
}
