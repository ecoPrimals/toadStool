// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native process runtime for executing native binaries as child processes

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::no_effect_underscore_binding,
    reason = "async trait impls use _prefixed params for forward-compat"
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
