// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright © 2026 ecoPrimals
//
// Hardware driver: requires unsafe for kernel ioctl/mmap/MMIO.
// Inherits workspace pedantic/nursery but relaxes hardware-specific patterns.
#![allow(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::unreadable_literal,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::cast_ptr_alignment,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::missing_const_for_fn,
    clippy::items_after_statements,
    clippy::similar_names,
    clippy::wildcard_imports,
    clippy::redundant_pub_crate,
    clippy::match_same_arms,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    clippy::option_if_let_else,
    clippy::redundant_clone,
    clippy::unnecessary_wraps,
    clippy::struct_excessive_bools,
    clippy::too_long_first_doc_paragraph,
    clippy::implicit_clone,
    clippy::map_unwrap_or,
    clippy::used_underscore_binding,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_bitwise_bool,
    clippy::unnecessary_struct_initialization,
    clippy::if_not_else,
    clippy::verbose_bit_mask,
    clippy::use_self,
    clippy::needless_lifetimes,
    clippy::trivially_copy_pass_by_ref,
    clippy::manual_let_else,
    clippy::non_send_fields_in_send_ty,
    clippy::uninlined_format_args,
    clippy::cloned_instead_of_copied,
    clippy::assigning_clones,
    clippy::branches_sharing_code,
    clippy::cast_precision_loss,
    clippy::unchecked_time_subtraction,
    clippy::single_match_else,
    clippy::struct_field_names,
    clippy::inconsistent_struct_constructor,
    clippy::needless_borrows_for_generic_args,
    clippy::or_fun_call,
    clippy::redundant_else,
    clippy::missing_fields_in_debug,
    clippy::bool_to_int_with_if,
    clippy::semicolon_if_nothing_returned,
    clippy::unused_self,
    clippy::used_underscore_items,
    clippy::needless_return,
    clippy::duplicated_attributes,
    reason = "hardware driver: register/MMIO patterns require casts, bitwise ops, and kernel-mirrored naming that conflict with pedantic lints"
)]
//! # toadstool-cylinder — Sovereign Hardware Driver Layer
//!
//! Phase C absorption of `coral-driver`'s hardware lifecycle modules into
//! toadStool's ownership. Provides DRM render node enumeration, VFIO device
//! management, and vendor-specific GPU dispatch (AMD amdgpu, NVIDIA nouveau/VFIO).
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────┐
//! │    ComputeDevice trait   │  ← vendor-agnostic API
//! ├──────────────────────────┤
//! │  AmdDevice               │  ← amdgpu DRM backend
//! │  NvDevice                │  ← nouveau DRM backend
//! │  NvVfioComputeDevice     │  ← VFIO direct-dispatch backend
//! └──────────────────────────┘
//!          │                │
//!     ioctl::drm        vfio/   ← pure Rust ioctl wrappers
//!          │                │
//!     /dev/dri/renderD*  /dev/vfio/* ← Linux DRM / VFIO
//! ```

pub mod error;
pub mod hardware;
pub mod linux_paths;

#[cfg(target_os = "linux")]
pub mod drm;

pub use error::{ChannelError, DevinitError, DriverError, DriverResult, PciDiscoveryError};
pub use hardware::{CompletionStyle, HardwareCapabilities, MemoryType, Vendor, WaveSize};

/// An opaque GPU buffer handle.
///
/// Handles are created by [`ComputeDevice::alloc`] and consumed by other
/// device operations. The raw ID is not exposed — callers cannot forge
/// handles, ensuring the driver owns the validity invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub(crate) u32);

impl BufferHandle {
    /// Create a handle from a raw ID. For mock devices; enable `test-utils` feature.
    #[cfg(feature = "test-utils")]
    #[must_use]
    pub const fn from_id(id: u32) -> Self {
        Self(id)
    }
}

/// GPU memory domain for buffer placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryDomain {
    /// Device-local VRAM (fastest for GPU access).
    Vram,
    /// Host-visible system memory (CPU-accessible).
    Gtt,
    /// Either VRAM or GTT (driver picks based on size/pressure).
    VramOrGtt,
}

