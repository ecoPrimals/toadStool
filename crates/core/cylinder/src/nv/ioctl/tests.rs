// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from mod.rs (S334).

use super::*;

#[test]
fn ioctl_numbers_match_kernel_header() {
    assert_eq!(
        DRM_NOUVEAU_CHANNEL_ALLOC, 0x42,
        "CHANNEL_ALLOC = DRM_COMMAND_BASE + 0x02"
    );
    assert_eq!(
        DRM_NOUVEAU_CHANNEL_FREE, 0x43,
        "CHANNEL_FREE = DRM_COMMAND_BASE + 0x03"
    );
    assert_eq!(
        DRM_NOUVEAU_GEM_NEW, 0x80,
        "GEM_NEW = DRM_COMMAND_BASE + 0x40"
    );
    assert_eq!(
        DRM_NOUVEAU_GEM_PUSHBUF, 0x81,
        "GEM_PUSHBUF = DRM_COMMAND_BASE + 0x41"
    );
    assert_eq!(
        DRM_NOUVEAU_GEM_CPU_PREP, 0x82,
        "GEM_CPU_PREP = DRM_COMMAND_BASE + 0x42"
    );
    assert_eq!(
        DRM_NOUVEAU_VM_INIT, 0x50,
        "VM_INIT = DRM_COMMAND_BASE + 0x10"
    );
    assert_eq!(
        DRM_NOUVEAU_VM_BIND, 0x51,
        "VM_BIND = DRM_COMMAND_BASE + 0x11"
    );
    assert_eq!(DRM_NOUVEAU_EXEC, 0x52, "EXEC = DRM_COMMAND_BASE + 0x12");
}

#[test]
fn gem_domain_flags() {
    assert_eq!(_NOUVEAU_GEM_DOMAIN_CPU, 1);
    assert_eq!(NOUVEAU_GEM_DOMAIN_VRAM, 2);
    assert_eq!(NOUVEAU_GEM_DOMAIN_GART, 4);
    assert_eq!(
        NOUVEAU_GEM_DOMAIN_MAPPABLE, 8,
        "MAPPABLE = (1 << 3) per kernel header"
    );
}

#[test]
fn struct_sizes_are_reasonable() {
    assert!(std::mem::size_of::<NouveauChannelAlloc>() > 0);
}

#[test]
fn nvif_constants_match_mesa() {
    assert_eq!(NVIF_ROUTE_NVIF, 0x00);
    assert_eq!(NVIF_ROUTE_HIDDEN, 0xFF);
    assert_eq!(NVIF_OWNER_NVIF, 0x00);
    assert_eq!(NVIF_OWNER_ANY, 0xFF);
}

#[test]
fn nvif_compute_class_definitions() {
    assert_eq!(NVIF_CLASS_FERMI_TWOD_A, 0x902D);
    assert_eq!(NVIF_CLASS_KEPLER_INLINE_TO_MEMORY_B, 0xA0B5);
    assert_eq!(NVIF_CLASS_VOLTA_COMPUTE_A, 0xC3C0);
    assert_eq!(NVIF_CLASS_TURING_COMPUTE_A, 0xC5C0);
    assert_eq!(NVIF_CLASS_AMPERE_COMPUTE_A, 0xC6C0);
}

#[test]
fn subchan_spec_layout() {
    let s = SubchanSpec {
        handle: 1,
        grclass: NVIF_CLASS_VOLTA_COMPUTE_A,
    };
    assert_eq!(s.handle, 1);
    assert_eq!(s.grclass, 0xC3C0);
}

#[test]
fn channel_alloc_struct_has_subchan_array() {
    let alloc = NouveauChannelAlloc::default();
    assert_eq!(alloc.subchan.len(), 8);
}

#[test]
fn channel_alloc_struct_size_matches_kernel_abi() {
    // NouveauChannelAlloc (kernel drm_nouveau_channel_alloc):
    //   fb_ctxdma_handle: u32 (4)
    //   tt_ctxdma_handle: u32 (4)
    //   channel: i32 (4)
    //   pushbuf_domains: u32 (4)
    //   notifier_handle: u32 (4)
    //   subchan: [NouveauSubchan; 8] = 8 * 8 = 64
    //   nr_subchan: u32 (4)
    //   Total: 88 bytes (20 header + 64 subchan + 4 trailer)
    assert_eq!(
        std::mem::size_of::<NouveauChannelAlloc>(),
        88,
        "NouveauChannelAlloc must match kernel drm_nouveau_channel_alloc (88 bytes)"
    );
}

#[test]
fn channel_free_struct_size() {
    assert_eq!(
        std::mem::size_of::<NouveauChannelFree>(),
        4,
        "NouveauChannelFree must match kernel drm_nouveau_channel_free (4 bytes)"
    );
}

#[test]
fn nouveau_subchan_struct_size() {
    assert_eq!(
        std::mem::size_of::<NouveauSubchan>(),
        8,
        "NouveauSubchan must be 8 bytes (handle + grclass)"
    );
}

#[test]
fn dump_channel_alloc_hex_is_nonempty() {
    let hex = dump_channel_alloc_hex(NVIF_CLASS_VOLTA_COMPUTE_A);
    assert!(hex.contains("NouveauChannelAlloc"));
    assert!(hex.contains("bytes"));
}

#[test]
fn ioctl_uses_drm_iowr_pub() {
    use crate::drm;
    let nr = drm::drm_iowr_pub(
        DRM_NOUVEAU_CHANNEL_ALLOC,
        size_of_u32::<NouveauChannelAlloc>(),
    );
    assert!(nr > 0);
    assert_eq!(nr & 0xFF, 0x42, "encoded NR field = CHANNEL_ALLOC = 0x42");
}

#[test]
#[expect(clippy::cast_possible_truncation, reason = "test structs are small")]
fn size_of_u32_matches_struct_sizes() {
    assert_eq!(
        size_of_u32::<NouveauChannelAlloc>(),
        std::mem::size_of::<NouveauChannelAlloc>() as u32
    );
}
