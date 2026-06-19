// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GPC_BROADCAST, GenerationProfile,
    InstanceBlockFormat, LOCAL_MEM_WINDOW_VOLTA, LaunchMethod, MemoryType, NctaidSource,
    PGRAPH_STATUS, PTOP_DEVICE_INFO, PageTableFormat, PowerSafetyProfile, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Ada Lovelace (AD102/AD103/AD104) — RTX 4090, RTX 4080, etc.
pub const ADA: GenerationProfile = GenerationProfile {
    name: "Ada",
    sm_range: 89..=89,
    qmd_version: QmdVersion::V30,
    qmd_word_count: 64,
    channel_class: 0xC56F, // AMPERE_CHANNEL_GPFIFO_A (shared with Ampere)
    compute_class: 0xC9C0, // ADA_COMPUTE_A
    launch_method: LaunchMethod::Pcas2,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::AcrSec2,
    memory_type: MemoryType::Gddr6x,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "ad102",
    has_full_rate_fp64: false,
    recommended_workgroup_size: 256,
    max_cta_per_sm: 16,
    page_table_format: PageTableFormat::V2FiveLevel,
    instance_block_format: InstanceBlockFormat::Subcontexted,
    runlist_format: RunlistFormat::Gv100PerRunlist,
    power_safety: PowerSafetyProfile::FIRMWARE_MANAGED,
    fecs_pc_offset: FECS_PC,
    gpc_broadcast_offset: GPC_BROADCAST,
    ce0_base_offset: CE0_BASE,
    pgraph_status_offset: PGRAPH_STATUS,
    ce_class: 0xC8B5, // ADA_DMA_COPY_A
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
