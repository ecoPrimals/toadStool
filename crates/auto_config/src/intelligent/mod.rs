// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Intelligent Auto-Configuration System
//!
//! Core intelligence layer for `ToadStool`'s zero-touch auto-configuration.
//! This module analyzes system capabilities, detects patterns, and generates
//! optimal configurations automatically.
//!
//! ## Pipeline Architecture
//!
//! This module is organized into 4 pipeline stages:
//! - **detection**: Platform and capability detection (Stage 1)
//! - **analysis**: Pattern recognition and usage learning (Stage 2)
//! - **generation**: Configuration generation (Stage 3)
//! - **validation**: Configuration validation (Stage 4)

pub mod analysis;
pub mod detection;
pub mod generation;
pub mod validation;

#[cfg(feature = "runtime")]
mod auto_config;

// Re-export pipeline types (always available)
pub use analysis::*;
pub use detection::*;
pub use generation::*;
pub use validation::*;

#[cfg(feature = "runtime")]
pub use auto_config::IntelligentAutoConfig;