/// Compute dispatch dimensions.
#[derive(Debug, Clone, Copy)]
pub struct DispatchDims {
    /// Number of workgroups in the X dimension.
    pub x: u32,
    /// Number of workgroups in the Y dimension.
    pub y: u32,
    /// Number of workgroups in the Z dimension.
    pub z: u32,
}

impl DispatchDims {
    /// Create dispatch dimensions for a 3D grid.
    #[must_use]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Create dispatch dimensions for a 1D linear grid.
    #[must_use]
    pub const fn linear(n: u32) -> Self {
        Self { x: n, y: 1, z: 1 }
    }
}

/// Compiler-derived metadata passed to the driver for QMD construction.
#[derive(Debug, Clone, Copy)]
pub struct ShaderInfo {
    /// General-purpose register count (from compiler RA).
    pub gpr_count: u32,
    /// Shared memory in bytes (from shader analysis).
    pub shared_mem_bytes: u32,
    /// Barrier count used by the shader.
    pub barrier_count: u32,
    /// Workgroup size (threads per CTA), from `@workgroup_size`.
    pub workgroup: [u32; 3],
    /// Wave/warp size: 32 for RDNA wave32 / NVIDIA, 64 for GCN wave64.
    pub wave_size: u32,
    /// Per-thread local (scratch) memory in bytes.
    pub local_mem_bytes: Option<u32>,
}

impl Default for ShaderInfo {
    fn default() -> Self {
        Self {
            gpr_count: 0,
            shared_mem_bytes: 0,
            barrier_count: 0,
            workgroup: [1, 1, 1],
            wave_size: 32,
            local_mem_bytes: None,
        }
    }
}

/// Vendor-agnostic GPU compute device.
///
/// Implementations provide the full lifecycle: open device, allocate
/// buffers, upload shader binary, dispatch workgroups, synchronize,
/// and read back results.
pub trait ComputeDevice: Send + Sync {
    /// Allocate a GPU buffer.
    fn alloc(&mut self, size: u64, domain: MemoryDomain) -> DriverResult<BufferHandle>;

    /// Free a GPU buffer.
    fn free(&mut self, handle: BufferHandle) -> DriverResult<()>;

    /// Upload data from host to a GPU buffer.
    fn upload(&mut self, handle: BufferHandle, offset: u64, data: &[u8]) -> DriverResult<()>;

    /// Read data from a GPU buffer to host.
    fn readback(&self, handle: BufferHandle, offset: u64, len: usize) -> DriverResult<Vec<u8>>;

    /// Dispatch a compute shader.
    fn dispatch(
        &mut self,
        shader: &[u8],
        buffers: &[BufferHandle],
        dims: DispatchDims,
        info: &ShaderInfo,
    ) -> DriverResult<()>;

    /// Wait for all submitted work to complete.
    fn sync(&mut self) -> DriverResult<()>;

    /// Query the hardware capabilities of this device.
    fn capabilities(&self) -> &HardwareCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_handle_equality() {
        assert_eq!(BufferHandle(1), BufferHandle(1));
        assert_ne!(BufferHandle(1), BufferHandle(2));
    }

    #[test]
    fn buffer_handle_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BufferHandle(1));
        set.insert(BufferHandle(2));
        assert!(set.contains(&BufferHandle(1)));
        assert!(!set.contains(&BufferHandle(99)));
    }

    #[test]
    fn dispatch_dims_new() {
        let d = DispatchDims::new(8, 4, 2);
        assert_eq!(d.x, 8);
        assert_eq!(d.y, 4);
        assert_eq!(d.z, 2);
    }

    #[test]
    fn dispatch_dims_linear() {
        let d = DispatchDims::linear(256);
        assert_eq!(d.x, 256);
        assert_eq!(d.y, 1);
        assert_eq!(d.z, 1);
    }

    #[test]
    fn memory_domain_equality() {
        assert_eq!(MemoryDomain::Vram, MemoryDomain::Vram);
        assert_ne!(MemoryDomain::Vram, MemoryDomain::Gtt);
    }

    #[test]
    fn shader_info_default() {
        let info = ShaderInfo::default();
        assert_eq!(info.gpr_count, 0);
        assert_eq!(info.workgroup, [1, 1, 1]);
        assert_eq!(info.wave_size, 32);
        assert_eq!(info.local_mem_bytes, None);
    }
}
