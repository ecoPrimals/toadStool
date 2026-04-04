// SPDX-License-Identifier: AGPL-3.0-only
//! Workload health monitoring and interrupt patterns.
//!
//! This module implements substrate-agnostic workload health monitoring, absorbed from
//! **hotSpring v0.6.25**'s biomeGate Brain Architecture (`specs/BIOMEGATE_BRAIN_ARCHITECTURE.md`).
//! The brain architecture treats the compute fleet as a biological brain with attention states,
//! interrupts, and corrective actions. ToadStool generalizes this from hotSpring's physics-specific
//! patterns into substrate-agnostic workload health monitoring.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Attention state of a workload, analogous to brain attention levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttentionState {
    /// Nominal — workload progressing as expected.
    Green,
    /// Alert — anomaly detected, increase monitoring frequency.
    Yellow,
    /// Critical — workload failing, corrective action needed.
    Red,
}

/// Substrate-agnostic anomaly kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkloadAnomaly {
    /// Workload progress has stalled (no forward progress for N intervals).
    Stalled,
    /// Workload is diverging (error/residual increasing).
    Diverging,
    /// Workload is slower than expected (wall time >> predicted).
    SlowerThanExpected,
    /// Workload throughput collapsed (sudden drop in ops/sec).
    ThroughputCollapse,
    /// Device health degraded (NVVM poisoning, thermal throttle).
    DeviceDegraded,
    /// Memory pressure — workload approaching allocation limits.
    MemoryPressure,
    /// Workload exceeded its deadline.
    DeadlineExceeded,
}

/// Corrective actions the orchestrator can take in response to an interrupt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterruptAction {
    /// Informational only — log and continue.
    NoAction,
    /// Increase monitoring frequency.
    IncreaseMonitoring {
        /// Polling interval in milliseconds.
        interval_ms: u64,
    },
    /// Decrease monitoring frequency (things are improving).
    DecreaseMonitoring {
        /// Polling interval in milliseconds.
        interval_ms: u64,
    },
    /// Kill the current workload.
    KillWorkload {
        /// Reason for killing.
        reason: String,
    },
    /// Restart workload with different parameters.
    RestartWorkload {
        /// Reason for restart.
        reason: String,
    },
    /// Migrate workload to a different substrate.
    MigrateSubstrate {
        /// Target substrate identifier.
        target: String,
        /// Reason for migration.
        reason: String,
    },
    /// Preempt workload (yield resources to higher priority).
    Preempt {
        /// Reason for preemption.
        reason: String,
    },
}

/// A recorded workload interrupt with anomaly, severity, and recommended action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadInterrupt {
    /// Detected anomaly kind.
    pub anomaly: WorkloadAnomaly,
    /// Current attention state (severity).
    pub severity: AttentionState,
    /// Recommended corrective action.
    pub action: InterruptAction,
    /// Human-readable context.
    pub context: String,
    /// Timestamp (milliseconds since epoch).
    pub timestamp_ms: u64,
}

/// Monitors workload health with attention-state escalation and corrective actions.
pub struct WorkloadHealthMonitor {
    state: AttentionState,
    interrupts: Vec<WorkloadInterrupt>,
    /// Number of consecutive anomaly-free intervals.
    healthy_streak: u32,
    /// Number of consecutive anomaly intervals.
    anomaly_streak: u32,
    /// Threshold for escalation from Green to Yellow.
    yellow_threshold: u32,
    /// Threshold for escalation from Yellow to Red.
    red_threshold: u32,
}

impl WorkloadHealthMonitor {
    /// Create a new monitor with default thresholds (yellow=3, red=5).
    #[must_use]
    pub const fn new() -> Self {
        Self::with_thresholds(3, 5)
    }

    /// Create a monitor with custom escalation thresholds.
    #[must_use]
    pub const fn with_thresholds(yellow_threshold: u32, red_threshold: u32) -> Self {
        Self {
            state: AttentionState::Green,
            interrupts: Vec::new(),
            healthy_streak: 0,
            anomaly_streak: 0,
            yellow_threshold,
            red_threshold,
        }
    }

