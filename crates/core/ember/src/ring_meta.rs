// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ring and mailbox metadata persisted alongside device holds.
//!
//! Absorbed from coralReef `coral-ember`. [`RingMeta`] and related structs
//! are shared with glowplug for IPC round-trips without pulling in VFIO
//! device types. The data is hardware-agnostic — it describes GPU ring
//! buffers, mailbox engines, and fence values for state reconstruction
//! after daemon restarts.

use serde::{Deserialize, Serialize};

/// Persistent metadata for mailbox/ring reconstruction after daemon restart.
///
/// Ember holds this alongside device fds. When glowplug dies and restarts,
/// it reads this metadata to restore its mailbox and ring state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RingMeta {
    /// Active mailbox engine names and their capacities.
    pub mailboxes: Vec<MailboxMeta>,
    /// Active ring names and their capacities.
    pub rings: Vec<RingMetaEntry>,
    /// Monotonic version — incremented on each update for consistency checking.
    pub version: u64,
}

/// Metadata for one mailbox (engine name + capacity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxMeta {
    /// Engine name (e.g. `"fecs"`, `"gpccs"`, `"sec2"`).
    pub engine: String,
    /// Slot capacity.
    pub capacity: usize,
}

/// Metadata for one ring (name + capacity + last fence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingMetaEntry {
    /// Ring name (e.g. `"gpfifo"`, `"ce0"`).
    pub name: String,
    /// Entry capacity.
    pub capacity: usize,
    /// Last consumed fence value (for continuity after restart).
    pub last_fence: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_meta_default_is_empty() {
        let meta = RingMeta::default();
        assert!(meta.mailboxes.is_empty());
        assert!(meta.rings.is_empty());
        assert_eq!(meta.version, 0);
    }

    #[test]
    fn ring_meta_roundtrip_json() {
        let meta = RingMeta {
            mailboxes: vec![MailboxMeta {
                engine: "fecs".into(),
                capacity: 16,
            }],
            rings: vec![RingMetaEntry {
                name: "gpfifo".into(),
                capacity: 64,
                last_fence: 42,
            }],
            version: 3,
        };
        let json = serde_json::to_string(&meta).expect("serialize");
        let back: RingMeta = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.mailboxes.len(), 1);
        assert_eq!(back.mailboxes[0].engine, "fecs");
        assert_eq!(back.rings[0].last_fence, 42);
        assert_eq!(back.version, 3);
    }
}
