// SPDX-License-Identifier: AGPL-3.0-only
//! Trace diffing — isolate compute-specific events.
//!
//! Compares a baseline trace (no compute) against a compute trace
//! to identify operations unique to compute initialization.

use crate::observer::{TraceEvent, TraceEventKind};

/// Diff two ordered event streams.
///
/// Returns events that appear in `compute` but not in `baseline`,
/// matched by event kind (register offset, ioctl number).
pub fn diff_traces(baseline: &[TraceEvent], compute: &[TraceEvent]) -> Vec<TraceEvent> {
    let baseline_keys: std::collections::HashSet<u64> =
        baseline.iter().filter_map(|e| event_key(&e.kind)).collect();

    compute
        .iter()
        .filter(|e| {
            event_key(&e.kind)
                .map(|k| !baseline_keys.contains(&k))
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

/// Extract a u64 key from an event for deduplication.
fn event_key(kind: &TraceEventKind) -> Option<u64> {
    match kind {
        TraceEventKind::RegisterWrite { offset, .. } => Some(*offset),
        TraceEventKind::RegisterRead { offset, .. } => Some(*offset | (1 << 63)),
        TraceEventKind::IoctlCall { ioctl_nr, .. } => Some(*ioctl_nr | (1 << 62)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observer::TraceEvent;

    fn reg_write(offset: u64, value: u64) -> TraceEvent {
        TraceEvent {
            timestamp_us: 0,
            kind: TraceEventKind::RegisterWrite {
                offset,
                value,
                width: 4,
            },
            context: String::new(),
        }
    }

    #[test]
    fn diff_removes_baseline_events() {
        let baseline = vec![reg_write(0x100, 1), reg_write(0x200, 2)];
        let compute = vec![
            reg_write(0x100, 1),
            reg_write(0x200, 2),
            reg_write(0x300, 3),
        ];

        let diff = diff_traces(&baseline, &compute);
        assert_eq!(diff.len(), 1);
        match &diff[0].kind {
            TraceEventKind::RegisterWrite { offset, .. } => assert_eq!(*offset, 0x300),
            _ => panic!("expected RegisterWrite"),
        }
    }

    #[test]
    fn diff_empty_baseline_keeps_all() {
        let compute = vec![reg_write(0x100, 1), reg_write(0x200, 2)];
        let diff = diff_traces(&[], &compute);
        assert_eq!(diff.len(), 2);
    }
}