    /// Record a healthy interval. May de-escalate if `healthy_streak` >= `yellow_threshold`.
    /// Red never auto-de-escalates; use `reset()` for that.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "not const due to future evolution"
    )] // Mutates self
    pub fn report_healthy(&mut self) {
        self.healthy_streak += 1;
        self.anomaly_streak = 0;

        if self.healthy_streak >= self.yellow_threshold {
            match self.state {
                AttentionState::Yellow => {
                    self.state = AttentionState::Green;
                    self.healthy_streak = 0;
                }
                AttentionState::Green | AttentionState::Red => {
                    // Green: already nominal; Red: never auto-de-escalate
                }
            }
        }
    }

    /// Record an anomaly, escalate if thresholds exceeded, and return an interrupt with
    /// recommended action.
    #[must_use]
    pub fn report_anomaly(&mut self, anomaly: WorkloadAnomaly) -> WorkloadInterrupt {
        self.anomaly_streak += 1;
        self.healthy_streak = 0;

        // Escalation logic
        match self.state {
            AttentionState::Green if self.anomaly_streak >= self.yellow_threshold => {
                self.state = AttentionState::Yellow;
            }
            AttentionState::Yellow if self.anomaly_streak >= self.red_threshold => {
                self.state = AttentionState::Red;
            }
            _ => {}
        }

        let action = self.select_action(anomaly);
        let context = Self::build_context(anomaly, self.state, &action);

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let interrupt = WorkloadInterrupt {
            anomaly,
            severity: self.state,
            action,
            context,
            timestamp_ms,
        };

        self.interrupts.push(interrupt.clone());
        interrupt
    }

    fn select_action(&self, anomaly: WorkloadAnomaly) -> InterruptAction {
        match self.state {
            AttentionState::Green => InterruptAction::IncreaseMonitoring { interval_ms: 500 },
            AttentionState::Yellow => match anomaly {
                WorkloadAnomaly::Stalled | WorkloadAnomaly::Diverging => {
                    InterruptAction::RestartWorkload {
                        reason: format!("{anomaly:?} detected"),
                    }
                }
                _ => InterruptAction::IncreaseMonitoring { interval_ms: 250 },
            },
            AttentionState::Red => match anomaly {
                WorkloadAnomaly::DeviceDegraded => InterruptAction::MigrateSubstrate {
                    target: "fallback".to_string(),
                    reason: "Device degraded, migrating to alternate substrate".to_string(),
                },
                _ => InterruptAction::KillWorkload {
                    reason: format!("Critical: {anomaly:?}"),
                },
            },
        }
    }

    fn build_context(
        anomaly: WorkloadAnomaly,
        severity: AttentionState,
        action: &InterruptAction,
    ) -> String {
        format!("Anomaly {anomaly:?} at {severity:?} — action: {action:?}")
    }

    /// Current attention state.
    #[must_use]
    pub const fn state(&self) -> AttentionState {
        self.state
    }

    /// Reset to Green and clear streaks. Use when starting a new workload or after manual recovery.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "not const due to future evolution"
    )] // Mutates self
    pub fn reset(&mut self) {
        self.state = AttentionState::Green;
        self.healthy_streak = 0;
        self.anomaly_streak = 0;
    }

    /// All recorded interrupts.
    #[must_use]
    pub fn interrupts(&self) -> &[WorkloadInterrupt] {
        &self.interrupts
    }
}

