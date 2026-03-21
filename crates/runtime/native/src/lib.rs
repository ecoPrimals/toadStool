// SPDX-License-Identifier: AGPL-3.0-only
//! Native process runtime for executing native binaries as child processes

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::no_effect_underscore_binding
)]

mod capabilities;
mod engine;
mod process;
mod security;
mod validation;

#[cfg(test)]
mod tests;

/// Native runtime engine for native workload execution
pub use engine::NativeRuntimeEngine;
