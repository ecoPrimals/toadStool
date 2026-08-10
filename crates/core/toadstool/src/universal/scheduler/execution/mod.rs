// SPDX-License-Identifier: AGPL-3.0-or-later
//! Job execution backends for `UniversalScheduler`
//!
//! Handles execution routing for Native, WASM, Primal, and BiomeOS job types.

mod biome_os;
mod discover;
#[cfg(feature = "runtime")]
mod native;
mod primal;
mod wasm;

#[cfg(test)]
mod tests;
