// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::nv::registers::pmc::InterruptProfile;

use super::{
    BootStrategy, CE0_BASE, CompletionStrategy, FECS_PC, GenerationProfile, GPC_BROADCAST,
    InstanceBlockFormat, LaunchMethod, LOCAL_MEM_WINDOW_LEGACY, MemoryType, NctaidSource,
    PageTableFormat, PGRAPH_STATUS, PowerSafetyProfile, PTOP_DEVICE_INFO, QmdVersion,
    RUNLIST_PBDMA_MAP, RunlistFormat,
};

/// Kepler (GK110/GK210) — Tesla K40, Tesla K80.
pub const KEPLER: GenerationProfile = GenerationProfile {
    name: "Kepler",
    sm_range: 35..=37,
    qmd_version: QmdVersion::V21,
    qmd_word_count: 64,
    channel_class: 0xA06F, // KEPLER_CHANNEL_GPFIFO_A
    compute_class: 0xA1C0, // KEPLER_COMPUTE_B
    launch_method: LaunchMethod::Pcas,
    local_mem_window: LOCAL_MEM_WINDOW_LEGACY,
    completion: CompletionStrategy::GpGetPoll,
    boot_strategy: BootStrategy::NoAcr,
    memory_type: MemoryType::Gddr5,
    has_hardware_f64_rcp: true,
    nctaid_source: NctaidSource::SystemRegister,
    userd_gp_get: true,
    firmware_chip: "gk210",
    has_full_rate_fp64: true,
    recommended_workgroup_size: 128,
    max_cta_per_sm: 32,
    page_table_format: PageTableFormat::V1TwoLevel,
    instance_block_format: InstanceBlockFormat::Simple,
    runlist_format: RunlistFormat::Gk104Global,
    power_safety: PowerSafetyProfile::PRE_FIRMWARE,
    fecs_pc_offset: FECS_PC,
    gpc_broadcast_offset: GPC_BROADCAST,
    ce0_base_offset: CE0_BASE,
    pgraph_status_offset: PGRAPH_STATUS,
    ce_class: 0xA0B5, // KEPLER_DMA_COPY_A
    interrupt_profile: InterruptProfile::PRE_VOLTA,
    ptop_device_info_base: PTOP_DEVICE_INFO,
    runlist_pbdma_map_base: RUNLIST_PBDMA_MAP,
};
