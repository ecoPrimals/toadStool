// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pure Rust driver for `BrainChip` Akida neuromorphic processors.
//!
//! This crate provides the full software stack for AKD1000 / AKD1500 access.
//! No Python. No C++ SDK. No vendor `MetaTF`.
//!
//! # Backend hierarchy
//!
//! ```text
//! Primary (no kernel module required):
//!   VfioBackend  — VFIO/IOMMU + full DMA (preferred for production)
//!
//! Fallback (when C akida_pcie module is loaded):
//!   KernelBackend — /dev/akida* read/write
//!
//! Development:
//!   UserspaceBackend — BAR mmap, no DMA
//! ```
//!
//! # Quick start
//!
//! ```no_run
//! use akida_driver::DeviceManager;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mgr  = DeviceManager::discover()?;
//! let caps = mgr.devices()[0].capabilities();
//!
//! println!("{:?} — {} NPs, {} MB SRAM, PCIe Gen{} x{}",
//!          caps.chip_version, caps.npu_count, caps.memory_mb,
//!          caps.pcie.generation, caps.pcie.lanes);
//!
//! let model_bytes = std::fs::read("model.fbz")?;
//! let mut dev = mgr.open_first()?;
//! dev.write(&model_bytes)?;
//! let mut out = vec![0u8; 1024];
//! dev.read(&mut out)?;
//! # Ok(())
//! # }
//! ```
//!
//! # Measured results (AKD1000, `PCIe` x1 Gen2, Feb 2026)
//!
//! | Metric | Value |
//! |--------|-------|
//! | DMA throughput (sustained) | 37 MB/s |
//! | Single inference | 54 µs / 18,500 Hz |
//! | Batch=8 | 390 µs/sample / 20,700 /s |
//! | Energy per inference | 1.4 µJ |
//! | 24-hour production calls (Exp 022) | 5,978 |

#![warn(clippy::expect_used, clippy::unwrap_used)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]

// Platform-agnostic modules (available on all targets)
mod backend;
pub mod backends;
mod capabilities;
mod error;
pub mod evolution;
pub mod hybrid;
pub mod sentinel;
mod synthetic;

// Hardware modules — Linux-only (VFIO/sysfs kernel interfaces).
#[cfg(target_os = "linux")]
mod device;
#[cfg(target_os = "linux")]
mod discovery;
#[cfg(target_os = "linux")]
pub mod glowplug;
#[cfg(target_os = "linux")]
mod inference;
#[cfg(target_os = "linux")]
mod io;
#[cfg(target_os = "linux")]
mod loading;
#[cfg(target_os = "linux")]
pub mod mmio;
#[cfg(target_os = "linux")]
pub mod puf;
#[cfg(target_os = "linux")]
pub mod setup;
#[cfg(target_os = "linux")]
pub mod sram;
#[cfg(target_os = "linux")]
pub mod tenancy;
#[cfg(target_os = "linux")]
pub mod vfio;

/// Hardware identification constants (re-exported from akida-chip).
pub mod pcie_ids {
    pub use akida_chip::pcie::device_id;
    pub use akida_chip::pcie::{
        ALL_DEVICE_IDS, BRAINCHIP_VENDOR_ID, ChipVariant, MEASURED_DMA_THROUGHPUT_MB_S,
        OPTIMAL_BATCH_SIZE, PCIE_GEN2_X1_ROUNDTRIP_US, lspci_filter,
    };
}

#[cfg(target_os = "linux")]
pub use backend::select_backend;
pub use backend::{BackendSelection, BackendType, LoadVerification, ModelHandle, NpuBackend};
#[cfg(target_os = "linux")]
pub use backends::UserspaceBackend;
pub use backends::software::{SoftwareBackend, pack_software_model};
pub use capabilities::{
    BatchCapabilities, Capabilities, ChipVersion, ClockMode, MeshTopology, PcieConfig,
    WeightMutationSupport,
};
#[cfg(target_os = "linux")]
pub use device::{AkidaDevice, DeviceHandle};
#[cfg(target_os = "linux")]
pub use discovery::{DeviceInfo, DeviceManager};
pub use error::{AkidaError, Result};
pub use hybrid::{
    EsnSubstrate, EsnWeights, HybridEsn, SubstrateInfo, SubstrateMode, SubstrateSelector,
};
#[cfg(target_os = "linux")]
pub use inference::{InferenceConfig, InferenceExecutor, InferenceResult};
#[cfg(target_os = "linux")]
pub use loading::{LoadConfig, LoadMetrics, ModelLoader, ModelProgram, NpuConfig};
#[cfg(target_os = "linux")]
pub use vfio::VfioBackend;

#[cfg(any(test, feature = "test-mocks"))]
pub use synthetic::SyntheticNpuBackend;

/// Commonly used types.
pub mod prelude {
    pub use crate::{
        AkidaError, Capabilities, EsnSubstrate, EsnWeights, HybridEsn, Result, SubstrateMode,
        SubstrateSelector,
    };

    #[cfg(target_os = "linux")]
    pub use crate::{
        AkidaDevice, DeviceManager, InferenceConfig, InferenceExecutor, InferenceResult,
        LoadConfig, ModelLoader, ModelProgram, NpuConfig, VfioBackend,
    };
}
