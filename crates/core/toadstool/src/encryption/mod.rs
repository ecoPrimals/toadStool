// SPDX-License-Identifier: AGPL-3.0-or-later
//! Encryption Layer for ToadStool
//!
//! **Design Philosophy**:
//! - Capability-based: Discover crypto providers at runtime
//! - Self-knowledge: Toadstool knows it can execute, not who provides crypto
//! - Zero hardcoding: No URLs, ports, or specific primal names
//! - Modern Rust: Strong types, zero-copy where possible
//! - Graceful degradation: Works without encryption

mod builder;
mod context;

pub mod capability;
pub mod provider;

#[cfg(test)]
mod tests;

// Re-export pure types from toadstool-core
pub use toadstool_core::encryption::*;

pub use builder::EncryptionContextBuilder;
pub use capability::CryptoCapability;
pub use context::EncryptionContext;
pub use provider::{CryptoProvider, CryptoProviderRegistry, NoopCryptoProvider};
