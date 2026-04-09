// SPDX-License-Identifier: AGPL-3.0-or-later
//! Security SecurityProvider Implementation
//!
//! Default in-process implementation of the generic `SecurityProvider` trait.
//! This is ONE of many possible implementations (HSM, KMS, local keyring, etc.).
//!
//! ## Philosophy: "Security is ONE Option, Not THE Option"
//!
//! - Security implements the generic SecurityProvider trait
//! - Code uses SecurityProvider trait, not DistributedSecurityProvider directly
//! - Can be swapped with other implementations at runtime
//! - Discovered via Universal Adapter, not hardcoded
//!
//! ## Deep Debt Compliance
//!
//! - ✅ No hardcoding in consumers (they use SecurityProvider trait)
//! - ✅ Runtime discovery (Universal Adapter finds Security)
//! - ✅ Self-knowledge (Security knows itself, not what others need)
//! - ✅ Pluggable (can swap for HSM/KMS/etc.)

// Allow deprecated during migration - security will be evolved
#![expect(deprecated, reason = "security provider migration in progress")]

pub mod adapters;
pub mod client;

#[cfg(test)]
mod tests;

pub use adapters::*;
pub use client::DistributedSecurityProvider;
