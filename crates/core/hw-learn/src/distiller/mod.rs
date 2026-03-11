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
    pub vendor: Vendor,
    pub generation: String,
    pub chip: String,
    /// Shader model / compute capability (e.g., "sm70", "gfx1030", "gen12").
    pub compute_class: String,
}

/// GPU vendor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Vendor {
    Amd,
    Intel,
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
    Amdgpu,
    Nouveau,
    NvidiaDrm,
    I915,
    Xe,
    Custom(String),
}

/// A single step in an init recipe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InitStep {
    /// Write a value to a register at the given offset.
    RegisterWrite {
        offset: u64,
        value: u64,
        function: RegFunction,
    },
    /// Make a DRM ioctl call with raw arguments.
    IoctlCall {
        ioctl_nr: u64,
        args: Vec<u8>,
    },
    /// Load firmware for a specific engine.
    FirmwareLoad {
        engine: Engine,
        path: PathBuf,
    },
    /// Wait for a specified duration (microseconds).
    Delay {
        us: u64,
    },
    /// Verify a condition after previous steps.
    Verify {
        check: VerifyCheck,
    },
}

/// Functional classification of a register write.
///
/// The same abstract functions exist across all GPU vendors;
/// only the register addresses differ.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegFunction {
    ClockEnable,
    PowerGate,
    EngineReset,
    ContextAlloc,
    ChannelBind,
    InterruptEnable,
    ThermalConfig,
    MemoryConfig,
    /// Unclassified — the distiller couldn't determine the function.
    Unknown,
}

/// GPU engine/subunit identifier.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Engine {
    Pmu,
    Gsp,
    Acr,
    Gr,
    Ce,
    Sec2,
    GuC,
    HuC,
    Custom(String),
}

/// Verification check to run after applying init steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerifyCheck {
    /// Read a register and check it matches the expected value.
    RegisterMatch { offset: u64, expected: u64, mask: u64 },
    /// Attempt a DRM ioctl and check it succeeds.
    IoctlSucceeds { ioctl_nr: u64 },
    /// Submit a trivial compute dispatch and verify readback.
    ComputeReadback,
}

/// Distiller — converts raw observations into recipes.
pub struct RecipeDistiller;

impl RecipeDistiller {
    /// Distill a recipe from a single observation.
    ///
    /// When `baseline` is provided, the distiller diffs the two traces
    /// to isolate compute-specific operations.
    pub fn distill(
        observation: &ObserveResult,
        baseline: Option<&ObserveResult>,
        target_arch: GpuArch,
    ) -> InitRecipe {
        let compute_events = if let Some(base) = baseline {
            diff::diff_traces(&base.events, &observation.events)
        } else {
            observation.events.clone()
        };

        let classified = classify::classify_events(&compute_events);
        recipe::build_recipe(classified, target_arch, &observation.driver)
    }
}
