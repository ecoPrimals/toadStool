// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based privilege snapshot for graceful degradation.

use rustix::thread::{CapabilitySet, capabilities};

/// Snapshot of privileges relevant to mount and seccomp setup.
#[derive(Debug, Clone)]
pub struct LinuxPrivilegeProbe {
    /// Effective `CAP_SYS_ADMIN` (needed for many mount namespaces / mount operations).
    pub effective_sys_admin: bool,
}

impl LinuxPrivilegeProbe {
    pub(crate) fn probe() -> Self {
        let caps = capabilities(None);
        match caps {
            Ok(c) => Self {
                effective_sys_admin: c.effective.contains(CapabilitySet::SYS_ADMIN),
            },
            Err(e) => {
                tracing::warn!(error = ?e, "could not read process capabilities; assuming unprivileged");
                Self {
                    effective_sys_admin: false,
                }
            }
        }
    }

    pub(crate) fn can_attempt_mount(&self) -> bool {
        self.effective_sys_admin
    }
}
