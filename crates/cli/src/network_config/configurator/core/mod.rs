// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core configurator functionality
//!
//! This module provides the core construction and orchestration methods
//! for the orchestration network configurator.

mod apply_validate;
mod defaults;

#[cfg(test)]
mod tests;

use super::*;

/// Core configurator trait
///
/// Provides construction and main orchestration methods
#[expect(
    clippy::redundant_pub_crate,
    reason = "explicit visibility for clarity"
)]
pub(crate) trait ConfiguratorCore {
    /// Create a new configurator
    fn new() -> Self;

    /// Get default configuration
    fn default_config() -> OrchestrationNetworkConfig;

    /// Apply all configuration
    async fn apply_configuration(&self) -> ToadStoolResult<()>;

    /// Validate all configuration
    fn validate_configuration(&self) -> ToadStoolResult<()>;
}

impl super::OrchestrationNetworkConfigurator {
    /// Generate a summary of the current configuration
    pub fn generate_configuration_summary(&self) -> String {
        format!(
            "Orchestration network configuration summary:\n\
             - Service Mesh: {}\n\
             - Proxy: configured\n\
             - Inter-Service: configured\n\
             - Traffic Management: configured\n\
             - Status: active",
            if self.config.service_mesh.enabled {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}
