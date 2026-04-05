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

// Platform-specific implementations
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
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
