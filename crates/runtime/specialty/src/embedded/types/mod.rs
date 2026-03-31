// SPDX-License-Identifier: AGPL-3.0-only
//! Type definitions for embedded systems support
//!
//! This module contains all type definitions for embedded system adapters,
//! including job types, languages, debugging interfaces, and file representations.

mod interfaces;
mod job;
mod toolchain;

pub use interfaces::*;
pub use job::*;
pub use toolchain::*;

#[cfg(test)]
mod tests;
