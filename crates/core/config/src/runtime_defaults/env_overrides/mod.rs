//! Environment variable configuration overrides
//!
//! Handles applying environment variable overrides to ToadStool configuration.
//! Domain-specific handlers are split into focused submodules.

mod app;
mod features;
mod logging;
mod network;
mod parse;
mod resources;
mod runtime;
mod security;

use super::ConfigResult;
use crate::ToadStoolConfig;

impl ToadStoolConfig {
    /// Apply environment variable overrides to configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Environment variables contain invalid values (e.g., non-numeric for numbers)
    /// - Port numbers are invalid
    /// - Resource limits are out of range
    pub fn apply_env_overrides(&mut self) -> ConfigResult<()> {
        app::apply(self)?;
        network::apply(self)?;
        resources::apply(self)?;
        features::apply(self)?;
        runtime::apply(self)?;
        security::apply(self)?;
        logging::apply(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
