// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GenerationProfile, GPC_BROADCAST,
    InstanceBlockFormat, LaunchMethod, LOCAL_MEM_WINDOW_VOLTA, MemoryType, NctaidSource,
    PageTableFormat, PGRAPH_STATUS, PowerSafetyProfile, PTOP_DEVICE_INFO, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Ampere A (GA100) — A100 datacenter.
pub const AMPERE_A: GenerationProfile = GenerationProfile {
    name: "Ampere A",
    sm_range: 80..=80,
    qmd_version: QmdVersion::V23,
    qmd_word_count: 64,
    channel_class: 0xC56F, // AMPERE_CHANNEL_GPFIFO_A
    compute_class: 0xC6C0, // AMPERE_COMPUTE_A
    launch_method: LaunchMethod::Pcas2,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::AcrSec2,
    memory_type: MemoryType::Hbm2,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "ga100",
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
    ce_class: 0xC6B5, // AMPERE_DMA_COPY_A
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};

/// Ampere B (GA102/GA104/GA106/GA107) — RTX 3090, RTX 3080, etc.
pub const AMPERE_B: GenerationProfile = GenerationProfile {
    name: "Ampere B",
    sm_range: 81..=88,
    qmd_version: QmdVersion::V23,
    qmd_word_count: 64,
    channel_class: 0xC56F, // AMPERE_CHANNEL_GPFIFO_A
    compute_class: 0xC7C0, // AMPERE_COMPUTE_B
    launch_method: LaunchMethod::Pcas2,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::AcrSec2,
    memory_type: MemoryType::Gddr6x,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "ga102",
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
    ce_class: 0xC7B5, // AMPERE_DMA_COPY_B
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
