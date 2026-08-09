// SPDX-License-Identifier: AGPL-3.0-or-later
//! Encryption types — security levels, payloads, config.
//! Pure data structures (no async, no I/O) suitable for WASM targets.

/// Security level definitions for encryption operations.
pub mod security;
/// Core encryption types: payloads, keys, metadata.
pub mod types;
/// Encryption configuration and input/output wrappers.
pub mod config;
/// Cryptographic error types.
pub mod error;

pub use security::SecurityLevel;
pub use types::{EncryptedPayload, EncryptionKey, EncryptionMetadata};
pub use config::{EncryptedInput, EncryptedOutput, EncryptionConfig};
pub use error::CryptoError;
