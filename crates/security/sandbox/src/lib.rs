// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    async_fn_in_trait,
    clippy::must_use_candidate,
    clippy::struct_excessive_bools,
    clippy::map_unwrap_or,
    clippy::unused_async, // Platform stubs are async for trait/API consistency
    clippy::no_effect_underscore_binding,
    clippy::unreadable_literal,
)]

//! Cross-Platform Security Sandboxing for ToadStool
//!
//! This crate provides comprehensive security sandboxing capabilities including:
//! - Cross-platform process isolation
//! - Advanced seccomp filtering (Linux)
//! - Capability-based access control
//! - Resource containment and monitoring
//! - Security policy enforcement

pub mod helpers;
pub mod manager;
pub mod traits;
pub mod types;

/// Sandbox operation errors
#[derive(Debug)]
pub enum SandboxError {
    /// Platform does not support the requested sandbox feature
    PlatformNotSupported(String),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlatformNotSupported(msg) => write!(f, "platform not supported: {msg}"),
        }
    }
}

impl std::error::Error for SandboxError {}

/// Sandbox operation result
pub type SandboxResult<T> = std::result::Result<T, SandboxError>;

// Platform-specific implementations
#[cfg(target_os = "linux")]
pub mod linux;
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

// Re-export public APIs
pub use manager::CrossPlatformSandboxManager;
pub use traits::SandboxManager;
pub use types::{
    BandwidthLimits, FilesystemMount, MountType, NetworkConfig, NetworkIsolationMode,
    ResourceLimits, ResourceUsage, SandboxConfig, SandboxInfo, SandboxLifetime, SandboxSpec,
    SandboxStatus, SecurityViolation, ViolationSeverity,
};

// Re-export platform-specific managers
#[cfg(target_os = "linux")]
pub use linux::LinuxSandboxManager;
#[cfg(target_os = "macos")]
pub use macos::MacOSSandboxManager;
#[cfg(target_os = "windows")]
pub use windows::WindowsSandboxManager;

// Unit tests for library code coverage
#[cfg(test)]
mod lib_tests;
