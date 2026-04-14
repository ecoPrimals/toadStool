// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration validation
//!
//! Validates ToadStool configuration values to ensure they are within acceptable ranges

mod app;
mod cache;
mod database;
mod logging;
mod metrics;
mod network;
mod resources;
mod runtime;
mod security;

use crate::ToadStoolConfig;

impl ToadStoolConfig {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid:
    /// - Port numbers are 0 or out of valid range
    /// - Resource limits are outside 0-100% range
    /// - Required fields (endpoints, names) are empty
    /// - Thread counts are 0
    /// - Timeout values are 0
    /// - Port ranges are invalid (start >= end)
    pub fn validate_runtime_config(&self) -> super::ConfigResult<()> {
        network::validate(self)?;
        resources::validate(self)?;
        app::validate(self)?;
        runtime::validate(self)?;
        logging::validate(self)?;
        security::validate(self)?;
        cache::validate(self)?;
        metrics::validate(self)?;
        database::validate(self)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
