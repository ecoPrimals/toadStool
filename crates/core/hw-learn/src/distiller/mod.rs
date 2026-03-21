// SPDX-License-Identifier: AGPL-3.0-only
//! Distill raw trace observations into minimal, portable init recipes.
//!
//! The distiller takes `ObserveResult` from the observer and produces
//! an `InitRecipe`: the smallest ordered set of operations needed to
//! initialize a GPU compute engine.
//!
//! ## Pipeline
//!
//! 1. **Diff** — compare compute-on vs compute-off traces to isolate
//!    compute-specific register writes.
//! 2. **Classify** — tag each register write by function (clock, power,
//!    engine reset, context alloc, channel bind, interrupt).
//! 3. **Recipe** — assemble the minimal ordered init sequence.

pub mod classify;
pub mod diff;
pub mod recipe;

pub use classify::GpuGen;
pub use diff::diff_traces;
pub use recipe::build_recipe;

use crate::observer::ObserveResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A vendor-neutral compute init recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRecipe {
    /// Architecture of the GPU that produced this recipe.
    pub source_arch: GpuArch,
    /// Driver used during observation.
    pub source_driver: DriverKind,
    /// Target architecture this recipe is intended for.
    pub target_arch: GpuArch,
    /// Ordered init operations.
    pub steps: Vec<InitStep>,
    /// Confidence score from validation (0.0 = untested, 1.0 = fully validated).
    pub confidence: f64,
    /// Human-readable description.
    pub description: String,
}

/// GPU architecture identifier — vendor + generation + chip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GpuArch {
    /// GPU vendor (AMD, Intel, NVIDIA).
    pub vendor: Vendor,
    /// Marketing generation (e.g., "RDNA2", "Volta", "Ada").
    pub generation: String,
    /// Chip codename (e.g., "Navi21", "GV100", "AD104").
    pub chip: String,
    /// Shader model / compute capability (e.g., "sm70", "gfx1030", "gen12").
    pub compute_class: String,
}

/// GPU vendor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Vendor {
    /// AMD (RDNA, GCN, CDNA).
    Amd,
    /// Intel (Gen9+, Xe, Arc).
    Intel,
    /// NVIDIA (Maxwell through Ada).
    Nvidia,
}

impl std::fmt::Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amd => f.write_str("AMD"),
            Self::Intel => f.write_str("Intel"),
            Self::Nvidia => f.write_str("NVIDIA"),
        }
    }
}

/// Driver kind — needed to know what ioctl interface to use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DriverKind {
    /// AMD amdgpu (open-source).
    Amdgpu,
    /// NVIDIA nouveau (open-source).
    Nouveau,
    /// NVIDIA proprietary nvidia-drm.
    NvidiaDrm,
    /// Intel i915 (legacy).
    I915,
    /// Intel Xe (modern).
    Xe,
    /// Custom or unknown driver.
    Custom(String),
}

/// A single step in an init recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InitStep {
    /// Write a value to a register at the given offset.
    RegisterWrite {
        /// BAR-relative MMIO offset.
        offset: u64,
        /// Value to write (32- or 64-bit depending on register).
        value: u64,
        /// Functional classification of this write.
        function: RegFunction,
    },
    /// Make a DRM ioctl call with raw arguments.
    IoctlCall {
        /// DRM ioctl number.
        ioctl_nr: u64,
        /// Raw argument bytes.
        args: Vec<u8>,
    },
    /// Load firmware for a specific engine.
    FirmwareLoad {
        /// Target engine (PMU, GSP, MEC, etc.).
        engine: Engine,
        /// Path to firmware blob.
        path: PathBuf,
    },
    /// Wait for a specified duration (microseconds).
    Delay {
        /// Microseconds to wait.
        us: u64,
    },
    /// Verify a condition after previous steps.
    Verify {
        /// Check to run.
        check: VerifyCheck,
    },
}

/// Functional classification of a register write.
///
/// The same abstract functions exist across all GPU vendors;
/// only the register addresses differ.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegFunction {
    /// Clock gating / enable.
    ClockEnable,
    /// Power domain control.
    PowerGate,
    /// Engine soft/hard reset.
    EngineReset,
    /// Context/queue allocation.
    ContextAlloc,
    /// Channel/ring binding.
    ChannelBind,
    /// Interrupt enable/ack.
    InterruptEnable,
    /// Thermal sensor/throttle config.
    ThermalConfig,
    /// Memory controller / page table config.
    MemoryConfig,
    /// Unclassified — the distiller couldn't determine the function.
    Unknown,
}

/// GPU engine/subunit identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    /// Power Management Unit.
    Pmu,
    /// GPU System Processor (NVIDIA).
    Gsp,
    /// Acoustic Cover Removal / ACR (NVIDIA).
    Acr,
    /// Graphics/compute engine.
    Gr,
    /// Copy Engine.
    Ce,
    /// Security engine 2 (NVIDIA Turing+).
    Sec2,
    /// Graphics micro-controller (Intel).
    GuC,
    /// Hardware micro-controller (Intel).
    HuC,
    /// Custom engine name.
    Custom(String),
}

/// Verification check to run after applying init steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerifyCheck {
    /// Read a register and check it matches the expected value.
    RegisterMatch {
        /// BAR-relative register offset to read.
        offset: u64,
        /// Expected value after masking.
        expected: u64,
        /// Bitmask to apply before comparing.
        mask: u64,
    },
    /// Attempt a DRM ioctl and check it succeeds.
    IoctlSucceeds {
        /// DRM ioctl number to invoke.
        ioctl_nr: u64,
    },
    /// Submit a trivial compute dispatch and verify readback.
    ComputeReadback,
    /// Verify a memory region is accessible via sentinel write/readback.
    /// Used by the differential probe to confirm FB/HBM2 init steps.
    MemoryAccessible {
        /// Aperture name (e.g. "VRAM", "`SysMem`", "BAR2").
        aperture: String,
        /// Byte offset within the aperture to test.
        offset: u64,
        /// Sentinel value to write and expect back.
        sentinel: u64,
    },
}

/// Distiller — converts raw observations into recipes.
pub struct RecipeDistiller;

impl RecipeDistiller {
    /// Distill a recipe from a single observation.
    ///
    /// When `baseline` is provided, the distiller diffs the two traces
    /// to isolate compute-specific operations.
    #[must_use]
    pub fn distill(
        observation: &ObserveResult,
        baseline: Option<&ObserveResult>,
        target_arch: GpuArch,
    ) -> InitRecipe {
        let compute_events = baseline.map_or_else(
            || observation.events.clone(),
            |base| diff::diff_traces(&base.events, &observation.events),
        );

        let classified = classify::classify_events(&compute_events, Some(&target_arch.chip));
        recipe::build_recipe(classified, target_arch, &observation.driver)
    }
}
