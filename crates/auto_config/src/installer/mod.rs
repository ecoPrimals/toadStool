// SPDX-License-Identifier: AGPL-3.0-or-later
//! Smart installer for zero-touch ToadStool installation
//!
//! Domain modules:
//! - **paths**: Platform-specific path resolution
//! - **core**: Directory structure and toadstool executable
//! - **runtimes**: Container (Docker) and GPU runtime setup
//! - **`platform_components`**: Linux/macOS/Windows service configs
//! - **integration**: PATH, desktop shortcuts, shell completion
//! - **`config_manager`**: Apply ToadStool configuration
//! - **types**: `InstallationResult`, `InstallationConfig`

pub mod types;

#[cfg(feature = "runtime")]
mod config_manager;
#[cfg(feature = "runtime")]
mod core;
#[cfg(feature = "runtime")]
mod integration;
#[cfg(feature = "runtime")]
mod paths;
#[cfg(feature = "runtime")]
mod platform_components;
#[cfg(feature = "runtime")]
mod runtimes;

#[cfg(feature = "runtime")]
pub use config_manager::ConfigManager;

pub use types::{InstallationConfig, InstallationResult};

#[cfg(feature = "runtime")]
mod smart_installer;

#[cfg(feature = "runtime")]
pub use smart_installer::SmartInstaller;

#[cfg(all(test, feature = "runtime"))]
#[path = "mod_tests.rs"]
mod tests;
