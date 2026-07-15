// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Akida Reservoir Computing Research Library
//!
//! Experimental crate for exploring reservoir computing (echo state networks)
//! on `BrainChip` Akida neuromorphic hardware.
//!
//! # Research Questions
//!
//! 1. Can we extract internal NPU layer activations?
//! 2. Does Akida support recurrent architectures for echo state dynamics?
//! 3. Can we implement dual-chip ensemble reservoirs?
//! 4. What is the end-to-end latency vs traditional approaches?
//!
//! # Architecture
//!
//! ```text
//! Input → [Reservoir (Akida)] → Readout (CPU) → Output
//!         ↑ Random, fixed      ↑ Trained
//!         80 NPUs per chip     Linear regression
//! ```

#![allow(
    clippy::must_use_candidate,
    reason = "ergonomic research API — callers choose to use or discard"
)]
#![allow(
    dead_code,
    reason = "research crate: components wired incrementally as experiments mature"
)]

#[cfg(unix)]
pub mod ensemble;
pub mod error;
pub mod readout;
pub mod reservoir;
#[cfg(unix)]
pub mod state_extraction;

#[cfg(unix)]
pub use ensemble::{DualChipEnsemble, EnsembleConfig};
pub use error::{ReservoirError, Result as ReservoirResult};
pub use readout::{ReadoutPredictor, ReadoutTrainer};
pub use reservoir::{ReservoirConfig, ReservoirGenerator};
#[cfg(unix)]
pub use state_extraction::{LayerActivations, StateExtractor};

/// Re-export commonly used types
pub mod prelude {
    #[cfg(unix)]
    pub use crate::{DualChipEnsemble, EnsembleConfig, LayerActivations, StateExtractor};
    pub use crate::{
        ReadoutPredictor, ReadoutTrainer, ReservoirConfig, ReservoirError, ReservoirGenerator,
    };
}
