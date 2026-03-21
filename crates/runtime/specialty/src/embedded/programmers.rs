// SPDX-License-Identifier: AGPL-3.0-only
//! Programmer implementations for embedded systems
//!
//! # Future Feature
//!
//! This module contains placeholder types for future embedded systems support.
//! Full implementation will include device communication, programming, and verification.
//!
//! **Status**: Planned for future release
//! **Priority**: P3 (Low) - Optional advanced feature

/// Generic programmer for various devices
#[derive(Debug)]
pub struct GenericProgrammer;

/// EPROM-specific programmer
#[derive(Debug)]
pub struct EPROMProgrammer;

impl GenericProgrammer {
    /// Create a new generic programmer
    pub const fn new() -> Self {
        Self
    }
}

impl EPROMProgrammer {
    /// Create a new EPROM programmer
    pub const fn new() -> Self {
        Self
    }
}

impl Default for GenericProgrammer {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EPROMProgrammer {
    fn default() -> Self {
        Self::new()
    }
}

// Future Enhancement: Implement ProgrammerInterface trait for each programmer
// This will require full implementation of device communication, programming, and verification
// Tracked as future feature - not required for current production deployment
