// SPDX-License-Identifier: AGPL-3.0-or-later
//! Internal Lifecycle Operations for Biome Management
//!
//! Split into submodules by concern:
//! - `start` — biome/primal/service startup, workload conversion
//! - `stop`  — graceful/force shutdown, purge, signal handling

mod start;
mod stop;

#[cfg(test)]
mod tests;
