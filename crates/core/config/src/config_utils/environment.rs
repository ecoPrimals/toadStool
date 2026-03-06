// SPDX-License-Identifier: AGPL-3.0-or-later
//! Environment variable parsing and overrides
//!
//! Environment detection, debug/verbose flags, and env var collection.

use std::collections::HashMap;
use std::env;

use crate::env_config::EnvConfigLoader;

/// Resolve whether to use placeholder implementations for external services.
///
/// Reads `TOADSTOOL_STUB_EXTERNAL_SERVICES` (1/true/yes = use stubs).
/// Falls back to environment-appropriate default (dev: true, prod: false).
#[must_use]
pub fn stub_external_services() -> bool {
    let loader = EnvConfigLoader::new();
    let env = loader.get_string("ENV", crate::app::DEFAULT_ENVIRONMENT);
    let default = if env == crate::production::DEFAULT_PROD_ENVIRONMENT {
        crate::production::DEFAULT_PROD_STUB_EXTERNAL
    } else {
        crate::development::DEFAULT_DEV_STUB_EXTERNAL
    };
    loader.get_bool("STUB_EXTERNAL_SERVICES", default)
}

/// Get environment name from environment or default
#[must_use]
pub fn get_environment() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("ENV", "development")
}

/// Get debug mode from environment or default
#[must_use]
pub fn get_debug_mode() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("DEBUG", false)
}

/// Get verbose mode from environment or default
#[must_use]
pub fn get_verbose_mode() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("VERBOSE", false)
}

/// Get all environment variables with TOADSTOOL prefix
#[must_use]
pub fn get_all_toadstool_env_vars() -> HashMap<String, String> {
    env::vars()
        .filter(|(key, _)| key.starts_with("TOADSTOOL_"))
        .collect()
}