impl Default for WorkloadHealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn green_stays_green_on_healthy() {
        let mut m = WorkloadHealthMonitor::new();
        for _ in 0..10 {
            m.report_healthy();
        }
        assert_eq!(m.state(), AttentionState::Green);
    }

    #[test]
    fn escalation_to_yellow() {
        let mut m = WorkloadHealthMonitor::with_thresholds(3, 5);
        assert_eq!(m.state(), AttentionState::Green);

        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        assert_eq!(m.state(), AttentionState::Green);

        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        assert_eq!(m.state(), AttentionState::Green);

        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        assert_eq!(m.state(), AttentionState::Yellow);
    }

    #[test]
    fn escalation_to_red() {
        let mut m = WorkloadHealthMonitor::with_thresholds(3, 5);
        for _ in 0..3 {
            let _ = m.report_anomaly(WorkloadAnomaly::Diverging);
        }
        assert_eq!(m.state(), AttentionState::Yellow);

        let _ = m.report_anomaly(WorkloadAnomaly::Diverging);
        let _ = m.report_anomaly(WorkloadAnomaly::Diverging);
        assert_eq!(m.state(), AttentionState::Red);
    }

    #[test]
    fn de_escalation_yellow_to_green() {
        let mut m = WorkloadHealthMonitor::with_thresholds(3, 5);
        for _ in 0..3 {
            let _ = m.report_anomaly(WorkloadAnomaly::SlowerThanExpected);
        }
        assert_eq!(m.state(), AttentionState::Yellow);

        for _ in 0..3 {
            m.report_healthy();
        }
        assert_eq!(m.state(), AttentionState::Green);
    }

    #[test]
    fn red_never_auto_de_escalates() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 3);
        for _ in 0..3 {
            let _ = m.report_anomaly(WorkloadAnomaly::ThroughputCollapse);
        }
        assert_eq!(m.state(), AttentionState::Red);

        for _ in 0..20 {
            m.report_healthy();
        }
        assert_eq!(m.state(), AttentionState::Red);
    }

    #[test]
    fn action_selection_green_increase_monitoring() {
        let mut m = WorkloadHealthMonitor::new();
        let interrupt = m.report_anomaly(WorkloadAnomaly::MemoryPressure);
        assert!(matches!(
            interrupt.action,
            InterruptAction::IncreaseMonitoring { interval_ms: 500 }
        ));
    }

    #[test]
    fn action_selection_yellow_stalled_restart() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 5);
        let _ = m.report_anomaly(WorkloadAnomaly::SlowerThanExpected);
        let interrupt = m.report_anomaly(WorkloadAnomaly::Stalled);
        assert!(matches!(
            interrupt.action,
            InterruptAction::RestartWorkload { .. }
        ));
    }

    #[test]
    fn action_selection_yellow_diverging_restart() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 5);
        let _ = m.report_anomaly(WorkloadAnomaly::MemoryPressure);
        let interrupt = m.report_anomaly(WorkloadAnomaly::Diverging);
        assert!(matches!(
            interrupt.action,
            InterruptAction::RestartWorkload { .. }
        ));
    }

    #[test]
    fn action_selection_red_device_degraded_migrate() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 3);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::DeviceDegraded);
        let interrupt = m.report_anomaly(WorkloadAnomaly::DeviceDegraded);
        assert!(matches!(
            interrupt.action,
            InterruptAction::MigrateSubstrate { .. }
        ));
    }

    #[test]
    fn action_selection_red_other_kill() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 3);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let interrupt = m.report_anomaly(WorkloadAnomaly::DeadlineExceeded);
        assert!(matches!(
            interrupt.action,
            InterruptAction::KillWorkload { .. }
        ));
    }

    #[test]
    fn reset_clears_state() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 3);
        for _ in 0..3 {
            let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        }
        assert_eq!(m.state(), AttentionState::Red);

        m.reset();
        assert_eq!(m.state(), AttentionState::Green);
    }

    #[test]
    fn interrupts_recorded() {
        let mut m = WorkloadHealthMonitor::with_thresholds(2, 5);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);

        let interrupts = m.interrupts();
        assert_eq!(interrupts.len(), 3);
        assert!(
            interrupts
                .iter()
                .all(|i| i.anomaly == WorkloadAnomaly::Stalled)
        );
        assert!(interrupts.iter().all(|i| i.timestamp_ms > 0));
    }

    #[test]
    fn healthy_streak_resets_anomaly_streak() {
        let mut m = WorkloadHealthMonitor::with_thresholds(3, 5);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        m.report_healthy();
        assert_eq!(m.state(), AttentionState::Green);
        // After one healthy, anomaly_streak should be 0, so next anomaly starts fresh
        let _ = m.report_anomaly(WorkloadAnomaly::Stalled);
        assert_eq!(m.state(), AttentionState::Green);
    }
}
