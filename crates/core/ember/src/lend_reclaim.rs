// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lend / reclaim lifecycle for held resources.
//!
//! A held resource can be **lent** to a consumer (e.g. a workload, a runtime
//! engine, a peer primal). While lent, ember retains the metadata but the
//! handle is in the consumer's possession. When the consumer is done, they
//! **reclaim** — ember takes the handle back and restores full ownership.
//!
//! This models coralReef's lend/reclaim pattern for VFIO group FDs but
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
