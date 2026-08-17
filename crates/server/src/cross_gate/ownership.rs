// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hardware ownership for yield-to-owner dispatch in a multi-gate mesh.

use std::sync::Arc;

use std::sync::RwLock;

use toadstool_common::interned_strings::socket_env;

/// Tracks which gate owns the GPUs on this node (local owner vs remote owner).
#[derive(Debug, Clone)]
pub struct GateOwnership {
    /// This node's gate identity.
    pub local_gate_id: Arc<str>,
    /// Gate that owns the hardware running on this node.
    hardware_owner_gate_id: Arc<RwLock<Arc<str>>>,
}

impl GateOwnership {
    /// Create ownership state for a gate node.
    ///
    /// `TOADSTOOL_HARDWARE_OWNER_GATE_ID` overrides the default (local gate id)
    /// when this node is a guest on another gate's hardware.
    #[must_use]
    pub fn new(local_gate_id: impl AsRef<str>) -> Self {
        let local_gate_id = Arc::from(local_gate_id.as_ref());
        let initial_owner = match std::env::var(socket_env::TOADSTOOL_HARDWARE_OWNER_GATE_ID) {
            Ok(id) => Arc::from(id.as_str()),
            Err(_) => Arc::clone(&local_gate_id),
        };
        Self {
            local_gate_id,
            hardware_owner_gate_id: Arc::new(RwLock::new(initial_owner)),
        }
    }

    /// Resolved hardware owner gate id (may differ from local when guest).
    pub async fn hardware_owner_gate_id(&self) -> Arc<str> {
        Arc::clone(
            &*self
                .hardware_owner_gate_id
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// Whether the caller gate is the hardware owner (yield bypass).
    pub async fn caller_is_hardware_owner(&self, caller_gate_id: Option<&str>) -> bool {
        let Some(caller) = caller_gate_id else {
            return false;
        };
        caller == self.hardware_owner_gate_id().await.as_ref()
    }

    /// Record ownership from a `gate.update` advertisement.
    pub async fn note_gate_update(&self, gate_id: &Arc<str>, is_owner: bool) {
        if is_owner {
            *self
                .hardware_owner_gate_id
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::clone(gate_id);
        }
    }

    /// Reset hardware ownership to the local gate when the remote owner
    /// goes offline or revokes ownership.
    pub async fn revert_to_local_owner(&self) {
        *self
            .hardware_owner_gate_id
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Arc::clone(&self.local_gate_id);
    }
}
