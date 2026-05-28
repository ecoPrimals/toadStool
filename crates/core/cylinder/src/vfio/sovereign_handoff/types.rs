// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::vfio::module_patch::ModulePatchResult;
use crate::vfio::sovereign_tiers::TierEvidence;
use toadstool_ember::pri_ring_anchor::{BootServiceEvidence, PriRingAnchor};

/// Configuration for a sovereign warm handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffConfig {
    /// Target PCI BDF (e.g., "0000:02:00.0").
    pub bdf: String,

    /// Seeder driver name for sysfs bind (e.g., "nouveau").
    pub seeder_driver: String,

    /// Kernel module name (e.g., "nouveau").
    pub module_name: String,

    /// Module source strategy.
    pub module_source: ModuleSourceConfig,

    /// How long to wait after seeder binds before warm-swapping.
    pub settle: Duration,

    /// Final driver target (e.g., "vfio-pci").
    pub final_driver: String,

    /// Optional JSON-serialized [`PatchSet`] override. When present, the
    /// pipeline uses this instead of resolving the patch set by name from
    /// [`ModuleSourceConfig`]. Enables runtime-defined patch sets via RPC.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_set_override: Option<String>,

    /// Whether to skip the preflight health check. Useful for experiments
    /// that intentionally operate outside normal safety bounds.
    #[serde(default)]
    pub skip_preflight: bool,

    /// GPU SM architecture version (e.g. 35 for Kepler, 70 for Volta, 120 for
    /// Blackwell). Drives generation-aware behavior: interrupt quench register
    /// selection, GPC topology, catalyst capture offsets, tier classification.
    /// When `None`, the pipeline detects SM from BOOT0 or defaults to 70 (Volta).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm_version: Option<u32>,
}

/// Generation-aware hardware profile for catalyst handoff.
///
/// Captures the register topology and thresholds that vary across GPU
/// generations, so the pipeline can perform catalyst capture, settle
/// diagnostics, PRI recovery, and tier classification without hardcoded
/// offsets. Built from `GenerationProfile` or auto-detected from BOOT0.
#[derive(Debug, Clone)]
pub struct HandoffCapabilityProfile {
    /// SM architecture version (e.g. 35, 70, 120).
    pub sm: u32,
    /// Number of GPCs on this chip (6 for GV100, 8 for GA100, etc.).
    pub gpc_count: u32,
    /// BAR0 base offset for TPC register probes (GPC0).
    pub tpc_base: u32,
    /// Stride between GPC instances in BAR0 address space.
    pub tpc_gpc_stride: u32,
    /// BAR0 base for PCCSR channel status scan.
    pub pccsr_base: u32,
    /// Number of PCCSR channel slots to scan.
    pub pccsr_channel_count: u32,
    /// FECS falcon base in BAR0.
    pub fecs_base: u32,
    /// GPCCS falcon base in BAR0.
    pub gpccs_base: u32,
    /// PMU falcon base in BAR0.
    pub pmu_base: u32,
    /// PMC_ENABLE popcount threshold for "warm GPU" heuristic.
    pub pmc_warm_threshold: u32,
    /// BAR0 domain map for catalyst capture (name, start, end).
    pub bar0_domains: &'static [(&'static str, usize, usize)],
    /// Interrupt register semantics for this generation.
    pub interrupt_profile: crate::nv::registers::pmc::InterruptProfile,
    /// Chip codename for firmware artifact naming (e.g. "gv100", "gk210").
    pub chip_name: &'static str,
}

impl HandoffCapabilityProfile {
    /// Build from SM version using `GenerationProfile` and register constants.
    #[must_use]
    pub fn for_sm(sm: u32) -> Self {
        use crate::nv::registers::falcon;

        let profile = crate::nv::generation::profile_for_sm(sm);
        let (gpc_count, chip_name) = Self::gpc_topology_for_sm(sm);

        Self {
            sm,
            gpc_count,
            tpc_base: 0x50_4000,
            tpc_gpc_stride: 0x8000,
            pccsr_base: 0x80_0004,
            pccsr_channel_count: 64,
            fecs_base: falcon::FECS_BASE,
            gpccs_base: falcon::GPCCS_BASE,
            pmu_base: falcon::PMU_BASE,
            pmc_warm_threshold: 10,
            bar0_domains: Self::domains_for_sm(sm),
            interrupt_profile: profile.interrupt_profile,
            chip_name,
        }
    }

    fn gpc_topology_for_sm(sm: u32) -> (u32, &'static str) {
        match sm {
            35..=37 => (2, "gk210"),    // K80: 2 GPCs per die (GK210)
            50..=52 => (4, "gm200"),    // GM200: 4 GPCs
            60..=62 => (6, "gp100"),    // GP100: 6 GPCs
            70..=74 => (6, "gv100"),    // GV100: 6 GPCs
            75..=79 => (6, "tu102"),    // TU102: 6 GPCs
            80..=87 => (8, "ga100"),    // GA100: 8 GPCs
            89      => (12, "ad102"),   // AD102: 12 GPCs
            90..=99 => (8, "gh100"),    // GH100: 8 GPCs (estimated)
            100..=120 => (12, "gb100"), // GB100: 12 GPCs (estimated)
            _ => (6, "gv100"),          // fallback
        }
    }

