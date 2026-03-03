// SPDX-License-Identifier: AGPL-3.0-or-later
//! Zero-Configuration Deployment System
//!
//! This module provides rapid system discovery and deployment capabilities for ToadStool,
//! enabling sub-60-second bootstrap with full ecosystem integration.

// Re-export types
pub use types::*;

// Module declarations
mod configuration;
mod core;
mod deployment;
mod discovery;
mod service_discovery; // New: Modern service discovery protocols
mod types;
mod verification;

// Re-export core trait
pub use core::ZeroConfigCore;

// Re-export extension traits
pub use configuration::ConfigurationExt;
pub use deployment::DeploymentExt;
pub use discovery::DiscoveryExt;
pub use verification::VerificationExt;

// Re-export main struct from types
pub use types::ZeroConfigDeployment;

// Re-export standalone functions
pub use core::execute_zero_config_deployment;
