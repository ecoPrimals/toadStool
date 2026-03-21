// SPDX-License-Identifier: AGPL-3.0-only
//! Type definitions for specialty hardware runtime engine
//!
//! This module contains all type definitions used by the specialty runtime,
//! organized into logical submodules for better maintainability.

#![allow(ambiguous_glob_reexports)]

pub mod configs;
pub mod cross_compilation;
pub mod emulation;
pub mod jobs;
pub mod requirements;
pub mod systems;
pub mod traits;

// Re-export all types for convenient access.
// TerminalType (configs vs jobs) and OptimizationLevel (jobs vs requirements) exist in multiple
// submodules; lib.rs explicitly re-exports the canonical choices (jobs::TerminalType,
// requirements::OptimizationLevel).
pub use configs::*;
pub use cross_compilation::*;
pub use emulation::*;
pub use jobs::*;
pub use requirements::*;
pub use systems::*;
pub use traits::*;
