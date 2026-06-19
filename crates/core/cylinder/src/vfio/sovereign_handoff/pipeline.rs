// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::{Duration, Instant};

use crate::nv::registers::{falcon, gpc, pmc};
use crate::vfio::guarded_sysfs;
use crate::vfio::module_patch::ModulePatchResult;
use crate::vfio::sovereign_tiers::TierEvidence;
use toadstool_ember::pri_ring_anchor::{BootServiceEvidence, PriRingAnchor, PriRingHealth};

use super::lock::HandoffGuard;
use super::steps;
use super::types::{
    HandoffCapabilityProfile, HandoffConfig, HandoffResult, HandoffStep, RmChannelEvidence,
};

/// Shared mutable state for the sovereign handoff pipeline steps.
pub(crate) struct PipelineContext<'a> {
    pub config: &'a HandoffConfig,
    pub bar0: Option<&'a crate::vfio::device::MappedBar>,
    pub hw: HandoffCapabilityProfile,
    pub overall: Instant,
    pub deadline: Duration,
    pub steps: Vec<HandoffStep>,
    pub module_loaded: bool,
    pub patch_result: Option<ModulePatchResult>,
    pub sibling_state: Vec<(String, Option<String>)>,
    pub is_catalyst: bool,
    pub catalyst_snapshot_path: Option<String>,
    pub catalyst_alive_count: Option<usize>,
    pub catalyst_tier: Option<TierEvidence>,
    pub boot_evidence: Option<BootServiceEvidence>,
    pub rm_channel_evidence: Option<RmChannelEvidence>,
    pub needs_device_rollback: bool,
    pub module_unloaded: bool,
    pub tier: Option<TierEvidence>,
    pub override_path: String,
    pub probe_path: String,
    pub handoff_guard: Option<HandoffGuard>,
    pub irq_clutch_engaged: bool,
    heartbeat_fn: Option<&'a (dyn Fn() + Send)>,
    signal_fn: Option<&'a (dyn Fn(PipelineSignal) + Send)>,
}

impl PipelineContext<'_> {
    pub fn heartbeat(&self) {
        if let Some(f) = self.heartbeat_fn {
            f();
        }
    }

    pub fn signal(&self, s: PipelineSignal) {
        if let Some(f) = self.signal_fn {
            f(s);
        }
    }
}

/// This is the top-level entry point called from the dispatch handler.
pub fn execute_handoff(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
) -> HandoffResult {
    execute_handoff_inner(config, bar0, None, None)
}

/// Lifecycle signal from the handoff pipeline to the watchdog.
#[derive(Debug, Clone)]
pub enum PipelineSignal {
    EnterModuleCleanup,
    ExitModuleCleanup,
}

pub fn execute_handoff_with_heartbeat(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
    heartbeat_fn: impl Fn() + Send + 'static,
) -> HandoffResult {
    execute_handoff_inner(config, bar0, Some(Box::new(heartbeat_fn)), None)
}

pub fn execute_handoff_with_signals(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
    heartbeat_fn: impl Fn() + Send + 'static,
    signal_fn: impl Fn(PipelineSignal) + Send + 'static,
) -> HandoffResult {
    execute_handoff_inner(
        config,
        bar0,
        Some(Box::new(heartbeat_fn)),
        Some(Box::new(signal_fn)),
    )
}

