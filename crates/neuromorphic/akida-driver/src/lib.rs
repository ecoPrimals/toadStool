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

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_same_arms)]

mod capabilities;
mod device;
mod discovery;
mod error;
mod inference;
mod io;
mod loading;

pub use capabilities::{Capabilities, ChipVersion, PcieConfig};
pub use device::{AkidaDevice, DeviceHandle};
pub use discovery::{DeviceInfo, DeviceManager};
pub use error::{AkidaError, Result};
pub use inference::{InferenceConfig, InferenceExecutor, InferenceResult};
pub use loading::{LoadConfig, LoadMetrics, ModelLoader, ModelProgram, NpuConfig};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        AkidaDevice, Capabilities, DeviceManager, InferenceConfig, InferenceExecutor,
        InferenceResult, LoadConfig, LoadMetrics, ModelLoader, ModelProgram, NpuConfig, Result,
    };
}
