// SPDX-License-Identifier: AGPL-3.0-only
//! Path configuration utilities
//!
//! Config file paths, directory resolution, and XDG-compliant path helpers.

use crate::env_config::EnvConfigLoader;

/// Get data directory from environment or default
#[must_use]
pub fn get_data_dir() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("DATA_DIR", "./data")
}

/// Get cache directory from environment or default
#[must_use]
pub fn get_cache_dir() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("CACHE_DIR", "./cache")
}

/// Get temp directory from environment or default
#[must_use]
pub fn get_temp_dir() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("TEMP_DIR", "./tmp")
}

/// Get log directory from environment or default
#[must_use]
pub fn get_log_dir() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("LOG_DIR", "./logs")
}

/// Get encryption key path from environment or default
#[must_use]
pub fn get_encryption_key_path() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("ENCRYPTION_KEY_PATH", "./keys/encryption.key")
}

/// Get TLS cert path from environment or default
#[must_use]
pub fn get_tls_cert_path() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("TLS_CERT_PATH", "./certs/tls.crt")
}

/// Get TLS key path from environment or default
#[must_use]
pub fn get_tls_key_path() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("TLS_KEY_PATH", "./certs/tls.key")
}

/// Get CA cert path from environment or default
#[must_use]
pub fn get_ca_cert_path() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("CA_CERT_PATH", "./certs/ca.crt")
}
