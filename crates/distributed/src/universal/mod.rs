// SPDX-License-Identifier: AGPL-3.0-only
/// Universal adapter for cross-platform execution.
pub mod adapter;
/// Platform detection and capabilities.
pub mod platform;
/// Universal scheduler for job distribution.
pub mod scheduler;

// Universal substrate modules
mod detection;
/// Substrate detection (biological, quantum, neuromorphic, etc.).
pub mod substrate;
mod types;

pub use adapter::*;
pub use platform::*;
pub use scheduler::*;
pub use substrate::*;
