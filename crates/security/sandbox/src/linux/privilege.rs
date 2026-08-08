// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based privilege snapshot for graceful degradation.

use toadstool_common::platform::PrivilegeProbe;
use toadstool_hw_safe::LinuxPrivilegeProbeBackend;

/// Snapshot of privileges relevant to mount and seccomp setup.
#[derive(Debug, Clone)]
pub struct LinuxPrivilegeProbe {
    /// Effective `CAP_SYS_ADMIN` (needed for many mount namespaces / mount operations).
    pub effective_sys_admin: bool,
}

impl LinuxPrivilegeProbe {
    pub(crate) fn probe() -> Self {
        let backend = LinuxPrivilegeProbeBackend;
        Self {
            effective_sys_admin: backend.has_privilege("sys_admin"),
        }
    }

    pub(crate) fn can_attempt_mount(&self) -> bool {
        self.effective_sys_admin
    }
}
