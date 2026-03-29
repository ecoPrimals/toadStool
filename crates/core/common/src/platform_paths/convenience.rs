// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

use super::env::PathEnv;
use super::paths::PlatformPaths;

/// Get runtime directory using current environment.
///
/// Prefer using `PlatformPaths` for testability.
#[must_use]
pub fn runtime_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).runtime_dir()
}

/// Get temp directory using current environment.
#[must_use]
pub fn temp_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).temp_dir()
}

/// Get ToadStool socket directory using current environment.
#[must_use]
pub fn toadstool_socket_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_socket_dir()
}

/// Get ToadStool main socket path using current environment.
#[must_use]
pub fn toadstool_socket() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_socket()
}

/// Get ToadStool temp directory using current environment.
#[must_use]
pub fn toadstool_temp_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).toadstool_temp_dir()
}

/// Get biomeOS runtime directory using current environment.
#[must_use]
pub fn biomeos_runtime_dir() -> PathBuf {
    let env = PathEnv::from_env();
    PlatformPaths::new(&env).biomeos_runtime_dir()
}
