// SPDX-License-Identifier: AGPL-3.0-or-later
//! AMD-specific DRM ioctl definitions.
//!
//! Structures and constants from the amdgpu kernel driver, defined in
//! pure Rust (no `amdgpu-sys` or `drm-sys`).

mod ops;
mod types;

pub use ops::{
    create_bo_list, create_context, destroy_bo_list, destroy_context, gem_create, gem_mmap_offset,
    gem_va_map, query_gfx_version, submit_command, submit_command_ip, sync_fence, sync_fence_ip,
};
pub use types::{
    AMDGPU_GEM_CREATE_CPU_GTT_USWC, AMDGPU_GEM_DOMAIN_GTT, AMDGPU_GEM_DOMAIN_VRAM,
    AMDGPU_HW_IP_COMPUTE, AMDGPU_HW_IP_GFX, AMDGPU_VA_FLAGS_NONE, AMDGPU_VA_OP_MAP,
    AMDGPU_VA_OP_UNMAP, AMDGPU_VM_PAGE_EXECUTABLE, AMDGPU_VM_PAGE_READABLE,
    AMDGPU_VM_PAGE_WRITEABLE, AmdgpuCtx, AmdgpuGemCreate, AmdgpuGemMmap, AmdgpuGemVa,
};

#[cfg(test)]
mod tests {
    use super::types::*;
    use std::mem::{offset_of, size_of};

    #[test]
    fn gem_create_layout() {
        assert_eq!(size_of::<AmdgpuGemCreate>(), 32);
        assert_eq!(offset_of!(AmdgpuGemCreate, bo_size), 0);
        assert_eq!(offset_of!(AmdgpuGemCreate, alignment), 8);
        assert_eq!(offset_of!(AmdgpuGemCreate, domains), 16);
        assert_eq!(offset_of!(AmdgpuGemCreate, domain_flags), 24);
    }

    #[test]
    fn gem_mmap_layout() {
        assert_eq!(size_of::<AmdgpuGemMmap>(), 8);
        assert_eq!(offset_of!(AmdgpuGemMmap, handle_or_addr), 0);
    }

    #[test]
    fn ctx_layout() {
        assert_eq!(size_of::<AmdgpuCtx>(), 16);
        assert_eq!(offset_of!(AmdgpuCtx, op), 0);
        assert_eq!(offset_of!(AmdgpuCtx, flags), 4);
        assert_eq!(offset_of!(AmdgpuCtx, ctx_id), 8);
    }

    #[test]
    fn gem_va_layout() {
        assert_eq!(size_of::<AmdgpuGemVa>(), 40);
        assert_eq!(offset_of!(AmdgpuGemVa, handle), 0);
        assert_eq!(offset_of!(AmdgpuGemVa, operation), 8);
        assert_eq!(offset_of!(AmdgpuGemVa, flags), 12);
        assert_eq!(offset_of!(AmdgpuGemVa, va_address), 16);
        assert_eq!(offset_of!(AmdgpuGemVa, offset_in_bo), 24);
        assert_eq!(offset_of!(AmdgpuGemVa, map_size), 32);
    }

    #[test]
    fn bo_list_entry_layout() {
        assert_eq!(size_of::<AmdgpuBoListEntry>(), 8);
        assert_eq!(offset_of!(AmdgpuBoListEntry, bo_handle), 0);
        assert_eq!(offset_of!(AmdgpuBoListEntry, bo_priority), 4);
    }

    #[test]
    fn bo_list_in_layout() {
        assert_eq!(size_of::<AmdgpuBoListIn>(), 24);
        assert_eq!(offset_of!(AmdgpuBoListIn, operation), 0);
        assert_eq!(offset_of!(AmdgpuBoListIn, list_handle), 4);
        assert_eq!(offset_of!(AmdgpuBoListIn, bo_number), 8);
        assert_eq!(offset_of!(AmdgpuBoListIn, bo_info_size), 12);
        assert_eq!(offset_of!(AmdgpuBoListIn, bo_info_ptr), 16);
    }

    #[test]
    fn cs_chunk_layout() {
        assert_eq!(size_of::<AmdgpuCsChunk>(), 16);
        assert_eq!(offset_of!(AmdgpuCsChunk, chunk_id), 0);
        assert_eq!(offset_of!(AmdgpuCsChunk, length_dw), 4);
        assert_eq!(offset_of!(AmdgpuCsChunk, chunk_data), 8);
    }

    #[test]
    fn cs_chunk_ib_layout() {
        assert_eq!(size_of::<AmdgpuCsChunkIb>(), 32);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, pad), 0);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, flags), 4);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, va_start), 8);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, ib_bytes), 16);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, ip_type), 20);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, ip_instance), 24);
        assert_eq!(offset_of!(AmdgpuCsChunkIb, ring), 28);
    }

    #[test]
    fn cs_in_layout() {
        assert_eq!(size_of::<AmdgpuCsIn>(), 24);
        assert_eq!(offset_of!(AmdgpuCsIn, ctx_id), 0);
        assert_eq!(offset_of!(AmdgpuCsIn, bo_list_handle), 4);
        assert_eq!(offset_of!(AmdgpuCsIn, num_chunks), 8);
        assert_eq!(offset_of!(AmdgpuCsIn, flags), 12);
        assert_eq!(offset_of!(AmdgpuCsIn, chunks), 16);
    }

    #[test]
    fn wait_cs_in_layout() {
        assert_eq!(size_of::<AmdgpuWaitCsIn>(), 32);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, handle), 0);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, timeout), 8);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, ip_type), 16);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, ip_instance), 20);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, ring), 24);
        assert_eq!(offset_of!(AmdgpuWaitCsIn, ctx_id), 28);
    }

    #[test]
    fn default_structs_are_zeroed() {
        let gem = AmdgpuGemCreate::default();
        assert_eq!(gem.bo_size, 0);
        assert_eq!(gem.domains, 0);

        let ctx = AmdgpuCtx::default();
        assert_eq!(ctx.op, 0);
        assert_eq!(ctx.ctx_id, 0);

        let wait = AmdgpuWaitCsIn::default();
        assert_eq!(wait.handle, 0);
        assert_eq!(wait.timeout, 0);
    }

    #[test]
    fn info_request_raw_layout() {
        assert_eq!(size_of::<AmdgpuInfoRequestRaw>(), 32);
        assert_eq!(offset_of!(AmdgpuInfoRequestRaw, return_pointer), 0);
        assert_eq!(offset_of!(AmdgpuInfoRequestRaw, return_size), 8);
        assert_eq!(offset_of!(AmdgpuInfoRequestRaw, query), 12);
        assert_eq!(offset_of!(AmdgpuInfoRequestRaw, hw_ip_type), 16);
        assert_eq!(offset_of!(AmdgpuInfoRequestRaw, hw_ip_instance), 20);
    }

    #[test]
    fn info_hw_ip_layout() {
        assert_eq!(size_of::<AmdgpuInfoHwIp>(), 32);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, hw_ip_version_major), 0);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, hw_ip_version_minor), 4);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, capabilities_flags), 8);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, ib_start_alignment), 16);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, ib_size_alignment), 20);
        assert_eq!(offset_of!(AmdgpuInfoHwIp, available_rings), 24);
    }
}
