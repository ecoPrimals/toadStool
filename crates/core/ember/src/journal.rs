// SPDX-License-Identifier: AGPL-3.0-only

//! Immutable lifecycle event journal.
//!
//! Every significant lifecycle event (hold, release, swap, lend, reclaim,
//! health transition) is logged as a [`JournalEntry`] in the [`SwapJournal`].
//! The journal is append-only for diagnostics and provenance — it answers
//! "what happened to this device and when?"

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// A lifecycle event recorded in the journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// When the event occurred.
    pub timestamp: SystemTime,
    /// What kind of event.
    pub event: JournalEvent,
    /// Optional human-readable detail.
    pub detail: Option<String>,
}

/// The kind of lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JournalEvent {
    /// Resource was first acquired and held.
    Acquired,
    /// Resource was released back to the kernel.
    Released,
    /// Personality swap initiated.
    SwapStarted {
        /// Target personality name.
        target: String,
    },
    /// Personality swap completed.
    SwapCompleted {
        /// Resulting personality name.
        result: String,
    },
    /// Personality swap failed.
    SwapFailed {
        /// Error description.
        error: String,
    },
    /// Resource lent to a consumer.
    Lent {
        /// Who borrowed it.
        borrower: String,
    },
    /// Resource reclaimed from a consumer.
    Reclaimed,
    /// Health status changed.
    HealthChanged {
        /// New health status.
        status: String,
    },
    /// Resource reacquired after release.
    Reacquired,
}

/// Append-only journal of lifecycle events for a single device.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwapJournal {
    entries: Vec<JournalEntry>,
}

impl SwapJournal {
    /// Create an empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a lifecycle event.
    pub fn record(&mut self, event: JournalEvent, detail: Option<String>) {
        self.entries.push(JournalEntry {
            timestamp: SystemTime::now(),
            event,
            detail,
        });
    }

    /// Number of recorded events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all entries in chronological order.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// The most recent entry, if any.
    #[must_use]
    pub fn latest(&self) -> Option<&JournalEntry> {
        self.entries.last()
    }

    /// All entries of a specific event type.
    #[must_use]
    pub fn events_of_type(&self, target: &JournalEvent) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|e| std::mem::discriminant(&e.event) == std::mem::discriminant(target))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_lifecycle() {
        let mut journal = SwapJournal::new();
        assert!(journal.is_empty());

        journal.record(JournalEvent::Acquired, Some("initial hold".into()));
        journal.record(
            JournalEvent::SwapStarted {
                target: "vfio".into(),
            },
            None,
        );
        journal.record(
            JournalEvent::SwapCompleted {
                result: "vfio".into(),
            },
            None,
        );

        assert_eq!(journal.len(), 3);
        assert!(matches!(
            journal.latest().unwrap().event,
            JournalEvent::SwapCompleted { .. }
        ));
    }

    #[test]
    fn filter_by_event_type() {
        let mut journal = SwapJournal::new();
        journal.record(JournalEvent::Acquired, None);
        journal.record(JournalEvent::Released, None);
        journal.record(JournalEvent::Reacquired, None);
        journal.record(JournalEvent::Released, None);

        let releases = journal.events_of_type(&JournalEvent::Released);
        assert_eq!(releases.len(), 2);
    }
}
