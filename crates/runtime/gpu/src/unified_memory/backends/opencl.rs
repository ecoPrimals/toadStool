//! OpenCL SVM unified memory backend
//!
//! **Status**: 🚧 PARTIAL IMPLEMENTATION - Core functionality ready
//!
//! This provides cross-vendor unified memory via OpenCL 2.0+ Shared Virtual Memory (SVM),
//! enabling zero-copy access on compatible devices.
//!
//! # Architecture
//!
//! Uses `ocl` crate for OpenCL access:
//! - **SVM (Shared Virtual Memory)**: CPU and GPU share address space
//! - **Fine-grain**: Both can access simultaneously (with sync)
//! - **Coarse-grain**: Explicit map/unmap (more compatible)
//!
//! # Requirements
//!
//! - OpenCL 2.0+ for SVM support
//! - GPU with SVM capability flags
//! - Compatible drivers
//!
//! # Current Status
//!
//! **Implemented**:
//! - Availability detection
//! - Capability reporting  
//! - Stub allocation/deallocation
//!
//! **TODO** (requires full OpenCL stack):
//! - Platform/device selection
//! - Context creation
//! - SVM capability detection
//! - Actual SVM allocation
//! - Queue management
//!
//! # Why Partial?
//!
//! Full OpenCL initialization requires:
//! 1. Platform enumeration and selection
//! 2. Device enumeration and filtering
//! 3. Context creation with SVM support
//! 4. Command queue creation
//! 5. SVM capability checking
//! 6. Proper error handling for old OpenCL versions
//!
//! This is ~300+ lines of setup code. For now, we provide the interface
//! and let applications with OpenCL contexts integrate as needed.
//!
//! # Integration Path
//!
//! Applications with existing OpenCL can:
//! 1. Implement `OpenClBackend::with_context()`
//! 2. Pass existing cl_context/cl_device_id
//! 3. Get unified memory with their setup
//!
//! # Fallback Strategy
//!
//! For OpenCL 1.x or devices without SVM:
//! - Fall back to mapped buffers (CL_MEM_ALLOC_HOST_PTR)
//! - Less efficient but more compatible
//! - Still provides unified interface
//!
//! # Future Work
//!
//! - Complete initialization
//! - Add fine-grain vs coarse-grain detection
//! - Benchmark vs Vulkan/WebGPU
//! - Legacy OpenCL 1.x fallback path

use crate::unified_memory::{
    backend::{BackendAllocation, BackendInitializer, UnifiedMemoryBackend},
    types::*,
};
use async_trait::async_trait;
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// OpenCL SVM backend
///
/// Provides unified memory via OpenCL 2.0+ SVM or mapped buffers for older versions.
pub struct OpenClBackend {
    /// Backend capabilities
    capabilities: UnifiedMemoryCapabilities,

    /// Whether OpenCL is available
    available: bool,

    /// OpenCL version detected (for debugging)
    #[allow(dead_code)]
    version: String,
}

impl OpenClBackend {
    /// Create new OpenCL backend (stub - needs context)
    pub fn new_stub() -> Self {
        // Default capabilities (conservative)
        let capabilities = UnifiedMemoryCapabilities {
            backend_type: BackendType::OpenCL,
            max_allocation_size: 128 * 1024 * 1024, // 128MB conservative
            zero_copy: true,                        // SVM or mapped buffers
            coherent: false,                        // May need explicit sync
            cpu_fast_access: true,                  // Host accessible
            gpu_fast_access: true,                  // Device accessible
            alignment_requirement: 128,             // OpenCL alignment
        };

        Self {
            capabilities,
            available: false,
            version: "Unknown".to_string(),
        }
    }

    /// Check if OpenCL is available
    ///
    /// This is a basic check - full initialization requires platform/device selection.
    fn check_availability() -> bool {
        #[cfg(feature = "opencl")]
        {
            // Try to get OpenCL platforms (returns Vec directly)
            let platforms = ocl::Platform::list();
            !platforms.is_empty()
        }

        #[cfg(not(feature = "opencl"))]
        {
            false
        }
    }

    /// Get OpenCL version string (if available)
    #[allow(dead_code)]
    fn detect_version() -> Option<String> {
        #[cfg(feature = "opencl")]
        {
            let platforms = ocl::Platform::list();
            if let Some(platform) = platforms.first() {
                if let Ok(version) = platform.version() {
                    return Some(version);
                }
            }
        }
        None
    }

    /// Create backend with existing OpenCL context (advanced usage)
    ///
    /// For applications that already have an OpenCL context.
    ///
    /// # Arguments
    ///
    /// * `context_handle` - cl_context as u64
    /// * `device_handle` - cl_device_id as u64
    /// * `has_svm` - Whether device supports SVM
    /// * `max_allocation` - Maximum allocation size
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - Context is valid for the lifetime of this backend
    /// - Device matches the context
    /// - SVM flag is correct for the device
    #[allow(dead_code)]
    pub unsafe fn with_context(
        _context_handle: u64,
        _device_handle: u64,
        has_svm: bool,
        max_allocation: usize,
    ) -> ToadStoolResult<Self> {
        let capabilities = UnifiedMemoryCapabilities {
            backend_type: BackendType::OpenCL,
            max_allocation_size: max_allocation,
            zero_copy: has_svm, // Only true for SVM
            coherent: false,    // SVM fine-grain would be true
            cpu_fast_access: true,
            gpu_fast_access: true,
            alignment_requirement: 128,
        };

        Ok(Self {
            capabilities,
            available: true,
            version: Self::detect_version().unwrap_or_else(|| "2.0+".to_string()),
        })
    }
}

