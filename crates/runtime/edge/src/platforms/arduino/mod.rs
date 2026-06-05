// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Arduino Platform Support
//!
//! Implementation of Arduino board support for ToadStool Edge Runtime.
//! Supports various Arduino boards with serial communication and code deployment.

mod deploy;
mod device;
mod edge_device;
#[cfg(feature = "serial-transport")]
mod serial;
#[cfg(not(feature = "serial-transport"))]
mod serial_stub;

pub use device::ArduinoDevice;
