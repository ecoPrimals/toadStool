// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inputs for [`crate::capabilities::CapabilityDiscovery`] (loaded once; avoids scattered `env::var` in discovery).

use toadstool_common::interned_strings::socket_env;

/// Optional overrides for universal compute discovery (typically from env at process startup).
#[derive(Debug, Clone, Default)]
pub struct ComputeDiscoverySettings {
    /// Comma-separated wgpu adapter selector (see `TOADSTOOL_GPU_ADAPTER`).
    pub gpu_adapter_selector: Option<String>,
}

impl ComputeDiscoverySettings {
    /// Snapshot env-driven discovery options (call once when building a [`crate::UniversalRuntime`]).
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            gpu_adapter_selector: std::env::var(socket_env::TOADSTOOL_GPU_ADAPTER).ok(),
        }
    }
}
