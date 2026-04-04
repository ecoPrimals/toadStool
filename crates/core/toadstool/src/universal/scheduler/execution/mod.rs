// SPDX-License-Identifier: AGPL-3.0-only
//! Job execution backends for `UniversalScheduler`
//!
//! Handles execution routing for Native, WASM, Primal, and BiomeOS job types.

mod biome_os;
mod discover;
mod native;
mod primal;
mod wasm;

#[cfg(test)]
mod tests;
