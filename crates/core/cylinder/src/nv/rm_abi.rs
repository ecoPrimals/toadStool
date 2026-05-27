// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA RM ioctl ABI — types and constants ported from coral-kmod's `coral_kmod.h`.
//!
//! This module is the **canonical source** for RM ABI definitions in toadStool.
//! Local duplicates in `src/bin/rm_trigger.rs` (and elsewhere) should migrate here.

#![allow(dead_code, missing_docs, reason = "RM ABI definitions — hardware constants and ioctl structs")]

use bytemuck::Zeroable;

// ── Ioctl escape numbers ────────────────────────────────────────────────

pub const NV_IOCTL_MAGIC: u8 = b'F';

pub const NV_ESC_REGISTER_FD: u8 = 201;
pub const NV_ESC_RM_FREE: u8 = 0x29;
pub const NV_ESC_RM_CONTROL: u8 = 0x2A;
pub const NV_ESC_RM_ALLOC: u8 = 0x2B;
pub const NV_ESC_RM_MAP_MEMORY: u8 = 0x4E;
pub const NV_ESC_RM_UNMAP_MEMORY: u8 = 0x4F;
pub const NV_ESC_RM_MAP_MEMORY_DMA: u8 = 0x57;
pub const NV_ESC_RM_UNMAP_MEMORY_DMA: u8 = 0x58;

// ── RM status codes ─────────────────────────────────────────────────────

pub mod status {
    pub const NV_OK: u32 = 0x0000_0000;
    pub const INSUFFICIENT_RESOURCES: u32 = 0x0000_001A;
    pub const INSUFFICIENT_PERMISSIONS: u32 = 0x0000_001B;
    pub const INVALID_ARGUMENT: u32 = 0x0000_001F;
    pub const INVALID_CLIENT: u32 = 0x0000_0023;
    pub const INVALID_DEVICE: u32 = 0x0000_0026;
    pub const INVALID_OBJECT_HANDLE: u32 = 0x0000_0033;
    pub const INVALID_OBJECT_PARENT: u32 = 0x0000_0036;
    pub const INVALID_STATE: u32 = 0x0000_0040;
    pub const NO_MEMORY: u32 = 0x0000_0051;
    pub const NOT_SUPPORTED: u32 = 0x0000_0056;
    pub const OBJECT_NOT_FOUND: u32 = 0x0000_0057;
    pub const OPERATING_SYSTEM: u32 = 0x0000_0059;
    pub const PAGE_TABLE_NOT_AVAIL: u32 = 0x0000_005D;
    pub const TIMEOUT: u32 = 0x0000_0065;
}

// ── RM class IDs ────────────────────────────────────────────────────────

pub mod class {
    // Core objects
    pub const NV01_ROOT: u32 = 0x0000_0000;
    pub const NV01_ROOT_CLIENT: u32 = 0x0000_0041;
    pub const NV01_DEVICE_0: u32 = 0x0000_0080;
    pub const NV20_SUBDEVICE_0: u32 = 0x0000_2080;

    // Fermi
    pub const FERMI_VASPACE_A: u32 = 0x0000_90F1;
    pub const FERMI_CONTEXT_SHARE_A: u32 = 0x0000_9067;

    // Kepler
    pub const KEPLER_CHANNEL_GROUP_A: u32 = 0x0000_A06C;

    // Volta
    pub const VOLTA_CHANNEL_GPFIFO_A: u32 = 0x0000_C36F;
    pub const VOLTA_COMPUTE_A: u32 = 0x0000_C3C0;
    pub const VOLTA_USERMODE_A: u32 = 0x0000_C361;

    // Turing
    pub const TURING_COMPUTE_A: u32 = 0x0000_C5C0;

    // Ampere
    pub const AMPERE_CHANNEL_GPFIFO_A: u32 = 0x0000_C56F;
    pub const AMPERE_COMPUTE_A: u32 = 0x0000_C6C0;
    pub const AMPERE_COMPUTE_B: u32 = 0x0000_C7C0;

    // Ada
    pub const ADA_COMPUTE_A: u32 = 0x0000_C9C0;

    // Hopper
    pub const HOPPER_COMPUTE_A: u32 = 0x0000_CBC0;

    // Blackwell
    pub const BLACKWELL_CHANNEL_GPFIFO_A: u32 = 0x0000_C96F;
    pub const BLACKWELL_CHANNEL_GPFIFO_B: u32 = 0x0000_CA6F;
    pub const BLACKWELL_COMPUTE_A: u32 = 0x0000_CDC0;
    pub const BLACKWELL_COMPUTE_B: u32 = 0x0000_CEC0;

