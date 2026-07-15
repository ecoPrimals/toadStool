// SPDX-License-Identifier: AGPL-3.0-or-later
#![warn(missing_docs)]

//! Pure Rust driver for BrainChip Akida neuromorphic processors
//!
//! This crate provides direct, safe access to Akida AKD1000/AKD1500 neuromorphic
//! processors via the kernel driver at `/dev/akida*`.
//!
//! # Architecture Principles
//!
//! - **Zero Mocks**: Production code only, mocks isolated to tests
//! - **Capability-Based**: Devices discovered at runtime, no hardcoding
//! - **Safe Rust**: Minimal unsafe, encapsulated and documented
//! - **Idiomatic**: Modern Rust patterns, ergonomic API
//! - **Observable**: Comprehensive tracing for debugging
//!
//! # Example
//!
//! ```no_run
//! use akida_driver::{DeviceManager};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Discover devices at runtime
//! let manager = DeviceManager::discover()?;
//! println!("Found {} Akida device(s)", manager.device_count());
//!
//! // Query capabilities (no hardcoding)
//! for device in manager.devices() {
//!     let caps = device.capabilities();
//!     println!("Device {}: {} NPUs, {} MB memory",
//!              device.index(), caps.npu_count, caps.memory_mb);
//! }
//!
//! // Use first available device
//! let mut device = manager.open_first()?;
//! # Ok(())
//! # }
//! ```

#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(
    clippy::must_use_candidate,
    reason = "ergonomic device API — callers choose to use or discard"
)]
#![allow(
    clippy::match_same_arms,
    reason = "explicit arm listing aids hardware register clarity"
)]

mod backend;
pub mod backends;
mod capabilities;
#[cfg(unix)]
mod device;
#[cfg(unix)]
mod discovery;
mod error;
#[cfg(unix)]
mod inference;
#[cfg(unix)]
mod io;
#[cfg(unix)]
mod loading;
#[cfg(target_os = "linux")]
pub mod mmio;
#[cfg(any(test, feature = "test-mocks"))]
mod synthetic;

/// NPU hardware setup and initialization
pub mod setup;

/// Hardware identification constants
pub mod pcie_ids {
    /// BrainChip vendor ID (`0x1E7C`)
    pub const BRAINCHIP_VENDOR_ID: u16 = 0x1E7C;

    /// Supported Akida device IDs
    pub const AKIDA_DEVICE_IDS: &[u16] = &[
        0xBCA1, // AKD1000
        0xBCA2, // AKD1500
    ];

    /// Format vendor:device string for lspci
    pub fn lspci_filter() -> String {
        // Use first device ID for basic filtering (lspci doesn't support multiple)
        format!("{:04x}:{:04x}", BRAINCHIP_VENDOR_ID, AKIDA_DEVICE_IDS[0])
    }
}

pub use backend::{
    BackendSelection, BackendType, ModelHandle, NpuBackend, NpuBackendDispatch, select_backend,
};
#[cfg(unix)]
pub use backends::UserspaceBackend;
pub use capabilities::{
    BatchCapabilities, Capabilities, ChipVersion, ClockMode, MeshTopology, PcieConfig,
    WeightMutationSupport,
};
#[cfg(unix)]
pub use device::{AkidaDevice, DeviceHandle};
#[cfg(unix)]
pub use discovery::{DeviceInfo, DeviceManager};
pub use error::{AkidaError, Result};
#[cfg(unix)]
pub use inference::{InferenceConfig, InferenceExecutor, InferenceResult};
#[cfg(unix)]
pub use loading::{LoadConfig, LoadMetrics, ModelLoader, ModelProgram, NpuConfig};
#[cfg(any(test, feature = "test-mocks"))]
pub use synthetic::SyntheticNpuBackend;

/// Re-export commonly used types
pub mod prelude {
    #[cfg(unix)]
    pub use crate::{
        AkidaDevice, DeviceManager, InferenceConfig, InferenceExecutor, InferenceResult,
        LoadConfig, LoadMetrics, ModelLoader, ModelProgram, NpuConfig,
    };
    pub use crate::{Capabilities, Result};
}
