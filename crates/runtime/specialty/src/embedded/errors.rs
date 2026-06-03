// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed errors for embedded programmer / emulator placeholder paths.
//!
//! Transport (USB, serial, parallel) stays unimplemented until hardware lands;
//! these variants classify *why* an operation cannot proceed without stringly
//! placeholders.

/// Failure modes for [`super::types::ProgrammerInterface`] when no backend is wired.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbeddedProgrammerError {
    /// Placeholder programmer adapters were not registered (feature disabled).
    #[error(
        "embedded placeholder programmer adapters not registered; enable `embedded-placeholder-impls` feature"
    )]
    AdapterNotRegistered,

    /// Serial / parallel / USB transport was never configured for this interface.
    #[error("transport not configured for interface `{interface}`")]
    TransportNotConfigured {
        /// Human-readable interface label (e.g. `ISP`, `Parallel`).
        interface: &'static str,
    },
    /// No target device is connected on the configured link.
    #[error("device not connected: {device_id}")]
    DeviceNotConnected {
        /// Stable id for logs (often the platform id until enumeration exists).
        device_id: String,
    },
    /// Signature / device id does not match any known entry in the built-in table.
    #[error("unknown chip signature 0x{signature:06x} for family {family}")]
    ChipSignatureUnknown {
        /// Raw signature (e.g. AVR 3-byte id in low 24 bits).
        signature: u32,
        /// Family hint from config (e.g. `avr`, `pic`).
        family: String,
    },
    /// Timing violates documented programmer limits.
    #[error("invalid ISP timing: {detail}")]
    TimingInvalid {
        /// What failed (e.g. clock out of range).
        detail: String,
    },
    /// Programming voltage / I/O level inconsistent with chip family.
    #[error("voltage level incompatible with {chip_family}: {detail}")]
    VoltageIncompatible {
        /// Rough device class (e.g. `5v-avr`).
        chip_family: &'static str,
        /// Short explanation.
        detail: String,
    },
    /// Address range does not fit in target flash or EEPROM map.
    #[error(
        "address range out of bounds: address=0x{address:x} length={length} (limit 0x{limit:x})"
    )]
    AddressOutOfRange {
        /// Start address.
        address: u32,
        /// Length in bytes.
        length: u32,
        /// Exclusive end or max address for region.
        limit: u32,
    },
    /// Data length or alignment invalid for the protocol (e.g. page writes).
    #[error("invalid programming data: {detail}")]
    DataLayoutInvalid {
        /// What went wrong.
        detail: String,
    },
    /// Chip or connection configuration could not be parsed or is incomplete.
    #[error("invalid programmer configuration: {detail}")]
    ConfigurationInvalid {
        /// Human-readable detail.
        detail: String,
    },
    /// Requested operation not supported for this chip (e.g. EEPROM erase on part without EEPROM).
    #[error("operation not supported for chip: {detail}")]
    OperationNotSupported {
        /// Explanation.
        detail: String,
    },
}

/// Failure modes for [`super::types::EmbeddedEmulator`] when no CPU core is present.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmbeddedEmulatorError {
    /// Placeholder emulator adapters were not registered (feature disabled).
    #[error(
        "embedded placeholder emulator adapters not registered; enable `embedded-placeholder-impls` feature"
    )]
    AdapterNotRegistered,

    /// Emulator core / memory map not loaded for this platform build.
    #[error("CPU core not available for platform `{platform}`")]
    CoreNotAvailable {
        /// Platform id (e.g. `mos6502`, `z80`).
        platform: &'static str,
    },
    /// Debug / control transport (e.g. GDB stub) not configured.
    #[error("debug transport not configured for interface `{interface}`")]
    TransportNotConfigured {
        /// Interface label.
        interface: &'static str,
    },
    /// Emulator used before `initialize` / `load_rom` completed.
    #[error("emulator not ready: {detail}")]
    NotReady {
        /// What is missing.
        detail: String,
    },
}
