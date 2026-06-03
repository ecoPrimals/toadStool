// SPDX-License-Identifier: AGPL-3.0-or-later
//! Staged sovereign device initialization pipeline.
//!
//! Orchestrates the full path from cold/warm device to compute-ready state.
//! The pipeline is vendor/generation-agnostic — all hardware-specific
//! decisions are delegated to a [`SovereignStrategy`] implementation.
//!
//! # Stages
//!
//! ```text
//! 1.   bar0_probe          — Chip ID verification, PMC liveness check
//! 2.   pmc_enable          — Staged engine clock enable (strategy-aware mask)
//! 2b.  cg_sweep            — Clock gating disable across all domains (strategy)
//! 2a.  pgraph_reset        — PGRAPH engine reset (PMC_ENABLE bit 12 toggle)
//! 2c.  pri_recovery        — PRI bus fault acknowledge + ringmaster re-enumerate
//! 2d.  pgob_ungating       — PGRAPH GPC broadcast ungate (strategy)
//! 2e.  early_falcon_boot   — [cold ACR only] ACR DMA boot before memory training
//! 3.   memory_training     — Memory controller bring-up
//! 3b.  pmc_full            — Full engine ungating (post-devinit, strategy)
//! 3c.  engine_ungate       — Replay captured init sequences (strategy)
//! 4.   falcon_boot         — Microcontroller firmware boot (skipped if early)
//! 5.   gr_init             — GR engine register programming
//! 6.   verify              — Final memory/timer verification
//! ```
//!
//! On cold Volta+ GPUs with secure boot (HBM2 + AcrDmaHs), stage 2e runs
//! falcon_boot early so the PMU can drive HBM2 calibration in stage 3.
//!
//! # Contract
//!
//! The pipeline returns [`SovereignInitResult`] with per-stage outcomes.
//! Glowplug expects `all_ok`, `compute_ready`, and `halted_at` fields.
//!
//! **Important:** `compute_ready` means the init pipeline passed (PTIMER,
//! PRAMIN, PMC readback), NOT that shader dispatch is possible. On VFIO GPUs
//! where GPCCS is HS fuse-locked (Volta+), `compute_ready=true` coexists with
//! `classify_tier()` returning Tier 1 (WarmInfrastructure). Use
//! `sovereign.classify_tier` for dispatch readiness assessment.

mod context;
mod engine_ungate;
mod memory_path;
mod post_memory;
mod pre_memory;
mod result;

use std::time::Instant;

use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_strategy::SovereignStrategy;

pub use crate::vfio::sovereign_types::{
    HaltBefore, SovereignInitOptions, SovereignInitResult, StageResult, StageStatus,
};

use context::PipelineCtx;
use memory_path::MemoryPathOutcome;
use pre_memory::PreMemoryOutcome;

/// Run the full sovereign init pipeline on a device.
///
/// `bar0` must be a valid mapped BAR0 region from an active device.
/// `strategy` encodes all vendor/generation-specific decisions.
///
/// All MMIO in the probe stage uses fork isolation; subsequent stages
/// use direct BAR0 access (the controller's r/w helpers already have PRI
/// fault recovery).
pub fn sovereign_init(
    bar0: &MappedBar,
    bdf: &str,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
) -> SovereignInitResult {
    let pipeline_start = Instant::now();
    let mut ctx = PipelineCtx::new(bdf, pipeline_start);

    let pre = match pre_memory::run(&mut ctx, bar0, opts, strategy) {
        PreMemoryOutcome::Done(result) => return result,
        PreMemoryOutcome::Continue(cont) => cont,
    };

    let mem = match memory_path::run(&mut ctx, bar0, opts, strategy, &pre) {
        MemoryPathOutcome::Done(result) => return result,
        MemoryPathOutcome::Continue(cont) => cont,
    };

    post_memory::run(&mut ctx, bar0, opts, strategy, mem)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chip_id_to_sm_covers_titan_v() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0x140), 70);
    }

    #[test]
    fn chip_id_to_sm_covers_k80() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0x0E7), 35);
    }

    #[test]
    fn chip_id_to_sm_unknown_defaults_to_70() {
        use crate::vfio::sovereign_stages::chip_id_to_sm;
        assert_eq!(chip_id_to_sm(0xFFF), 70);
    }

    #[test]
    fn stage_status_serde_roundtrip() {
        let json = serde_json::to_string(&StageStatus::Ok).unwrap();
        assert_eq!(json, "\"ok\"");
        let back: StageStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, StageStatus::Ok);
    }

    #[test]
    fn sovereign_init_result_display_halted() {
        let r = SovereignInitResult {
            bdf: "0000:03:00.0".into(),
            identity_chip: 0x140,
            identity_raw: 0x140000A1,
            all_ok: true,
            compute_ready: false,
            halted_at: Some("memory_training".into()),
            stages: vec![],
            total_ms: 42,
            training_writes: None,
            warm_detected: false,
            boot_state: None,
        };
        let s = r.to_string();
        assert!(s.contains("HALTED@memory_training"));
        assert!(s.contains("42ms"));
    }

    #[test]
    fn sovereign_init_result_display_ready() {
        let r = SovereignInitResult {
            bdf: "0000:03:00.0".into(),
            identity_chip: 0x140,
            identity_raw: 0x140000A1,
            all_ok: true,
            compute_ready: true,
            halted_at: None,
            stages: vec![StageResult {
                name: "identity_probe".into(),
                status: StageStatus::Ok,
                detail: None,
                duration_ms: 1,
            }],
            total_ms: 100,
            training_writes: Some(42),
            warm_detected: true,
            boot_state: None,
        };
        let s = r.to_string();
        assert!(s.contains("COMPUTE_READY"));
        assert!(s.contains("0x140"));
    }

    #[test]
    fn halt_before_serde_roundtrip() {
        let json = serde_json::to_string(&HaltBefore::MemoryTraining).unwrap();
        assert_eq!(json, "\"memory_training\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::MemoryTraining);
    }

    #[test]
    fn halt_before_cg_sweep_serde() {
        let json = serde_json::to_string(&HaltBefore::CgSweep).unwrap();
        assert_eq!(json, "\"cg_sweep\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::CgSweep);
    }

    #[test]
    fn halt_before_pgob_ungate_serde() {
        let json = serde_json::to_string(&HaltBefore::PgobUngate).unwrap();
        assert_eq!(json, "\"pgob_ungate\"");
        let back: HaltBefore = serde_json::from_str(&json).unwrap();
        assert_eq!(back, HaltBefore::PgobUngate);
    }

    #[test]
    fn result_backward_compat_aliases() {
        let json = r#"{
            "bdf": "0000:03:00.0",
            "chip_id": 320,
            "boot0": 335544481,
            "all_ok": true,
            "compute_ready": true,
            "stages": [],
            "total_ms": 100,
            "hbm2_writes": 42,
            "warm_detected": false
        }"#;
        let r: SovereignInitResult = serde_json::from_str(json).unwrap();
        assert_eq!(r.identity_chip, 320);
        assert_eq!(r.identity_raw, 335544481);
        assert_eq!(r.training_writes, Some(42));
    }

    #[test]
    fn options_default_has_no_halt() {
        let opts = SovereignInitOptions::default();
        assert!(opts.halt_before.is_none());
        assert!(opts.golden_state.is_none());
        assert!(opts.vbios_rom.is_none());
        assert!(!opts.skip_gr_init);
    }
}