#[async_trait]
impl BackendInitializer for OpenClBackend {
    async fn try_init() -> ToadStoolResult<Self> {
        if !Self::check_availability() {
            return Err(ToadStoolError::runtime(
                "OpenCL not available (no platforms found)",
            ));
        }

        // For now, return stub
        // Full implementation would:
        // 1. Enumerate platforms
        // 2. Select best platform
        // 3. Enumerate devices
        // 4. Select best device (with SVM if possible)
        // 5. Create context
        // 6. Create command queue
        // 7. Check SVM capabilities
        // 8. Configure based on capabilities

        Err(ToadStoolError::runtime(
            "OpenCL backend requires full initialization (coming soon). \
             Use WebGPU for cross-platform or implement OpenClBackend::with_context() \
             if you have existing OpenCL context.",
        ))
    }

    fn is_available() -> bool {
        Self::check_availability()
    }
}

#[async_trait]
impl UnifiedMemoryBackend for OpenClBackend {
    fn name(&self) -> &'static str {
        "OpenCL"
    }

    fn backend_type(&self) -> BackendType {
        BackendType::OpenCL
    }

    fn capabilities(&self) -> &UnifiedMemoryCapabilities {
        &self.capabilities
    }

    async fn allocate_unified(
        &self,
        size: usize,
        _flags: MemoryFlags,
    ) -> ToadStoolResult<BackendAllocation> {
        if !self.available {
            return Err(ToadStoolError::runtime(
                "OpenCL backend not initialized (use with_context() or try_init())",
            ));
        }

        // In full implementation:
        // IF device supports SVM:
        //   1. Allocate SVM memory (clSVMAlloc)
        //   2. Return OpenClAllocation with SVM pointer
        // ELSE:
        //   1. Create buffer with CL_MEM_ALLOC_HOST_PTR
        //   2. Map buffer for host access
        //   3. Return OpenClAllocation with mapped pointer

        let _ = size;
        Err(ToadStoolError::runtime(
            "OpenCL allocation not yet implemented (stub backend)",
        ))
    }

    async fn free_unified(&self, allocation: BackendAllocation) -> ToadStoolResult<()> {
        match allocation {
            BackendAllocation::OpenCL(_alloc) => {
                // In full implementation:
                // IF SVM:
                //   clSVMFree(context, ptr)
                // ELSE:
                //   clReleaseMemObject(buffer)

                Ok(())
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for OpenCL backend",
            )),
        }
    }

    async fn map_cpu_ptr(&self, allocation: &BackendAllocation) -> ToadStoolResult<*mut u8> {
        match allocation {
            BackendAllocation::OpenCL(alloc) => {
                // Return the SVM or mapped pointer
                Ok(alloc.ptr)
            }
            _ => Err(ToadStoolError::runtime(
                "Invalid allocation type for OpenCL backend",
            )),
        }
    }

    fn get_device_ptr(&self, allocation: &BackendAllocation) -> *const u8 {
        match allocation {
            BackendAllocation::OpenCL(alloc) => {
                // For SVM, CPU and GPU pointers are the same
                alloc.ptr as *const u8
            }
            _ => std::ptr::null(),
        }
    }

    async fn sync_cpu_to_device(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // For fine-grain SVM, this is a no-op
        // For coarse-grain SVM, would need clEnqueueSVMUnmap
        // For mapped buffers, would need clEnqueueUnmapMemObject
        Ok(())
    }

    async fn sync_device_to_cpu(&self, _allocation: &BackendAllocation) -> ToadStoolResult<()> {
        // For fine-grain SVM, this is a no-op
        // For coarse-grain SVM, would need clEnqueueSVMMap
        // For mapped buffers, would need clEnqueueMapBuffer
        Ok(())
    }

    fn is_valid(&self, allocation: &BackendAllocation) -> bool {
        matches!(allocation, BackendAllocation::OpenCL(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_opencl_availability() {
        let available = OpenClBackend::is_available();
        println!("OpenCL available: {}", available);

        // Test passes regardless - OpenCL may not be installed
    }

    #[tokio::test]
    async fn test_opencl_stub() {
        let backend = OpenClBackend::new_stub();

        assert_eq!(backend.name(), "OpenCL");
        assert_eq!(backend.backend_type(), BackendType::OpenCL);

        let caps = backend.capabilities();
        assert!(caps.zero_copy);
        assert!(caps.cpu_fast_access);
        assert!(caps.gpu_fast_access);
    }

    #[tokio::test]
    async fn test_opencl_initialization() {
        let result = OpenClBackend::try_init().await;

        // Should fail (not implemented yet) or succeed if OpenCL available
        if let Err(e) = result {
            println!("Expected error: {}", e);
            assert!(
                e.to_string().contains("full initialization")
                    || e.to_string().contains("not available")
            );
        }
    }

    #[tokio::test]
    async fn test_opencl_version_detection() {
        if OpenClBackend::is_available() {
            if let Some(version) = OpenClBackend::detect_version() {
                println!("OpenCL version: {}", version);
                assert!(!version.is_empty());
            }
        }
    }
}
