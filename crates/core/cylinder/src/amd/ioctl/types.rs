// SPDX-License-Identifier: AGPL-3.0-or-later
//! AMD DRM ioctl types — `#[repr(C)]` structures matching the kernel ABI.

// amdgpu DRM ioctl command numbers (from amdgpu_drm.h)
pub(super) const DRM_COMMAND_BASE: u32 = 0x40;
pub(super) const DRM_AMDGPU_GEM_CREATE: u32 = DRM_COMMAND_BASE;
pub(super) const DRM_AMDGPU_GEM_MMAP: u32 = DRM_COMMAND_BASE + 0x01;
pub(super) const DRM_AMDGPU_CTX: u32 = DRM_COMMAND_BASE + 0x02;
pub(super) const DRM_AMDGPU_GEM_VA: u32 = DRM_COMMAND_BASE + 0x08;
pub(super) const DRM_AMDGPU_BO_LIST: u32 = DRM_COMMAND_BASE + 0x03;
pub(super) const DRM_AMDGPU_CS: u32 = DRM_COMMAND_BASE + 0x04;
pub(super) const DRM_AMDGPU_INFO: u32 = DRM_COMMAND_BASE + 0x05;
pub(super) const DRM_AMDGPU_WAIT_CS: u32 = DRM_COMMAND_BASE + 0x09;

/// GEM domain: device-local VRAM.
pub const AMDGPU_GEM_DOMAIN_VRAM: u32 = 0x4;
/// GEM domain: host-visible GTT (system memory).
pub const AMDGPU_GEM_DOMAIN_GTT: u32 = 0x2;

/// Use Write-Combine mapping for GTT buffers.
pub const AMDGPU_GEM_CREATE_CPU_GTT_USWC: u64 = 1 << 2;

pub(super) const AMDGPU_CTX_OP_ALLOC_CTX: u32 = 1;
pub(super) const AMDGPU_CTX_OP_FREE_CTX: u32 = 2;

/// VA operation: map a buffer into GPU VA space.
pub const AMDGPU_VA_OP_MAP: u32 = 1;
/// VA operation: unmap a buffer from GPU VA space.
pub const AMDGPU_VA_OP_UNMAP: u32 = 2;
/// VA flags: no special flags.
pub const AMDGPU_VA_FLAGS_NONE: u64 = 0;

/// VM page flag: readable by GPU.
pub const AMDGPU_VM_PAGE_READABLE: u32 = 1 << 1;
/// VM page flag: writable by GPU.
pub const AMDGPU_VM_PAGE_WRITEABLE: u32 = 1 << 2;
/// VM page flag: executable by GPU (for shader code).
pub const AMDGPU_VM_PAGE_EXECUTABLE: u32 = 1 << 3;

/// GFX (graphics + compute) ring.
pub const AMDGPU_HW_IP_GFX: u32 = 0;
/// Dedicated COMPUTE (MEC) ring.
pub const AMDGPU_HW_IP_COMPUTE: u32 = 1;

/// GEM create — matches `union drm_amdgpu_gem_create` (32 bytes).
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AmdgpuGemCreate {
    /// Buffer size in bytes (input); kernel returns handle in first 8 bytes.
    pub bo_size: u64,
    /// Alignment requirement in bytes.
    pub alignment: u64,
    /// Memory domains (VRAM, GTT) as bitmask.
    pub domains: u64,
    /// Domain-specific flags.
    pub domain_flags: u64,
}

/// GEM mmap — matches `union drm_amdgpu_gem_mmap` (8 bytes).
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AmdgpuGemMmap {
    /// Input: GEM handle (low 32 bits). Output: mmap offset address.
    pub handle_or_addr: u64,
}

/// Context operation input/output.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AmdgpuCtx {
    /// Operation (alloc/free).
    pub op: u32,
    /// Context flags.
    pub flags: u32,
    /// Context ID (input for free; output for alloc).
    pub ctx_id: u32,
    /// Padding for alignment.
    pub pad: u32,
}

/// GEM VA mapping.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AmdgpuGemVa {
    /// GEM buffer handle.
    pub handle: u32,
    /// Padding for alignment.
    pub pad: u32,
    /// VA operation (map/unmap).
    pub operation: u32,
    /// Page protection flags.
    pub flags: u32,
    /// GPU virtual address to map at.
    pub va_address: u64,
    /// Offset within the buffer object.
    pub offset_in_bo: u64,
    /// Size of the mapping in bytes.
    pub map_size: u64,
}

// --- BO list structs ---

pub(super) const AMDGPU_BO_LIST_OP_CREATE: u32 = 0;
pub(super) const AMDGPU_BO_LIST_OP_DESTROY: u32 = 1;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuBoListEntry {
    pub bo_handle: u32,
    pub bo_priority: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuBoListIn {
    pub operation: u32,
    pub list_handle: u32,
    pub bo_number: u32,
    pub bo_info_size: u32,
    pub bo_info_ptr: u64,
}

// --- CS submission structs ---

pub(super) const AMDGPU_CHUNK_ID_IB: u32 = 0x01;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuCsChunk {
    pub chunk_id: u32,
    pub length_dw: u32,
    pub chunk_data: u64,
}

/// IB chunk data — matches `drm_amdgpu_cs_chunk_ib`.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuCsChunkIb {
    pub pad: u32,
    pub flags: u32,
    pub va_start: u64,
    pub ib_bytes: u32,
    pub ip_type: u32,
    pub ip_instance: u32,
    pub ring: u32,
}

/// CS input — matches `drm_amdgpu_cs_in` (24 bytes).
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuCsIn {
    pub ctx_id: u32,
    pub bo_list_handle: u32,
    pub num_chunks: u32,
    pub flags: u32,
    pub chunks: u64,
}

// --- Wait CS structs ---

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuWaitCsIn {
    pub handle: u64,
    pub timeout: u64,
    pub ip_type: u32,
    pub ip_instance: u32,
    pub ring: u32,
    pub ctx_id: u32,
}

// --- Device info structs ---

pub(super) const AMDGPU_INFO_HW_IP_INFO: u32 = 0x01;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuInfoHwIp {
    pub hw_ip_version_major: u32,
    pub hw_ip_version_minor: u32,
    pub capabilities_flags: u64,
    pub ib_start_alignment: u32,
    pub ib_size_alignment: u32,
    pub available_rings: u32,
    pub pad: u32,
}

/// Raw layout for `DRM_AMDGPU_INFO`.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct AmdgpuInfoRequestRaw {
    pub return_pointer: u64,
    pub return_size: u32,
    pub query: u32,
    pub hw_ip_type: u32,
    pub hw_ip_instance: u32,
    pub pad: [u32; 2],
}
