// SPDX-License-Identifier: AGPL-3.0-only
//! Environment variable parsing and overrides
//!
//! Environment detection, debug/verbose flags, and env var collection.

use std::collections::HashMap;
use std::env;

use crate::env_config::EnvConfigLoader;

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
