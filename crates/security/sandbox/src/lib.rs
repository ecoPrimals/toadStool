#![deny(unsafe_code)]

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
pub use manager::*;
pub use traits::*;
pub use types::*;

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