    fn domains_for_sm(_sm: u32) -> &'static [(&'static str, usize, usize)] {
        // All NVIDIA GPUs share the same major domain layout in BAR0.
        // The Volta domain map is the most comprehensive and works as a
        // superset for other generations (unmapped regions just read 0/fault).
        &crate::nv::pri::VOLTA_BAR0_DOMAINS
    }
}

/// Module source configuration (cylinder-side, no glowplug dependency).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleSourceConfig {
    /// Module already loaded or loadable via the system.
    System,
    /// Binary-patch a stock module before loading.
    Patched {
        /// Stock module name for `modinfo -n` lookup.
        stock_module: String,
        /// Patch set name (resolved by `PatchSet::by_name`).
        patch_set: String,
    },
    /// Binary-patch a DKMS-built module (specific version) before loading.
    /// Used when the system's installed module is a different version
    /// (e.g., nvidia-580-open installed, but we need nvidia-470 proprietary).
    DkmsPatched {
        /// Module name in DKMS (e.g., "nvidia").
        dkms_module: String,
        /// DKMS version string (e.g., "470.256.02").
        dkms_version: String,
        /// Patch set name.
        patch_set: String,
    },
}

/// Result of a sovereign warm handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    /// Target BDF.
    pub bdf: String,
    /// Whether the full pipeline succeeded.
    pub success: bool,
    /// Which step halted the pipeline (if any).
    pub halted_at: Option<String>,
    /// Per-step outcomes.
    pub steps: Vec<HandoffStep>,
    /// Module patch result (if patching was used).
    pub patch_result: Option<ModulePatchResult>,
    /// Tier classification after handoff (if we got far enough).
    pub tier: Option<TierEvidence>,
    /// Whether a module was loaded by this handoff.
    pub module_loaded: bool,
    /// Whether the module was successfully unloaded after handoff.
    pub module_unloaded: bool,
    /// Catalyst capture: BAR0 snapshot taken while the catalyst driver
    /// owned the GPU (between settle and warm swap). Present only for
    /// catalyst strategies. Persisted to disk as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalyst_snapshot_path: Option<String>,
    /// Catalyst capture: register count in the snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalyst_alive_count: Option<usize>,
    /// Catalyst capture: tier evidence from the pre-swap snapshot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalyst_tier: Option<TierEvidence>,
    /// Boot service evidence captured during ExitBootServices (UEFI model).
    /// Present when a catalyst/boot_services strategy runs and firmware was
    /// alive during the settle phase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_service_evidence: Option<BootServiceEvidence>,
    /// RM channel evidence captured during catalyst settle (Exp 229).
    /// Present when rm_trigger runs with --channel mode and creates a full
    /// RM compute channel before warm swap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rm_channel_evidence: Option<RmChannelEvidence>,
    /// PRI ring anchor created from boot service evidence. Tracks PRI ring
    /// health across the driver swap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pri_ring_anchor: Option<PriRingAnchor>,
    /// Total wall-clock time in milliseconds.
    pub total_ms: u64,
}

/// Evidence from a full RM compute channel created by rm_trigger --channel (Exp 229).
///
/// Captures the RM-allocated channel metadata before warm swap so the
/// post-swap sovereign path can either:
/// - Phase B: verify FECS ctx-switch readiness with its own channel
/// - Phase A: adopt the RM channel's hardware layout for dispatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RmChannelEvidence {
    /// RM-assigned channel ID (from NvChannelAllocParams.cid after alloc).
    pub channel_id: Option<u32>,
    /// Doorbell token for USERMODE register writes.
    pub work_submit_token: Option<u32>,
    /// Number of RM alloc/control steps that succeeded (out of 15).
    pub steps_completed: u16,
    /// Whether the entire channel creation sequence succeeded.
    pub all_ok: bool,
}

impl RmChannelEvidence {
    /// Parse from rm_trigger JSON output.
    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        if !json.get("channel_mode")?.as_bool()? {
            return None;
        }
        let channel_id = json.get("channel_id")?.as_u64().map(|v| v as u32);
        let work_submit_token = json
            .get("work_submit_token")
            .and_then(|v| v.as_str())
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok());
        let steps = json.get("steps")?.as_array()?;
        let steps_completed = steps
            .iter()
            .filter(|s| s.get("ok").and_then(|v| v.as_bool()).unwrap_or(false))
            .count() as u16;
        let all_ok = json.get("success")?.as_bool().unwrap_or(false);
        Some(Self { channel_id, work_submit_token, steps_completed, all_ok })
    }
}

/// One step in the handoff pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffStep {
    pub name: String,
    pub ok: bool,
    pub detail: Option<String>,
    pub duration_ms: u64,
}

impl std::fmt::Display for HandoffResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.success {
            let tier_str = self
                .tier
                .as_ref()
                .map(|t| format!(" → {}", t.tier))
                .unwrap_or_default();
            write!(
                f,
                "HANDOFF OK ({}{}, {}ms)",
                self.bdf, tier_str, self.total_ms
            )
        } else {
            write!(
                f,
                "HANDOFF HALTED@{} ({}, {}ms)",
                self.halted_at.as_deref().unwrap_or("?"),
                self.bdf,
                self.total_ms
            )
        }
    }
}