fn execute_handoff_inner(
    config: &HandoffConfig,
    bar0: Option<&crate::vfio::device::MappedBar>,
    heartbeat_fn: Option<Box<dyn Fn() + Send>>,
    signal_fn: Option<Box<dyn Fn(PipelineSignal) + Send>>,
) -> HandoffResult {
    let overall = Instant::now();
    let deadline = guarded_sysfs::HANDOFF_DEADLINE;
    let hw = HandoffCapabilityProfile::for_sm(config.sm_version.unwrap_or(70));
    let heartbeat_ref = heartbeat_fn.as_deref();
    let signal_ref = signal_fn.as_deref();

    let mut ctx = PipelineContext {
        config,
        bar0,
        hw,
        overall,
        deadline,
        steps: Vec::new(),
        module_loaded: false,
        patch_result: None,
        sibling_state: Vec::new(),
        is_catalyst: false,
        catalyst_snapshot_path: None,
        catalyst_alive_count: None,
        catalyst_tier: None,
        boot_evidence: None,
        rm_channel_evidence: None,
        needs_device_rollback: false,
        module_unloaded: false,
        tier: None,
        override_path: String::new(),
        probe_path: String::new(),
        handoff_guard: None,
        irq_clutch_engaged: false,
        heartbeat_fn: heartbeat_ref,
        signal_fn: signal_ref,
    };

    // Forensic breadcrumb helper — writes timestamped markers to a file
    // that survives soft lockups. Check /var/log/handoff-forensics.log after reboot.
    fn crumb(msg: &str) {
        crate::vfio::sovereign_handoff::forensics::breadcrumb(&format!("PIPELINE: {msg}"));
    }

    crumb(&format!(
        "=== HANDOFF START bdf={} strategy={} ===",
        config.bdf, config.seeder_driver
    ));

    ctx.heartbeat();
    crumb("preflight");
    if let Some(result) = steps::preflight::run(&mut ctx) {
        return result;
    }

    ctx.heartbeat();
    crumb("module_prep");
    if let Some(result) = steps::module_prep::run(&mut ctx) {
        return result;
    }

    ctx.heartbeat();
    crumb("unbind_bind");
    if let Some(result) = steps::unbind_bind::run(&mut ctx) {
        return result;
    }

    ctx.heartbeat();
    crumb("rm_trigger");
    steps::rm_trigger::run(&mut ctx);

    ctx.heartbeat();
    crumb("settle_capture");
    if let Some(result) = steps::settle_capture::run(&mut ctx) {
        return result;
    }

    ctx.heartbeat();
    crumb("warm_swap");
    if let Some(result) = steps::warm_swap::run(&mut ctx) {
        return result;
    }

    crumb("recovery");
    steps::recovery::run(&mut ctx);

    ctx.heartbeat();
    crumb("classify_preserve");
    steps::classify_preserve::run(&mut ctx);

    ctx.heartbeat();
    crumb("cleanup");
    steps::cleanup::run(&mut ctx);
    crumb("=== HANDOFF COMPLETE ===");

    let pri_ring_anchor = ctx.boot_evidence.as_ref().map(|ev| {
        let mut anchor = PriRingAnchor::from_evidence(&ctx.config.bdf, ev.clone());
        let health = if let Ok(bar0) =
            crate::vfio::device::MappedBar::from_sysfs_rw(&ctx.config.bdf, 16 * 1024 * 1024)
        {
            let pmc = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
            let fecs = bar0
                .read_u32((falcon::FECS_BASE + falcon::CPUCTL) as usize)
                .unwrap_or(0xDEAD);
            let pgraph_on = pmc & (1 << 12) != 0;
            let fecs_ok = fecs & 0xBADF_0000 != 0xBADF_0000;
            let tpc0 = bar0.read_u32(gpc::gpc_tpc0(0) as usize).unwrap_or(0xBADF);
            let tpc_ok = tpc0 & 0xBADF_0000 != 0xBADF_0000;
            if pgraph_on && fecs_ok && tpc_ok {
                PriRingHealth::Healthy
            } else if pgraph_on && fecs_ok {
                PriRingHealth::Degraded { faulted_domains: 1 }
            } else {
                PriRingHealth::Destroyed
            }
        } else {
            PriRingHealth::Destroyed
        };
        anchor.update_health(health);
        tracing::info!(
            bdf = ctx.config.bdf.as_str(),
            health = ?anchor.health,
            compute_ready = anchor.is_compute_ready(),
            needs_reboot = anchor.needs_reboot(),
            "PRI ring anchor created from post-recovery state"
        );
        anchor
    });

    HandoffResult {
        bdf: ctx.config.bdf.clone(),
        success: true,
        halted_at: None,
        steps: ctx.steps,
        patch_result: ctx.patch_result,
        tier: ctx.tier,
        module_loaded: ctx.module_loaded,
        module_unloaded: ctx.module_unloaded,
        catalyst_snapshot_path: ctx.catalyst_snapshot_path,
        catalyst_alive_count: ctx.catalyst_alive_count,
        catalyst_tier: ctx.catalyst_tier,
        rm_channel_evidence: ctx.rm_channel_evidence,
        boot_service_evidence: ctx.boot_evidence,
        pri_ring_anchor,
        total_ms: ctx.overall.elapsed().as_millis() as u64,
    }
}
