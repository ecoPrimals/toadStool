// SPDX-License-Identifier: AGPL-3.0-or-later
//! Encryption types — security levels, payloads, config.
//! Pure data structures (no async, no I/O) suitable for WASM targets.

pub mod security;
pub mod types;
pub mod config;
pub mod error;

pub use security::SecurityLevel;
pub use types::{EncryptedPayload, EncryptionKey, EncryptionMetadata};
pub use config::{EncryptedInput, EncryptedOutput, EncryptionConfig};
pub use error::CryptoError;
