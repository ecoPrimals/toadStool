// SPDX-License-Identifier: AGPL-3.0-or-later
/// Substrate type definitions (biological, quantum, neuromorphic, etc.).
pub mod substrate;
mod types;

pub use substrate::*;
pub use types::*;

#[cfg(feature = "runtime")]
/// Universal adapter for cross-platform execution.
pub mod adapter;
#[cfg(feature = "runtime")]
/// Platform detection and capabilities.
pub mod platform;
#[cfg(feature = "runtime")]
/// Universal scheduler for job distribution.
pub mod scheduler;
#[cfg(feature = "runtime")]
mod detection;

#[cfg(feature = "runtime")]
pub use adapter::*;
#[cfg(feature = "runtime")]
pub use platform::*;
#[cfg(feature = "runtime")]
pub use scheduler::*;
