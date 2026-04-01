// SPDX-License-Identifier: AGPL-3.0-only

//! Firmware boundary — the interface between toadStool and opaque firmware.
//!
//! [`FirmwareInterface`] marks the boundary below which toadStool does not
//! rewrite. Firmware (FECS, GPCCS, PMU, SEC2, UEFI/BIOS, NPU microcode,
//! HSM secure elements) runs on its own processor and is treated as an
//! external system that we **interface with**, not **replace**.
//!
//! This is the same relationship as UEFI: the firmware runs on its own
//! execution context, we send commands and read status through a defined
//! interface (registers, mailboxes, IPC). The transport underneath may use
//! unsafe wrappers (MMIO, ioctl), but the firmware interface itself is safe.
//!
//! # Concrete Boundaries
//!
//! - **GPU FECS/GPCCS/PMU**: Falcon engines on GPU die — accessed via BAR0
//!   registers through `toadstool-hw-safe` (direct MMIO reads).
//! - **UEFI/BIOS**: Platform firmware — accessed via ACPI/sysfs (already safe).
//! - **Akida NPU**: Neuromorphic microcode — accessed via MMIO registers.
//! - **HSM/TEE**: Security processor firmware — accessed via ioctl/sysfs.
//! - **USB controller**: xHCI firmware — accessed via standard USB stack.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The interface between toadStool and opaque firmware running on a device.
///
/// Everything below this trait is firmware that we interface with but do not
/// reimplement. Everything above is pure Rust. The trait itself is safe —
/// any unsafe transport (MMIO, ioctl) is encapsulated in the implementation.
pub trait FirmwareInterface: Send + Sync + fmt::Debug {
    /// Status snapshot returned by [`probe_status`](FirmwareInterface::probe_status).
    type Status: fmt::Debug + Serialize + for<'de> Deserialize<'de>;

    /// Command that can be sent to the firmware.
    type Command: fmt::Debug;

    /// Error type for firmware operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Probe the firmware for its current status.
    ///
    /// # Errors
    ///
    /// Returns an error if the firmware is unreachable or the probe fails.
    fn probe_status(&self) -> Result<Self::Status, Self::Error>;

    /// Send a command to the firmware.
    ///
    /// # Errors
    ///
    /// Returns an error if the command is rejected or the firmware is
    /// unreachable.
    fn send_command(&self, cmd: Self::Command) -> Result<(), Self::Error>;

    /// The firmware version string, if discoverable.
    fn firmware_version(&self) -> Option<String>;

    /// Whether the firmware is currently responsive to probes.
    fn is_responsive(&self) -> bool;

    /// Human-readable name for this firmware engine (e.g. `"FECS"`, `"PMU"`,
    /// `"UEFI"`, `"Akida-v2"`).
    fn engine_name(&self) -> &str;
}

/// Firmware status when the engine is not present or not applicable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoFirmware;

/// Null firmware interface for devices that have no firmware boundary
/// (e.g. pure memory, simple GPIO).
#[derive(Debug)]
pub struct NoFirmwareInterface;

impl FirmwareInterface for NoFirmwareInterface {
    type Status = NoFirmware;
    type Command = ();
    type Error = std::convert::Infallible;

    fn probe_status(&self) -> Result<Self::Status, Self::Error> {
        Ok(NoFirmware)
    }

    fn send_command(&self, _cmd: Self::Command) -> Result<(), Self::Error> {
        Ok(())
    }

    fn firmware_version(&self) -> Option<String> {
        None
    }

    fn is_responsive(&self) -> bool {
        true
    }

    fn engine_name(&self) -> &'static str {
        "none"
    }
}
