// SPDX-License-Identifier: AGPL-3.0-or-later
//! Legacy systems compatibility layer.
//!
//! Provides emulation and compatibility mappings for legacy target systems.

use serde::{Deserialize, Serialize};
use std::future::Future;

use crate::{ExecutionRequest, ExecutionResponse, ToadStoolResult};

use super::CompatibilityLayer;

/// Legacy systems compatibility layer
#[derive(Debug)]
pub struct LegacyCompatibilityLayer {
    config: LegacyCompatConfig,
}

/// Configuration for legacy systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCompatConfig {
    /// Target legacy system
    pub target_system: String,
    /// Emulation mode
    pub emulation_mode: String,
    /// Resource limits
    pub resource_limits: std::collections::HashMap<String, u64>,
    /// Compatibility mappings
    pub compatibility_mappings: std::collections::HashMap<String, String>,
}

impl Default for LegacyCompatConfig {
    fn default() -> Self {
        Self {
            target_system: "generic".to_string(),
            emulation_mode: "basic".to_string(),
            resource_limits: std::collections::HashMap::new(),
            compatibility_mappings: std::collections::HashMap::new(),
        }
    }
}

impl Default for LegacyCompatibilityLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl LegacyCompatibilityLayer {
    /// Creates a new legacy compatibility layer with default config.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: LegacyCompatConfig::default(),
        }
    }

    /// Returns the legacy compatibility config.
    #[must_use]
    pub const fn get_config(&self) -> &LegacyCompatConfig {
        &self.config
    }
}

impl CompatibilityLayer for LegacyCompatibilityLayer {
    fn name(&self) -> &'static str {
        "legacy"
    }

    fn features(&self) -> Vec<String> {
        vec!["emulation".to_string(), "compatibility".to_string()]
    }

    fn can_handle(&self, _request: &ExecutionRequest) -> bool {
        true
    }

    fn execute_with_compatibility(
        &self,
        _request: ExecutionRequest,
    ) -> impl Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            Err(crate::error::ToadStoolError::not_supported(
                "Legacy compatibility execution not implemented. \
                 Use capability-based execution dispatch via the compute.execute \
                 JSON-RPC method instead.",
            ))
        }
    }

    fn initialize(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}
