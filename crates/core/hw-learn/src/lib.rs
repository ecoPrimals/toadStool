// SPDX-License-Identifier: AGPL-3.0-or-later
#![deny(unsafe_code)]
#![warn(missing_docs)]

//! Vendor-neutral GPU hardware learning.
//!
//! Observe working GPUs, distill init patterns into portable recipes,
//! and apply them to unlock compute on firmware-limited hardware.
//!
//! ## Architecture
//!
//! - **observer** — Trace GPU init sequences (mmiotrace, ioctl, GSP RPC, PM4, batch)
//! - **distiller** — Diff and classify traces into minimal init recipes
//! - **knowledge** — Cross-vendor recipe store with arch-aware register mapping
//! - **applicator** — Replay learned recipes on target hardware with verification
//! - **`brain_ext`** — Extensions to toadStool's `PrecisionBrain` (`LearningAdvisor`, `FirmwareInventory`)
//!
//! ## Design Principles
//!
//! 1. Vendor-neutral from day one — AMD, Intel, NVIDIA share the same abstractions.
//! 2. Pattern-based — the universal compute init skeleton (probe → firmware → power →
//!    reset → context → bind → verify) is vendor-invariant; only register addresses differ.
//! 3. Every working GPU is a teacher — fully open drivers (amdgpu, i915/xe) are gold
//!    standard; proprietary drivers are observable via UVM/RM.

pub mod applicator;
pub mod brain_ext;
pub mod distiller;
pub mod knowledge;
pub mod observer;

pub use applicator::{ApplyResult, ApplyVerdict, RecipeApplicator, RegisterAccess};
pub use brain_ext::{
    CapabilityGap, FirmwareInventory, FirmwareInventoryExt, FwStatus, LearningAdvisor,
    LearningOpportunity,
};
pub use distiller::{GpuGen, InitRecipe, InitStep, RecipeDistiller, RegFunction};
pub use knowledge::{ArchId, KnowledgeStore, RecipeId};
pub use observer::{ObserveConfig, ObserveResult, TraceEvent, TraceObserver};
