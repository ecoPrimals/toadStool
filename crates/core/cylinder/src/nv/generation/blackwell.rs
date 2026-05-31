// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GenerationProfile, GPC_BROADCAST,
    InstanceBlockFormat, LaunchMethod, LOCAL_MEM_WINDOW_VOLTA, MemoryType, NctaidSource,
    PageTableFormat, PGRAPH_STATUS, PowerSafetyProfile, PTOP_DEVICE_INFO, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Blackwell A (GB100/GB102) — B100, B200 datacenter.
pub const BLACKWELL_A: GenerationProfile = GenerationProfile {
    name: "Blackwell A",
    sm_range: 100..=119,
    qmd_version: QmdVersion::V50,
    qmd_word_count: 96,
    channel_class: 0xC96F, // BLACKWELL_CHANNEL_GPFIFO_A
    compute_class: 0xCDC0, // BLACKWELL_COMPUTE_A
    launch_method: LaunchMethod::Pcas2,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::SemaphoreFence,
    boot_strategy: BootStrategy::KmodPromote,
    memory_type: MemoryType::Hbm2,
    has_hardware_f64_rcp: false,
    nctaid_source: NctaidSource::DriverCbuf7,
    userd_gp_get: false,
    firmware_chip: "gb100",
    has_full_rate_fp64: true,
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
    ce_class: 0xC8B5, // BLACKWELL_DMA_COPY_A (provisional)
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};

/// Blackwell B (GB202/GB203/GB205/GB206/GB207) — RTX 5090, RTX 5080, RTX 5060, etc.
pub const BLACKWELL_B: GenerationProfile = GenerationProfile {
    name: "Blackwell B",
    sm_range: 120..=u32::MAX,
    qmd_version: QmdVersion::V50,
    qmd_word_count: 96,
    channel_class: 0xC96F, // BLACKWELL_CHANNEL_GPFIFO_A (CUDA R580 trace: 0xC96F for all Blackwell)
    compute_class: 0xCEC0, // BLACKWELL_COMPUTE_B
    launch_method: LaunchMethod::Pcas2,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::SemaphoreFence,
    boot_strategy: BootStrategy::KmodPromote,
    memory_type: MemoryType::Gddr7,
    has_hardware_f64_rcp: false,
    nctaid_source: NctaidSource::DriverCbuf7,
    userd_gp_get: false,
    firmware_chip: "gb202",
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
    ce_class: 0xC8B5, // BLACKWELL_DMA_COPY_B (provisional)
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
