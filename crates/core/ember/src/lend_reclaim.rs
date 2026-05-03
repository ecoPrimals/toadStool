// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lend / reclaim lifecycle for held resources.
//!
//! A held resource can be **lent** to a consumer (e.g. a workload, a runtime
//! engine, a peer primal). While lent, ember retains the metadata but the
//! handle is in the consumer's possession. When the consumer is done, they
//! **reclaim** — ember takes the handle back and restores full ownership.
//!
//! This models the visualization service's lend/reclaim pattern for VFIO group FDs but
//! generalizes it to any resource type.

use std::time::Instant;

use serde::{Deserialize, Serialize};

/// Tracks the lend/reclaim state of a resource.
#[derive(Debug, Clone)]
pub enum LendState {
    /// Resource is held by ember — not lent out.
    Held,
    /// Resource has been lent to a named consumer.
    Lent {
        /// Who borrowed the resource (e.g. process name, primal id).
        borrower: String,
        /// When the lend occurred.
        lent_at: Instant,
    },
}

impl LendState {
    /// Whether the resource is currently lent out.
    #[must_use]
    pub const fn is_lent(&self) -> bool {
        matches!(self, Self::Lent { .. })
    }

    /// The borrower name, if lent.
    #[must_use]
    pub fn borrower(&self) -> Option<&str> {
        match self {
            Self::Lent { borrower, .. } => Some(borrower),
            Self::Held => None,
        }
    }
}

/// Receipt issued when a resource is lent. Present this to reclaim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LendReceipt {
    /// Opaque token identifying this lend transaction.
    pub token: String,
    /// Who borrowed the resource.
    pub borrower: String,
}

impl LendReceipt {
    /// Create a new lend receipt.
    #[must_use]
    pub fn new(borrower: impl Into<String>) -> Self {
        let token = format!(
            "lend-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        Self {
            token,
            borrower: borrower.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_state_is_not_lent() {
        let state = LendState::Held;
        assert!(!state.is_lent());
        assert!(state.borrower().is_none());
    }

    #[test]
    fn lent_state_tracks_borrower() {
        let state = LendState::Lent {
            borrower: "workload-42".to_string(),
            lent_at: Instant::now(),
        };
        assert!(state.is_lent());
        assert_eq!(state.borrower(), Some("workload-42"));
    }

    #[test]
    fn receipt_has_nonempty_token_and_correct_borrower() {
        let receipt = LendReceipt::new("consumer-a");
        assert!(!receipt.token.is_empty());
        assert!(receipt.token.starts_with("lend-"));
        assert_eq!(receipt.borrower, "consumer-a");
    }

    #[test]
    fn two_receipts_differ() {
        let a = LendReceipt::new("x");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let b = LendReceipt::new("x");
        assert_ne!(a.token, b.token);
    }

    #[test]
    fn receipt_serde_roundtrip() {
        let receipt = LendReceipt::new("peer-primal");
        let json = serde_json::to_string(&receipt).expect("serialize");
        let back: LendReceipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.token, receipt.token);
        assert_eq!(back.borrower, receipt.borrower);
    }
}
