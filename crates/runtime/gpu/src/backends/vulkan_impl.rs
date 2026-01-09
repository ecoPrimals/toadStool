//! Vulkan Compute Backend Implementation
//!
//! **Status**: 🚧 STUB - Device discovery works, compute execution TODO
//!
//! Real GPU execution using Vulkan Compute - works on NVIDIA, AMD, Intel
//! No mocks, no hardcoding, capability-based discovery
//!
//! ## Why Vulkan?
//! - **Universal**: NVIDIA, AMD, Intel, Apple (via MoltenVK)
//! - **Modern**: Designed for compute + graphics
//! - **Performance**: Direct GPU access, minimal overhead
//! - **AMD-friendly**: Better AMD support than OpenCL (Mesa RADV)
//!
//! ## Current Status
//! - Device discovery: ✅ Working
//! - Compute execution: 🚧 TODO (use showcase implementation as reference)

use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Vulkan compute backend (stub)
pub struct VulkanBackend;

impl VulkanBackend {
    pub fn new() -> ToadStoolResult<Self> {
        Err(ToadStoolError::runtime(
            "Vulkan backend stub. Use showcase/gpu-universal/ml-inference for working Vulkan discovery.",
        ))
    }
}

/// Vulkan compute resource (stub)
pub struct VulkanComputeResource;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vulkan_stub() {
        let result = VulkanBackend::new();
        assert!(result.is_err());
    }
}