    // Memory
    pub const NV01_MEMORY_SYSTEM: u32 = 0x0000_003E;
    pub const NV01_MEMORY_LOCAL_USER: u32 = 0x0000_0040;
    pub const NV01_MEMORY_VIRTUAL: u32 = 0x0000_0070;
}

// ── VA space flags ──────────────────────────────────────────────────────

pub const NV_VASPACE_FLAGS_ENABLE_FAULTING: u32 = 0x0000_0004;
pub const NV_VASPACE_FLAGS_ENABLE_PAGE_FAULTING: u32 = 0x0000_0040;

// ── Memory alloc flags/attrs ────────────────────────────────────────────

pub const NVOS32_ALLOC_FLAGS_MAP_NOT_REQUIRED: u32 = 0x0000_0001;
pub const NVOS32_ALLOC_FLAGS_IGNORE_BANK_PLACEMENT: u32 = 0x0000_4000;
pub const NVOS32_ALLOC_FLAGS_ALIGNMENT_FORCE: u32 = 0x0000_8000;
pub const NVOS32_ATTR_PHYSICALITY_NONCONTIGUOUS: u32 = 0x0200_0000;
pub const NVOS32_ATTR_PHYSICALITY_CONTIGUOUS: u32 = 0x0400_0000;
pub const NVOS32_ATTR2_32BIT_ADDRESSABLE: u32 = 0x0000_0001;
pub const NVOS46_FLAGS_SHADER_ACCESS_READ_WRITE: u32 = 3 << 6;

// ── Engine types ────────────────────────────────────────────────────────

pub const NV2080_ENGINE_TYPE_GR0: u32 = 0x0000_0001;

// ── RM control commands ─────────────────────────────────────────────────

pub const NV2080_CTRL_CMD_GPU_GET_GID_INFO: u32 = 0x2080_014A;
pub const NV2080_CTRL_CMD_GR_CTXSW_SETUP_BIND: u32 = 0x2080_123A;
pub const NV2080_CTRL_CMD_GPU_PROMOTE_CTX: u32 = 0x2080_012B;
pub const NV2080_CTRL_CMD_INTERNAL_STATIC_KGR_GET_CONTEXT_BUFFERS_INFO: u32 = 0x2080_0A32;
pub const NVA06F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN: u32 = 0xC36F_0108;
pub const NV003E_CTRL_CMD_GET_SURFACE_PHYS_ATTR: u32 = 0x003E_0101;

// ── Channel scheduling ──────────────────────────────────────────────────

pub const NVA06C_CTRL_CMD_GPFIFO_SCHEDULE: u32 = 0xA06C_0101;
pub const NV906F_CTRL_CMD_BIND: u32 = 0x906F_0101;

// ── Promote context buffer limits ───────────────────────────────────────

pub const GPU_PROMOTE_CONTEXT_MAX_ENTRIES: usize = 16;
pub const ENGINE_CONTEXT_PROPERTIES_ENGINE_ID_COUNT: usize = 0x1A;
pub const INTERNAL_GR_MAX_ENGINES: usize = 8;
pub const NV_MAX_SUBDEVICES: usize = 8;

// ── Engine context property indices ─────────────────────────────────────

pub const ENGINE_CTX_ID_GRAPHICS: u32 = 0x00;
pub const ENGINE_CTX_ID_GRAPHICS_PATCH: u32 = 0x09;
pub const ENGINE_CTX_ID_GRAPHICS_BUNDLE_CB: u32 = 0x01;
pub const ENGINE_CTX_ID_GRAPHICS_PAGEPOOL: u32 = 0x04;
pub const ENGINE_CTX_ID_GRAPHICS_ATTRIBUTE_CB: u32 = 0x02;
pub const ENGINE_CTX_ID_GRAPHICS_RTV_CB_GLOBAL: u32 = 0x0B;
pub const ENGINE_CTX_ID_GRAPHICS_FECS_EVENT: u32 = 0x0D;
pub const ENGINE_CTX_ID_GRAPHICS_PRIV_ACCESS_MAP: u32 = 0x11;

// ════════════════════════════════════════════════════════════════════════
// NVIDIA RM ioctl parameter structs (repr(C), match kernel ABI)
// ════════════════════════════════════════════════════════════════════════

/// RM alloc ioctl parameters (`NV_ESC_RM_ALLOC`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmAllocParams {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_new: u32,
    pub h_class: u32,
    /// Pointer to class-specific params (user-kernel boundary).
    pub p_alloc_parms: u64,
    pub p_rights_requested: u64,
    pub params_size: u32,
    pub flags: u32,
    pub status: u32,
    _pad: u32,
}

const _: () = assert!(core::mem::size_of::<NvRmAllocParams>() == 48);

