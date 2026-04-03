// SPDX-License-Identifier: AGPL-3.0-only
//! V4L2 capture device support via `rustix` — zero C dependencies.
//!
//! Provides safe wrappers around `Video4Linux2` ioctl calls for reading frames
//! from HDMI capture cards (and other V4L2 sources like UVC webcams).
//!
//! # Architecture
//!
//! - `types` — `#[repr(C)]` kernel ABI structs (no unsafe)
//! - `ioctl` — safe wrappers for each V4L2 ioctl (unsafe containment zone)
//! - `device` — `CaptureDevice` API (pure safe Rust)

pub mod device;
mod ioctl;
pub mod types;

pub use device::{CaptureDevice, CaptureFormat, V4l2Capability};
