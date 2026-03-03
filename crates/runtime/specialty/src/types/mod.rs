// SPDX-License-Identifier: AGPL-3.0-or-later
//! Type definitions for specialty hardware runtime engine
//!
//! This module contains all type definitions used by the specialty runtime,
//! organized into logical submodules for better maintainability.

pub mod systems;
pub mod jobs;
pub mod requirements;
pub mod configs;
pub mod traits;
pub mod cross_compilation;
pub mod emulation;

// Re-export all types for convenient access
pub use systems::*;
pub use jobs::*;
pub use requirements::*;
pub use configs::*;
pub use traits::*;
pub use cross_compilation::*;
pub use emulation::*;