/// RM control ioctl parameters (`NV_ESC_RM_CONTROL`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmControlParams {
    pub h_client: u32,
    pub h_object: u32,
    pub cmd: u32,
    pub flags: u32,
    /// Pointer to cmd-specific params (user-kernel boundary).
    pub params: u64,
    pub params_size: u32,
    pub status: u32,
}

const _: () = assert!(core::mem::size_of::<NvRmControlParams>() == 32);

/// RM free ioctl parameters (`NV_ESC_RM_FREE`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmFreeParams {
    pub h_root: u32,
    pub h_object_parent: u32,
    pub h_object_old: u32,
    pub status: u32,
}

const _: () = assert!(core::mem::size_of::<NvRmFreeParams>() == 16);

/// RM map-memory-DMA ioctl parameters (`NV_ESC_RM_MAP_MEMORY_DMA`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmMapMemoryDmaParams {
    pub h_client: u32,
    pub h_device: u32,
    /// Virtual memory handle.
    pub h_dma: u32,
    pub h_memory: u32,
    pub offset: u64,
    pub length: u64,
    pub flags: u32,
    pub flags2: u32,
    pub kind_override: u32,
    pub pad: u32,
    /// Out: GPU VA.
    pub dma_offset: u64,
    pub status: u32,
    pub pad2: u32,
}

const _: () = assert!(core::mem::size_of::<NvRmMapMemoryDmaParams>() == 64);

/// RM unmap-memory-DMA ioctl parameters (`NV_ESC_RM_UNMAP_MEMORY_DMA`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmUnmapMemoryDmaParams {
    pub h_client: u32,
    pub h_device: u32,
    pub h_dma: u32,
    pub h_memory: u32,
    pub flags: u32,
    pub pad: u32,
    pub dma_offset: u64,
    pub status: u32,
    pub pad2: u32,
}

/// Register-fd ioctl parameters (`NV_ESC_REGISTER_FD`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRegisterFdParams {
    pub ctl_fd: i32,
}

/// NV0080 device allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct Nv0080AllocParams {
    pub h_client_share: u32,
    pub h_target_client: u32,
    pub h_target_device: u32,
    pub flags: u32,
    pub va_space_size: u64,
    pub va_start_internal: u64,
    pub va_limit_internal: u64,
    pub va_mode: u32,
    pub device_id: u32,
}

/// NV2080 subdevice allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct Nv2080AllocParams {
    pub sub_device_id: u32,
}

/// Fermi VA space allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvVaspaceAllocParams {
    pub index: u32,
    pub flags: u32,
    pub va_size: u64,
    pub va_start_internal: u64,
    pub va_limit_internal: u64,
    pub big_page_size: u32,
    pub pad: u32,
    pub va_base: u64,
}

/// Memory descriptor embedded in channel alloc params.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvMemoryDescParams {
    pub base: u64,
    pub size: u64,
    pub address_space: u32,
    pub cache_attrib: u32,
}

/// Kepler channel group allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvChannelGroupAllocParams {
    pub h_object_error: u32,
    pub h_object_ecc_error: u32,
    pub h_vaspace: u32,
    pub engine_type: u32,
    pub b_is_calling_context_vgpu_plugin: u8,
    _pad0: [u8; 7],
    pub p_gpu_grp_info: u64,
}

/// Fermi context share allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvCtxShareAllocParams {
    pub h_vaspace: u32,
    pub flags: u32,
    pub h_subdevice: u32,
}

/// Channel GPFIFO allocation parameters (Volta+).
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct NvChannelAllocParams {
    pub h_object_error: u32,
    pub h_object_buffer: u32,
    pub gpfifo_offset: u64,
    pub gpfifo_entries: u32,
    pub flags: u32,
    pub h_context_share: u32,
    pub h_vaspace: u32,
    pub h_userd_memory: [u32; NV_MAX_SUBDEVICES],
    pub userd_offset: [u64; NV_MAX_SUBDEVICES],
    pub engine_type: u32,
    pub cid: u32,
    pub sub_device_id: u32,
    pub h_object_ecc_error: u32,
    pub instance_mem: NvMemoryDescParams,
    pub userd_mem: NvMemoryDescParams,
    pub ramfc_mem: NvMemoryDescParams,
    pub mthdbuf_mem: NvMemoryDescParams,
    pub h_phys_channel_group: u32,
    pub internal_flags: u32,
    pub error_notifier_mem: NvMemoryDescParams,
    pub ecc_error_notifier_mem: NvMemoryDescParams,
    pub process_id: u32,
    pub sub_process_id: u32,
    pub encrypt_iv: [u32; 3],
    pub decrypt_iv: [u32; 3],
    pub hmac_nonce: [u32; 8],
    pub tpc_config_id: u32,
}

