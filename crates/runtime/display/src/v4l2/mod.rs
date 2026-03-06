// SPDX-License-Identifier: AGPL-3.0-or-later
//! V4L2 capture device support via `rustix` — zero C dependencies.
//!
//! Provides safe wrappers around `Video4Linux2` ioctl calls for reading frames
//! from HDMI capture cards (and other V4L2 sources like UVC webcams).

pub mod device;

pub use device::{CaptureDevice, CaptureFormat, V4l2Capability};
