// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration validation (Pipeline Stage 4)

use tracing::warn;

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::ToadStoolConfig;

/// Configuration validator
pub struct ConfigValidator;

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validate the generated configuration
    pub async fn validate_configuration(&self, config: &ToadStoolConfig) -> ToadStoolResult<()> {
        // Basic validation checks
        if config.runtime.max_concurrent_executions == 0 {
            return Err(ToadStoolError::configuration(
                "No concurrent executions configured",
            ));
        }

        if config.runtime.resource_limits.max_memory_usage == 0.0 {
            return Err(ToadStoolError::configuration("No memory limit configured"));
        }

        // Advanced validation
        if config.runtime.wasm.max_memory == 0 {
            warn!("WASM runtime has no memory limit set");
        }

        if config.runtime.gpu.is_some() {
            if let Some(gpu_config) = &config.runtime.gpu {
                if gpu_config.max_memory_per_device == 0 {
                    warn!("GPU runtime enabled but no memory limit set");
                }
            }
        }

        Ok(())
    }
}
