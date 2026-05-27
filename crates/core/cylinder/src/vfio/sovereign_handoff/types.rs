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
    /// PRI ring anchor created from boot service evidence. Tracks PRI ring
    /// health across the driver swap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pri_ring_anchor: Option<PriRingAnchor>,
    /// Total wall-clock time in milliseconds.
    pub total_ms: u64,
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
