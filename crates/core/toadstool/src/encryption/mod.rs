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
mod config;
mod context;
pub mod error;
mod security;

pub mod capability;
pub mod provider;
pub mod types;

#[cfg(test)]
mod tests;

pub use builder::EncryptionContextBuilder;
pub use capability::CryptoCapability;
pub use config::{EncryptedInput, EncryptedOutput, EncryptionConfig};
pub use context::EncryptionContext;
pub use error::CryptoError;
pub use provider::{CryptoProvider, CryptoProviderRegistry, NoopCryptoProvider};
pub use security::SecurityLevel;
pub use types::{EncryptedPayload, EncryptionKey, EncryptionMetadata};
