// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal GPU Compute Engine Implementation

mod defaults;
mod devices;
mod execution;
mod init;
mod meta;
mod runtime_engine;
mod types;

pub use types::UniversalGpuEngine;

#[cfg(test)]
mod tests;
