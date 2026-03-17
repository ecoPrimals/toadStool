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

/// Per-stage progress report emitted by the streaming callback.
///
/// Absorbed from healthSpring V13 `execute_streaming()` callback pattern.
#[derive(Clone, Debug)]
pub struct StageProgress {
    /// Zero-based index of the completed stage.
    pub stage_index: usize,
    /// Total number of stages in the pipeline.
    pub total_stages: usize,
    /// Name / label of the completed stage.
    pub stage_name: String,
    /// Wall time for this stage in seconds.
    pub elapsed_secs: f64,
}

impl StageProgress {
    /// Fraction of progress in `[0.0, 1.0]`.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.total_stages > 0 {
            (self.stage_index + 1) as f64 / self.total_stages as f64
        } else {
            0.0
        }
    }
}

/// Callback signature for streaming progress updates.
///
/// The caller provides a closure that is invoked after each stage completes.
/// This enables real-time progress reporting without polling.
pub type ProgressCallback = Box<dyn FnMut(&StageProgress) + Send>;

/// Streaming dispatch context for tracking and batching compute work.
///
/// Backend-agnostic: the caller manages the actual GPU/CPU submission;
/// this context tracks dispatch/submission counts and timing.
///
/// Optionally accepts a [`ProgressCallback`] (absorbed from healthSpring V13)
/// for per-stage progress reporting.
pub struct StreamingDispatchContext {
    mode: DispatchMode,
    stats: DispatchStats,
    started: Instant,
    progress_cb: Option<ProgressCallback>,
    total_stages: usize,
}

impl StreamingDispatchContext {
    /// Create a new streaming dispatch context.
    #[must_use]
    pub fn new(mode: DispatchMode) -> Self {
        Self {
            mode,
            stats: DispatchStats::default(),
            started: Instant::now(),
            progress_cb: None,
            total_stages: 0,
        }
    }

    /// Attach a progress callback (healthSpring `execute_streaming()` pattern).
    ///
    /// The callback fires after each [`Self::record_dispatch_with_progress`] call,
    /// enabling real-time UIs and log-based monitoring.
    pub fn with_progress(mut self, total_stages: usize, cb: ProgressCallback) -> Self {
        self.total_stages = total_stages;
        self.progress_cb = Some(cb);
        self
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

    /// Record a dispatch with per-stage progress notification.
    ///
    /// If a progress callback is attached, emits a [`StageProgress`] with the
    /// stage name and elapsed time since context creation.
    pub fn record_dispatch_with_progress(&mut self, stage_name: &str) {
        let idx = self.stats.n_dispatches;
        self.stats.n_dispatches += 1;
        if let Some(cb) = self.progress_cb.as_mut() {
            let progress = StageProgress {
                stage_index: idx,
                total_stages: self.total_stages,
                stage_name: stage_name.to_string(),
                elapsed_secs: self.started.elapsed().as_secs_f64(),
            };
            cb(&progress);
        }
    }

    /// Record a submission event (encoder flushed to substrate).
    pub fn record_submission(&mut self) {
        self.stats.n_submissions += 1;
    }

    /// Whether a submission should be flushed based on the current mode.
    #[must_use]
    pub const fn should_submit(&self) -> bool {
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
    pub const fn stats(&self) -> &DispatchStats {
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
        assert!((stats.amortization_ratio() - 0.0).abs() < f64::EPSILON);
        assert!((stats.per_dispatch_us() - 0.0).abs() < f64::EPSILON);
        assert!((stats.per_submission_us() - 0.0).abs() < f64::EPSILON);
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

    #[test]
    fn stage_progress_fraction() {
        let p = StageProgress {
            stage_index: 2,
            total_stages: 10,
            stage_name: "eigensolve".to_string(),
            elapsed_secs: 0.1,
        };
        assert!((p.fraction() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn stage_progress_fraction_zero_stages() {
        let p = StageProgress {
            stage_index: 0,
            total_stages: 0,
            stage_name: "empty".to_string(),
            elapsed_secs: 0.0,
        };
        assert!((p.fraction() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_callback_fires() {
        use std::sync::{Arc, Mutex};

        let log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let log_clone = Arc::clone(&log);
        let cb: ProgressCallback = Box::new(move |p: &StageProgress| {
            log_clone.lock().unwrap().push(p.stage_name.clone());
        });

        let mut ctx = StreamingDispatchContext::new(DispatchMode::Streaming).with_progress(3, cb);
        ctx.record_dispatch_with_progress("compile");
        ctx.record_dispatch_with_progress("dispatch");
        ctx.record_dispatch_with_progress("readback");

        let captured = log.lock().unwrap();
        assert_eq!(captured.len(), 3);
        assert_eq!(captured[0], "compile");
        assert_eq!(captured[1], "dispatch");
        assert_eq!(captured[2], "readback");
    }

    #[test]
    fn no_callback_dispatch_with_progress_still_counts() {
        let mut ctx = StreamingDispatchContext::new(DispatchMode::Single);
        ctx.record_dispatch_with_progress("step-1");
        ctx.record_dispatch_with_progress("step-2");
        assert_eq!(ctx.stats().n_dispatches, 2);
    }
}
