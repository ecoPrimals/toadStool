//! Akida Reservoir Computing Research Library
//!
//! Experimental crate for exploring reservoir computing (echo state networks)
//! on BrainChip Akida neuromorphic hardware.
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

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(dead_code)] // Research crate, many components not yet used

pub mod reservoir;
pub mod state_extraction;
pub mod readout;
pub mod ensemble;

pub use reservoir::{ReservoirConfig, ReservoirGenerator};
pub use state_extraction::{StateExtractor, LayerActivations};
pub use readout::{ReadoutTrainer, ReadoutPredictor};
pub use ensemble::{DualChipEnsemble, EnsembleConfig};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{
        ReservoirConfig,
        ReservoirGenerator,
        StateExtractor,
        LayerActivations,
        ReadoutTrainer,
        ReadoutPredictor,
        DualChipEnsemble,
        EnsembleConfig,
    };
}
