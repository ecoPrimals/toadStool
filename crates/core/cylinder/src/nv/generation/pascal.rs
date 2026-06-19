// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GPC_BROADCAST, GenerationProfile,
    InstanceBlockFormat, LOCAL_MEM_WINDOW_LEGACY, LaunchMethod, MemoryType, NctaidSource,
    PGRAPH_STATUS, PTOP_DEVICE_INFO, PageTableFormat, PowerSafetyProfile, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Pascal (GP100/GP102) — GTX 1080, Tesla P100.
///
/// `has_full_rate_fp64` is true because this profile covers GP100 (1:2).
/// Consumer GP10x (1:32) shares SM 6.x but uses different device IDs.
pub const PASCAL: GenerationProfile = GenerationProfile {
    name: "Pascal",
    sm_range: 60..=62,
    qmd_version: QmdVersion::V21,
    qmd_word_count: 64,
    channel_class: 0xC06F, // PASCAL_CHANNEL_GPFIFO_A
    compute_class: 0xC0C0, // PASCAL_COMPUTE_A
    launch_method: LaunchMethod::Pcas,
    local_mem_window: LOCAL_MEM_WINDOW_LEGACY,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::AcrSec2,
    memory_type: MemoryType::Hbm2,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "gp100",
    has_full_rate_fp64: true,
    recommended_workgroup_size: 256,
    max_cta_per_sm: 32,
    page_table_format: PageTableFormat::V2FiveLevel,
    instance_block_format: InstanceBlockFormat::Simple,
    runlist_format: RunlistFormat::Gk104Global,
    power_safety: PowerSafetyProfile::FIRMWARE_MANAGED,
    fecs_pc_offset: FECS_PC,
    gpc_broadcast_offset: GPC_BROADCAST,
    ce0_base_offset: CE0_BASE,
    pgraph_status_offset: PGRAPH_STATUS,
    ce_class: 0xC0B5, // PASCAL_DMA_COPY_A
    interrupt_profile: InterruptProfile::PRE_VOLTA,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
