// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security context for workload execution

use serde::{Deserialize, Serialize};

use crate::{ToadStoolError, ToadStoolResult};

use super::types::{Capability, FilesystemSecurity, IsolationLevel, NetworkSecurity, UserContext};

/// Security context for workload execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Capabilities granted to the workload
    pub capabilities: Vec<Capability>,
    /// User context
    pub user_context: Option<UserContext>,
    /// Network security settings
    pub network_security: NetworkSecurity,
    /// File system security settings
    pub filesystem_security: FilesystemSecurity,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::Standard,
            capabilities: vec![Capability::Execute, Capability::Read],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        }
    }
}

impl SecurityContext {
    /// Create a security context for a specific isolation level
    #[must_use]
    pub fn for_isolation_level(level: IsolationLevel) -> Self {
        Self {
            isolation_level: level,
            capabilities: vec![Capability::Execute],
            user_context: None,
            network_security: NetworkSecurity::default(),
            filesystem_security: FilesystemSecurity::default(),
        }
    }

    /// Add a capability to the security context
    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Check if a capability is granted
    ///
    /// Called frequently during execution - inlined for performance
    #[inline]
    #[must_use]
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check whether this context has the named permission.
    ///
    /// Maps common string names (`"read"`, `"write"`, `"execute"`, `"network_client"`,
    /// `"network_server"`, `"system_info"`, `"process_management"`) to their
    /// `Capability` counterparts. Wildcard `"*"` matches any non-empty capability
    /// list. Unknown names return `false`.
    pub fn has_permission(&self, name: &str) -> bool {
        if name == "*" {
            return !self.capabilities.is_empty();
        }
        let cap = match name {
            "read" => Capability::Read,
            "write" => Capability::Write,
            "execute" => Capability::Execute,
            "network_client" => Capability::NetworkClient,
            "network_server" => Capability::NetworkServer,
            "system_info" => Capability::SystemInfo,
            "process_management" => Capability::ProcessManagement,
            other => Capability::Custom(other.to_string()),
        };
        self.has_capability(&cap)
    }

    /// Set user context
    #[must_use]
    pub fn with_user_context(mut self, user_context: UserContext) -> Self {
        self.user_context = Some(user_context);
        self
    }

    /// Validate the security context
    ///
    /// # Errors
    ///
    /// Returns error if capabilities are empty or mutually inconsistent with filesystem policy.
    pub fn validate(&self) -> ToadStoolResult<()> {
        // Basic validation of the security context
        if self.capabilities.is_empty() {
            return Err(ToadStoolError::validation(
                "Security context must have at least one capability",
            ));
        }

        // Check for conflicting capabilities
        if self.capabilities.contains(&Capability::Read)
            && self.capabilities.contains(&Capability::Write)
            && self.filesystem_security.read_only
        {
            return Err(ToadStoolError::validation(
                "Cannot have write capability with read-only filesystem",
            ));
        }

        Ok(())
    }
}
