// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based service discovery and invocation
//!
//! Deep Debt Solution: Primals discover each other by capability at runtime,
//! not by hardcoded names. This enables true ecosystem agnosticism.
//!
//! Philosophy: "Know thyself, discover others"

mod discovery;
mod error;
mod provider;
mod serialize;

// Re-export public API for backward compatibility
#[cfg(unix)]
pub use discovery::discover_all;
pub use error::{CapabilityError, Result};
pub use provider::CapabilityProvider;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
