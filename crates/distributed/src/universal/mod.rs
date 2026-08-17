// SPDX-License-Identifier: AGPL-3.0-or-later
/// Substrate type definitions (biological, quantum, neuromorphic, etc.).
pub mod substrate;
mod types;

pub use substrate::*;
pub use types::*;

#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
/// Universal adapter for cross-platform execution.
pub mod adapter;
#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
mod detection;
#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
/// Platform detection and capabilities.
pub mod platform;
#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
/// Universal scheduler for job distribution.
pub mod scheduler;

#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
pub use adapter::*;
#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
pub use platform::*;
#[cfg(all(feature = "runtime", feature = "legacy-scheduler"))]
pub use scheduler::*;