impl Default for NvChannelAllocParams {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// NV01_MEMORY_* allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvMemoryAllocParams {
    pub owner: u32,
    pub mem_type: u32,
    pub flags: u32,
    _reserved0: u32,
    _reserved1: u64,
    pub attr: u32,
    pub attr2: u32,
    pub format: u32,
    _reserved2: [u32; 7],
    pub size: u64,
    pub alignment: u64,
    pub offset: u64,
    pub limit: u64,
    _tail: [u64; 4],
}

/// NV01_MEMORY_VIRTUAL allocation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvMemoryVirtualAllocParams {
    pub offset: u64,
    pub limit: u64,
    pub h_vaspace: u32,
}

// ── RM control-specific payloads ────────────────────────────────────────

/// `NV2080_CTRL_CMD_GPU_GET_GID_INFO` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct Nv2080GpuGetGidInfoParams {
    pub index: u32,
    pub flags: u32,
    pub length: u32,
    pub data: [u8; 256],
}

impl Default for Nv2080GpuGetGidInfoParams {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `NVA06F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvA06fGetWorkSubmitTokenParams {
    pub work_submit_token: u32,
}

/// `NV2080_CTRL_CMD_GR_CTXSW_SETUP_BIND` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvGrCtxswSetupBindParams {
    pub h_client: u32,
    pub h_channel: u32,
    pub v_mem_ptr: u64,
}

/// `NV003E_CTRL_CMD_GET_SURFACE_PHYS_ATTR` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvGetSurfacePhysAttrParams {
    pub mem_offset: u64,
    pub mem_format: u32,
    pub compr_offset: u32,
    pub compr_format: u32,
    pub gpu_cache_attr: u32,
    pub gpu_p2p_cache_attr: u32,
    pub mmu_context: u32,
    pub contig_segment_size: u64,
}

/// Single entry in `NV2080_CTRL_CMD_GPU_PROMOTE_CTX`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvPromoteCtxBufferEntry {
    pub gpu_phys_addr: u64,
    pub gpu_virt_addr: u64,
    pub size: u64,
    pub phys_attr: u32,
    pub buffer_id: u16,
    pub b_initialize: u8,
    pub b_nonmapped: u8,
}

const _: () = assert!(core::mem::size_of::<NvPromoteCtxBufferEntry>() == 32);

/// `NV2080_CTRL_CMD_GPU_PROMOTE_CTX` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct NvGpuPromoteCtxParams {
    pub engine_type: u32,
    pub h_client: u32,
    pub ch_id: u32,
    pub h_chan_client: u32,
    pub h_object: u32,
    pub h_virt_memory: u32,
    pub virt_address: u64,
    pub size: u64,
    pub entry_count: u32,
    _pad: u32,
    pub promote_entry: [NvPromoteCtxBufferEntry; GPU_PROMOTE_CONTEXT_MAX_ENTRIES],
}

impl Default for NvGpuPromoteCtxParams {
    fn default() -> Self {
        Self::zeroed()
    }
}

const _: () = assert!(core::mem::size_of::<NvGpuPromoteCtxParams>() == 560);

/// Per-engine context buffer size/alignment info.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvEngineContextBufferInfo {
    pub size: u32,
    pub alignment: u32,
}

/// GR context buffer table for one engine.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct NvGrContextBuffersInfo {
    pub engine: [NvEngineContextBufferInfo; ENGINE_CONTEXT_PROPERTIES_ENGINE_ID_COUNT],
}

impl Default for NvGrContextBuffersInfo {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `NV2080_CTRL_CMD_INTERNAL_STATIC_KGR_GET_CONTEXT_BUFFERS_INFO` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable)]
pub struct NvGetContextBuffersInfoParams {
    pub engine_context_buffers_info: [NvGrContextBuffersInfo; INTERNAL_GR_MAX_ENGINES],
}

impl Default for NvGetContextBuffersInfoParams {
    fn default() -> Self {
        Self::zeroed()
    }
}

/// `NV906F_CTRL_CMD_BIND` parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvChannelBindParams {
    pub h_engine_object: u32,
    pub engine_class_1: u32,
    pub engine_class_2: u32,
    pub engine_type: u32,
}

/// RM map-memory ioctl parameters (`NV_ESC_RM_MAP_MEMORY`).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Zeroable)]
pub struct NvRmMapMemoryParams {
    pub h_client: u32,
    pub h_device: u32,
    pub h_memory: u32,
    pub pad: u32,
    pub offset: u64,
    pub length: u64,
    /// Out: mapped address.
    pub p_linear_address: u64,
    pub status: u32,
    pub flags: u32,
    /// Which fd to map on (-1 = nvidiactl).
    pub fd: i32,
    pub pad2: u32,
}
