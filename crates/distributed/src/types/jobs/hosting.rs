// SPDX-License-Identifier: AGPL-3.0-or-later
//! Recursive hosting configuration and cross-platform compatibility modes.

use std::collections::HashMap;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

/// Compatibility mode for cross-platform execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CompatibilityMode {
    /// Native execution on host OS.
    Native,
    /// Containerized execution.
    Container,
    /// Emulated execution (e.g. QEMU).
    Emulated,
    /// Hybrid native + emulated.
    Hybrid,
    /// Linux compatibility layer.
    LinuxCompat,
    /// Windows compatibility layer.
    WindowsCompat,
    /// macOS compatibility layer.
    MacOSCompat,
    /// Container compatibility layer.
    ContainerCompat,
    /// Legacy system compatibility.
    LegacyCompat {
        /// Legacy system type identifier.
        system_type: String,
    },
}

/// Configuration for `ToadStool` hosting.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToadStoolHostingConfig {
    /// Enable hosting.
    pub enabled: bool,
    /// Hosting mode (standalone, child, etc.).
    pub mode: String,
    /// Resource limits (key-value).
    pub resource_limits: HashMap<String, u64>,
    /// Security settings (key-value).
    pub security_settings: HashMap<String, String>,
    /// Resource allocation for the instance.
    pub resource_allocation: Option<crate::types::resources::ResourceAllocation>,
}

impl Hash for ToadStoolHostingConfig {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.enabled.hash(state);
        self.mode.hash(state);

        let mut resource_limits: Vec<_> = self.resource_limits.iter().collect();
        resource_limits.sort_by_key(|&(k, _)| k);
        for (k, v) in resource_limits {
            k.hash(state);
            v.hash(state);
        }

        let mut security_settings: Vec<_> = self.security_settings.iter().collect();
        security_settings.sort_by_key(|&(k, _)| k);
        for (k, v) in security_settings {
            k.hash(state);
            v.hash(state);
        }

        if let Some(ref allocation) = self.resource_allocation {
            allocation.hash(state);
        }
    }
}

impl CompatibilityMode {
    /// Get mode as string (zero-copy for standard modes)
    ///
    /// Returns a static string for standard modes, only allocates for LegacyCompat
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Container => "container",
            Self::Emulated => "emulated",
            Self::Hybrid => "hybrid",
            Self::LinuxCompat => "linux_compat",
            Self::WindowsCompat => "windows_compat",
            Self::MacOSCompat => "macos_compat",
            Self::ContainerCompat => "container_compat",
            Self::LegacyCompat { .. } => "legacy_compat",
        }
    }
}
