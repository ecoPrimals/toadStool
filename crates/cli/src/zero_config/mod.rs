//! Zero-Configuration Deployment System
//!
//! This module provides rapid system discovery and deployment capabilities for ToadStool,
//! enabling sub-60-second bootstrap with full ecosystem integration.

// Re-export types
pub use types::*;

// Module declarations
mod types;
mod core;
mod discovery;
mod configuration;
mod deployment;
mod verification;

// Re-export core trait
pub use core::ZeroConfigCore;

// Re-export extension traits
pub use discovery::DiscoveryExt;
pub use configuration::ConfigurationExt;
pub use deployment::DeploymentExt;
pub use verification::VerificationExt;

// Re-export main struct from types
pub use types::ZeroConfigDeployment;

// Re-export standalone functions
pub use core::execute_zero_config_deployment;
