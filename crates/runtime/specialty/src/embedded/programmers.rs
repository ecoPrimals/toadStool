// SPDX-License-Identifier: AGPL-3.0-or-later
//! Programmer implementations for embedded systems
//!
//! [`GenericProgrammer`] and [`EPROMProgrammer`] hold parsed configuration and a
//! [`super::protocol_engine::ProtocolEngine`] for transport-free validation and sequence
//! generation. Hardware I/O remains in future transport crates.

use crate::embedded::chip_database::{AvrChipInfo, PicChipInfo};
use crate::embedded::protocol_engine::ProtocolEngine;

/// Generic programmer for ISP / ICSP targets (AVR, PIC, …).
#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub struct GenericProgrammer {
    pub(crate) inner: Option<GenericProgrammerInner>,
}

/// Parsed programmer state after successful [`GenericProgrammer`] initialization.
#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub(crate) struct GenericProgrammerInner {
    pub avr: Option<&'static AvrChipInfo>,
    pub pic: Option<&'static PicChipInfo>,
    pub clock_hz: u64,
    #[expect(
        dead_code,
        reason = "stored from config; will be used by voltage-aware programming"
    )]
    pub voltage_mv: u32,
    pub connected: bool,
    pub engine: ProtocolEngine,
}

impl GenericProgrammer {
    /// Create an uninitialized programmer (call `initialize` via trait).
    pub const fn new() -> Self {
        Self { inner: None }
    }
}

impl Default for GenericProgrammer {
    fn default() -> Self {
        Self::new()
    }
}

/// EPROM / parallel ROM programmer (abstract parallel bus protocol).
#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub struct EPROMProgrammer {
    pub(crate) inner: Option<EpromProgrammerInner>,
}

#[derive(Debug)]
#[cfg_attr(
    not(feature = "embedded-placeholder-impls"),
    allow(dead_code, reason = "fields used only when adapter impls are compiled")
)]
pub(crate) struct EpromProgrammerInner {
    pub device_name: String,
    pub size_bytes: u32,
    pub voltage_mv: u32,
    pub connected: bool,
    #[expect(
        dead_code,
        reason = "stored from config; will drive actual EPROM protocol I/O"
    )]
    pub engine: ProtocolEngine,
}

impl EPROMProgrammer {
    /// Create an uninitialized EPROM programmer (call `initialize` via trait).
    pub const fn new() -> Self {
        Self { inner: None }
    }
}

impl Default for EPROMProgrammer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_programmer_new_and_default_agree() {
        let a = GenericProgrammer::new();
        let b = GenericProgrammer::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn eprom_programmer_new_and_default_agree() {
        let a = EPROMProgrammer::new();
        let b = EPROMProgrammer::default();
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
    }

    #[test]
    fn programmer_types_debug_contains_struct_names() {
        let g = format!("{:?}", GenericProgrammer::new());
        let e = format!("{:?}", EPROMProgrammer::new());
        assert!(g.contains("GenericProgrammer"), "{g}");
        assert!(e.contains("EPROMProgrammer"), "{e}");
    }
}
