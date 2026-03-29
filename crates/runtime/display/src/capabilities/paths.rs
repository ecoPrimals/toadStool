// SPDX-License-Identifier: AGPL-3.0-only
//! XDG-compliant socket and discovery directory paths.

use std::path::PathBuf;

/// Socket path for the display backend (XDG compliant, no hardcoding).
pub(super) fn get_socket_path() -> PathBuf {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let env = PathEnv::from_env();
    let paths = PlatformPaths::new(&env);
    paths.toadstool_socket_dir().join("display.sock")
}

/// Discovery directory for capability JSON files (XDG compliant).
pub(super) fn get_discovery_dir() -> PathBuf {
    use toadstool_common::platform_paths::{PathEnv, PlatformPaths};
    let env = PathEnv::from_env();
    let paths = PlatformPaths::new(&env);
    paths.runtime_dir().join("ecoPrimals/discovery")
}
