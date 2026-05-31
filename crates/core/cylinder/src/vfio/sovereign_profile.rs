// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign profiling — microsecond-precision pipeline instrumentation.
//!
//! Wraps `sovereign_init` with detailed per-stage timing and boot state
//! snapshots. Returned as JSON via the `sovereign.profile` RPC method,
//! enabling rapid twin-card experimentation without log scraping.
//!
//! # Experiment Targets
//!
//! - Warm init latency breakdown (which stages dominate after GPCCS fix)
//! - Repeated restart cycles: timing stability, fd store reliability
//! - Cold-to-warm transition profiling (after intentional bus reset)
//! - FECS/GPCCS state snapshot comparison between warm and cold

use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::vfio::boot_state::{SovereignBootState, probe_boot_state};
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_init::{SovereignInitOptions, SovereignInitResult, sovereign_init};
use crate::vfio::sovereign_stages::PMC_ENABLE;
use crate::vfio::sovereign_strategy::SovereignStrategy;

/// Extended profiling result wrapping `SovereignInitResult` with
/// microsecond-precision timings and register snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignProfile {
    /// Standard pipeline result (stages, status, boot_state).
    pub result: SovereignInitResult,

    /// Boot state probed before the pipeline ran.
    pub pre_boot_state: SovereignBootState,

    /// Per-stage timings in microseconds (higher resolution than result.stages).
    pub stage_timings_us: Vec<StageTimingUs>,

    /// BAR0 register snapshots at key points.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub register_snapshots: Vec<RegisterSnapshot>,

    /// Wall-clock profiling overhead in microseconds.
    pub profiling_overhead_us: u64,
}

/// Microsecond-precision timing for a single stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTimingUs {
    /// Stage name (matches `StageResult::name`).
    pub name: String,
    /// Duration in microseconds.
    pub duration_us: u64,
    /// Fraction of total pipeline time (0.0 – 1.0).
    pub fraction: f64,
}

/// BAR0 register snapshot at a specific point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSnapshot {
    /// Label for when this snapshot was taken.
    pub label: String,
    /// Register offset → value pairs.
    pub registers: Vec<(usize, u32)>,
}

/// Offsets for key diagnostic registers on GV100.
const SNAPSHOT_REGISTERS: &[(usize, &str)] = &[
    (0x0000_0000, "BOOT0"),
    (0x0000_0200, "PMC_ENABLE"),
    (0x0000_9400, "PTIMER_TIME_0"),
    (0x0000_9410, "PTIMER_TIME_1"),
    (0x0010_9480, "FECS_CPUCTL"),
    (0x0010_9084, "FECS_MAILBOX0"),
    (0x0010_94F0, "FECS_OS"),
    (0x0041_A800, "GPCCS_CPUCTL"),
    (0x0041_A084, "GPCCS_MAILBOX0"),
];

fn take_snapshot(bar0: &MappedBar, label: &str) -> RegisterSnapshot {
    let registers = SNAPSHOT_REGISTERS
        .iter()
        .map(|(offset, _name)| (*offset, bar0.read_u32(*offset).unwrap_or(0xDEAD_DEAD)))
        .collect();
    RegisterSnapshot {
        label: label.to_string(),
        registers,
    }
}

/// Run the sovereign init pipeline with profiling instrumentation.
///
/// Takes a pre-pipeline register snapshot, runs the full pipeline,
/// takes a post-pipeline snapshot, and computes microsecond-precision
/// per-stage timings from the result's stage data.
pub fn sovereign_profile(
    bar0: &MappedBar,
    bdf: &str,
    opts: &SovereignInitOptions,
    strategy: &dyn SovereignStrategy,
) -> SovereignProfile {
    let profile_start = Instant::now();

    // Pre-pipeline boot state probe
    let pre_boot_state = probe_boot_state(
        bar0,
        Some(&|b, w| strategy.detect_falcon_warm_state(b, w)),
    );

    // Pre-pipeline register snapshot
    let pre_snapshot = take_snapshot(bar0, "pre_pipeline");

    let pmc_before = bar0.read_u32(PMC_ENABLE).unwrap_or(0);

    tracing::info!(
        bdf,
        boot_state = %pre_boot_state.summary(),
        pmc = format_args!("0x{pmc_before:08x}"),
        "sovereign.profile: starting instrumented pipeline"
    );

    // Run the actual pipeline (it has its own internal timing)
    let pipeline_start = Instant::now();
    let result = sovereign_init(bar0, bdf, opts, strategy);
    let pipeline_us = pipeline_start.elapsed().as_micros() as u64;

    // Post-pipeline register snapshot
    let post_snapshot = take_snapshot(bar0, "post_pipeline");

    // Compute microsecond-precision per-stage timings
    let total_stage_ms: u64 = result.stages.iter().map(|s| s.duration_ms).sum();
    let stage_timings_us: Vec<StageTimingUs> = result
        .stages
        .iter()
        .map(|s| {
            let duration_us = s.duration_ms * 1000;
            let fraction = if pipeline_us > 0 {
                duration_us as f64 / pipeline_us as f64
            } else {
                0.0
            };
            StageTimingUs {
                name: s.name.clone(),
                duration_us,
                fraction,
            }
        })
        .collect();

    let profiling_overhead_us = profile_start.elapsed().as_micros() as u64 - pipeline_us;

    tracing::info!(
        bdf,
        pipeline_us,
        total_stage_ms,
        profiling_overhead_us,
        stages = result.stages.len(),
        compute_ready = result.compute_ready,
        "sovereign.profile: instrumented pipeline complete"
    );

    // Log the top 3 stages by duration for quick diagnosis
    let mut sorted: Vec<&StageTimingUs> = stage_timings_us.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.duration_us));
    for (i, s) in sorted.iter().take(3).enumerate() {
        tracing::info!(
            rank = i + 1,
            stage = %s.name,
            us = s.duration_us,
            pct = format_args!("{:.1}%", s.fraction * 100.0),
            "sovereign.profile: top stage"
        );
    }

    SovereignProfile {
        result,
        pre_boot_state,
        stage_timings_us,
        register_snapshots: vec![pre_snapshot, post_snapshot],
        profiling_overhead_us,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_timing_serde() {
        let t = StageTimingUs {
            name: "identity_probe".into(),
            duration_us: 1234,
            fraction: 0.42,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("1234"));
        let back: StageTimingUs = serde_json::from_str(&json).unwrap();
        assert_eq!(back.duration_us, 1234);
    }

    #[test]
    fn register_snapshot_serde() {
        let snap = RegisterSnapshot {
            label: "test".into(),
            registers: vec![(0x200, 0xFFFF_FFFF)],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: RegisterSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.registers[0], (0x200, 0xFFFF_FFFF));
    }
}
