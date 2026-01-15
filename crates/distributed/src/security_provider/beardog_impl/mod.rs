//! BearDog SecurityProvider Implementation
//!
//! BearDog primal implementation of the generic SecurityProvider trait.
//! This is ONE of many possible implementations (HSM, KMS, local keyring, etc.).
//!
//! ## Philosophy: "BearDog is ONE Option, Not THE Option"
//!
//! - BearDog implements the generic SecurityProvider trait
//! - Code uses SecurityProvider trait, not BearDogSecurityProvider directly
//! - Can be swapped with other implementations at runtime
//! - Discovered via Universal Adapter, not hardcoded
//!
//! ## Deep Debt Compliance
//!
//! - ✅ No hardcoding in consumers (they use SecurityProvider trait)
//! - ✅ Runtime discovery (Universal Adapter finds BearDog)
//! - ✅ Self-knowledge (BearDog knows itself, not what others need)
//! - ✅ Pluggable (can swap for HSM/KMS/etc.)

// Allow deprecated during migration - beardog_integration will be evolved
#![allow(deprecated)]

pub mod client;
pub mod adapters;

#[cfg(test)]
mod tests;

pub use client::BearDogSecurityProvider;
pub use adapters::*;
