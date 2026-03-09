// SPDX-License-Identifier: AGPL-3.0-only

//! Streaming compute dispatch -- absorbed from hotSpring.
//!
//! Provides a backend-agnostic dispatch batching pattern that reduces
//! per-dispatch overhead by amortizing GPU command encoder submissions.
//!
//! Instead of N separate CPU-GPU-CPU round-trips, streaming mode batches
//! all dispatches into a single submission, yielding up to 20x overhead
//! reduction for workloads like HMC trajectories.
//!
//! # Origin
//!
//! Absorbed from `hotSpring/barracuda/src/streaming_dispatch.rs` (v0.6.24).
//! hotSpring's version is tied to `GpuF64`; this version is substrate-agnostic
//! so any primal or spring can use it for dispatch planning.

use std::time::Instant;

/// Dispatch mode for compute submissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DispatchMode {
    /// One command encoder per dispatch (simple, debuggable).
    Single,
    /// All dispatches batched into one encoder (production).
    Streaming,
    /// Multiple trajectory batches per encoder.
    MegaBatch {
        /// Number of dispatches per batch submission.
        batch_size: usize,
    },
}

impl DispatchMode {
    /// Whether this mode batches multiple dispatches per submission.
    #[must_use]
    pub const fn is_batched(&self) -> bool {
        !matches!(self, Self::Single)
    }
}

/// Statistics from a dispatch run.
#[derive(Clone, Debug, Default)]
pub struct DispatchStats {
    /// Number of individual compute dispatches recorded.
    pub n_dispatches: usize,
    /// Number of encoder submissions to the substrate.
    pub n_submissions: usize,
    /// Total wall time in seconds.
    pub wall_seconds: f64,
    /// Effective dispatches per submission (computed on finish).
    pub dispatches_per_submission: f64,
}

impl DispatchStats {
    /// Compute the dispatch-to-submission amortization ratio.
    #[must_use]
    pub fn amortization_ratio(&self) -> f64 {
        if self.n_submissions > 0 {
            self.n_dispatches as f64 / self.n_submissions as f64
        } else {
            0.0
        }
    }

    /// Per-dispatch overhead in microseconds.
    #[must_use]
    pub fn per_dispatch_us(&self) -> f64 {
        if self.n_dispatches > 0 {
            self.wall_seconds * 1e6 / self.n_dispatches as f64
        } else {
            0.0
        }
    }

    /// Per-submission overhead in microseconds.
    #[must_use]
    pub fn per_submission_us(&self) -> f64 {
        if self.n_submissions > 0 {
            self.wall_seconds * 1e6 / self.n_submissions as f64
        } else {
            0.0
        }
    }
}

/// Streaming dispatch context for tracking and batching compute work.
///
/// Backend-agnostic: the caller manages the actual GPU/CPU submission;
/// this context tracks dispatch/submission counts and timing.
pub struct StreamingDispatchContext {
    mode: DispatchMode,
    stats: DispatchStats,
    started: Instant,
}

impl StreamingDispatchContext {
    /// Create a new streaming dispatch context.
    #[must_use]
    pub fn new(mode: DispatchMode) -> Self {
        Self {
            mode,
            stats: DispatchStats::default(),
            started: Instant::now(),
        }
    }

    /// Get the dispatch mode.
    #[must_use]
    pub const fn mode(&self) -> DispatchMode {
        self.mode
    }

    /// Record a dispatch event (one compute kernel enqueued).
    pub fn record_dispatch(&mut self) {
        self.stats.n_dispatches += 1;
    }

    /// Record a submission event (encoder flushed to substrate).
    pub fn record_submission(&mut self) {
        self.stats.n_submissions += 1;
    }

    /// Whether a submission should be flushed based on the current mode.
    #[must_use]
    pub fn should_submit(&self) -> bool {
        match self.mode {
            DispatchMode::Single => true,
            DispatchMode::Streaming => false,
            DispatchMode::MegaBatch { batch_size } => {
                batch_size > 0 && self.stats.n_dispatches % batch_size == 0
            }
        }
    }

    /// Get current statistics (snapshot).
    #[must_use]
    pub fn stats(&self) -> &DispatchStats {
        &self.stats
    }

    /// Finalize and return statistics with wall time and ratios computed.
    #[must_use]
    pub fn finish(mut self) -> DispatchStats {
        self.stats.wall_seconds = self.started.elapsed().as_secs_f64();
        self.stats.dispatches_per_submission = self.stats.amortization_ratio();
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_mode_batched() {
        assert!(!DispatchMode::Single.is_batched());
        assert!(DispatchMode::Streaming.is_batched());
        assert!(DispatchMode::MegaBatch { batch_size: 10 }.is_batched());
    }

    #[test]
    fn stats_amortization_ratio() {
        let stats = DispatchStats {
            n_dispatches: 100,
            n_submissions: 5,
            wall_seconds: 0.05,
            dispatches_per_submission: 0.0,
        };
        assert!((stats.amortization_ratio() - 20.0).abs() < 0.01);
        assert!(stats.per_dispatch_us() > 0.0);
        assert!(stats.per_submission_us() > 0.0);
    }

    #[test]
    fn stats_zero_dispatches() {
        let stats = DispatchStats::default();
        assert_eq!(stats.amortization_ratio(), 0.0);
        assert_eq!(stats.per_dispatch_us(), 0.0);
        assert_eq!(stats.per_submission_us(), 0.0);
    }

    #[test]
    fn streaming_context_single_mode() {
        let mut ctx = StreamingDispatchContext::new(DispatchMode::Single);
        assert!(ctx.should_submit());
        ctx.record_dispatch();
        ctx.record_submission();
        assert_eq!(ctx.stats().n_dispatches, 1);
        assert_eq!(ctx.stats().n_submissions, 1);
        let stats = ctx.finish();
        assert!((stats.dispatches_per_submission - 1.0).abs() < 0.01);
    }

    #[test]
    fn streaming_context_streaming_mode() {
        let mut ctx = StreamingDispatchContext::new(DispatchMode::Streaming);
        assert!(!ctx.should_submit());
        for _ in 0..20 {
            ctx.record_dispatch();
        }
        ctx.record_submission();
        let stats = ctx.finish();
        assert_eq!(stats.n_dispatches, 20);
        assert_eq!(stats.n_submissions, 1);
        assert!((stats.dispatches_per_submission - 20.0).abs() < 0.01);
    }

    #[test]
    fn streaming_context_megabatch_mode() {
        let mut ctx = StreamingDispatchContext::new(DispatchMode::MegaBatch { batch_size: 5 });
        for i in 1..=10 {
            ctx.record_dispatch();
            if ctx.should_submit() {
                ctx.record_submission();
            }
            if i == 5 || i == 10 {
                assert_eq!(ctx.stats().n_submissions, i / 5);
            }
        }
        let stats = ctx.finish();
        assert_eq!(stats.n_dispatches, 10);
        assert_eq!(stats.n_submissions, 2);
    }

    #[test]
    fn finish_records_wall_time() {
        let ctx = StreamingDispatchContext::new(DispatchMode::Single);
        let stats = ctx.finish();
        assert!(stats.wall_seconds >= 0.0);
    }
}
