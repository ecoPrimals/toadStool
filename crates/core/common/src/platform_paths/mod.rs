// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Platform-Agnostic Path Resolution
//!
//! Pure Rust, zero-hardcoding path resolution for cross-platform compatibility.
//!
//! ## ecoBin v2.0 Compliance
//!
//! This module follows the ecoBin Architecture Standard v2.0 requirements:
//! - **No hardcoded paths**: Uses XDG, environment detection, and `std::env::temp_dir()`
//! - **Platform-agnostic**: Works on Linux, macOS, Windows, Android, WASM
//! - **Capability-based**: Self-knowledge only, discovers paths at runtime
//!
//! ## Path Resolution Priority
//!
//! 1. **Environment variable** (highest priority)
//! 2. **XDG standard** (Linux/Unix)
//! 3. **Platform standard** (macOS/Windows/Android)
//! 4. **Temp directory fallback** (universal)
//!
//! ## Usage
//!
//! ```
//! use toadstool_common::platform_paths::{PlatformPaths, PathEnv};
//!
//! // Production: capture from environment
//! let env = PathEnv::from_env();
//! let paths = PlatformPaths::new(&env);
//!
//! // Get runtime directory for sockets
//! let runtime_dir = paths.runtime_dir();
//!
//! // Get ToadStool-specific paths
//! let socket_dir = paths.toadstool_socket_dir();
//! ```

mod convenience;
mod env;
mod paths;

pub use convenience::{
    biomeos_runtime_dir, runtime_dir, temp_dir, toadstool_socket, toadstool_socket_dir,
    toadstool_temp_dir,
};
pub use env::{PathEnv, Platform};
pub use paths::PlatformPaths;

#[cfg(test)]
mod tests;
