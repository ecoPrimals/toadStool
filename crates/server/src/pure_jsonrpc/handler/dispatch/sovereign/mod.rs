// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign GPU handlers — ember-managed VFIO pipeline RPC methods.

mod catalyst_boot;
mod ce_validate;
mod init;
mod pmu_investigate;
mod profile;
mod warm_handoff;
mod warm_status;

use super::DispatchHandler;

impl DispatchHandler {
    /// `sovereign.init` via ember — runs the sovereign pipeline using
    /// the clutch (preferred) or cached device BAR0 + DMA.
    pub(crate) async fn sovereign_init_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        init::sovereign_init_ember(self, params).await
    }

    /// `sovereign.ce_validate` via ember — validates the sovereign DMA pipeline.
    pub(crate) async fn sovereign_ce_validate_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        ce_validate::sovereign_ce_validate_ember(self, params).await
    }

    /// `sovereign.pmu_investigate` — Exp 211 PMU mailbox investigation.
    pub(crate) async fn sovereign_pmu_investigate(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        pmu_investigate::sovereign_pmu_investigate(self, params).await
    }

    /// `sovereign.warm_handoff` — sovereign driver rotation pipeline.
    pub(crate) async fn sovereign_warm_handoff(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        warm_handoff::sovereign_warm_handoff(self, params).await
    }

    /// `sovereign.catalyst_boot` — catalyst-free boot pipeline.
    pub(crate) async fn sovereign_catalyst_boot(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        catalyst_boot::sovereign_catalyst_boot(self, params).await
    }

    /// `sovereign.profile` via ember — instrumented pipeline with microsecond timing.
    pub(crate) async fn sovereign_profile_ember(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        profile::sovereign_profile_ember(self, params).await
    }

    /// `sovereign.warm_status` — lightweight warm keepalive status for all known GPUs.
    pub(crate) async fn sovereign_warm_status(
        &self,
    ) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
        warm_status::sovereign_warm_status(self).await
    }
}

/// Probe boot state and sovereignty tier via sysfs BAR0.
/// Returns (state_name, pmc_hex, pramin_ok) or None on failure.
pub(super) fn probe_boot_state_sysfs(bdf: &str) -> Option<(String, String, bool)> {
    use toadstool_cylinder::vfio::device::MappedBar;
    use toadstool_cylinder::vfio::probe_boot_state;

    let bar = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024).ok()?;
    let state = probe_boot_state(&bar, None);
    let pmc = bar.read_u32(0x200).unwrap_or(0);
    let pramin_ok = state.is_warm();
    let state_name = if state.is_warm() { "warm" } else { "cold" };
    Some((state_name.to_string(), format!("0x{pmc:08x}"), pramin_ok))
}

/// Classify the sovereignty tier for a device via sysfs BAR0.
pub(super) fn classify_tier_sysfs(
    bdf: &str,
) -> Option<toadstool_cylinder::vfio::sovereign_tiers::TierEvidence> {
    use toadstool_cylinder::vfio::device::MappedBar;
    let bar = MappedBar::from_sysfs_rw(bdf, 16 * 1024 * 1024).ok()?;
    Some(toadstool_cylinder::vfio::sovereign_tiers::classify_tier(&bar))
}
