// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GenerationProfile, GPC_BROADCAST,
    InstanceBlockFormat, LaunchMethod, LOCAL_MEM_WINDOW_VOLTA, MemoryType, NctaidSource,
    PageTableFormat, PGRAPH_STATUS, PowerSafetyProfile, PTOP_DEVICE_INFO, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Volta (GV100) — Titan V, Tesla V100.
pub const VOLTA: GenerationProfile = GenerationProfile {
    name: "Volta",
    sm_range: 70..=74,
    qmd_version: QmdVersion::V22,
    qmd_word_count: 64,
    channel_class: 0xC36F, // VOLTA_CHANNEL_GPFIFO_A
    compute_class: 0xC3C0, // VOLTA_COMPUTE_A
    launch_method: LaunchMethod::Pcas,
    local_mem_window: LOCAL_MEM_WINDOW_VOLTA,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::AcrSec2,
    memory_type: MemoryType::Hbm2,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "gv100",
    has_full_rate_fp64: true,
    recommended_workgroup_size: 128,
    max_cta_per_sm: 32,
    page_table_format: PageTableFormat::V2FiveLevel,
    instance_block_format: InstanceBlockFormat::Subcontexted,
    runlist_format: RunlistFormat::Gv100PerRunlist,
    power_safety: PowerSafetyProfile::FIRMWARE_MANAGED,
    fecs_pc_offset: FECS_PC,
    gpc_broadcast_offset: GPC_BROADCAST,
    ce0_base_offset: CE0_BASE,
    pgraph_status_offset: PGRAPH_STATUS,
    ce_class: 0xC3B5, // VOLTA_DMA_COPY_A
    interrupt_profile: InterruptProfile::VOLTA_PLUS,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
