// SPDX-License-Identifier: AGPL-3.0-or-later

use uuid::Uuid;

use super::config::EncryptionConfig;
use super::context::EncryptionContext;
use super::security::SecurityLevel;

/// Builder for encryption contexts
///
/// **Design**: Fluent API, modern Rust idioms
pub struct EncryptionContextBuilder {
    execution_id: Uuid,
    config: EncryptionConfig,
}

impl EncryptionContextBuilder {
    /// Creates a new builder for the given execution ID.
    pub fn new(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            config: EncryptionConfig::default(),
        }
    }

    /// Sets whether encryption is required (fail if unavailable).
    pub const fn required(mut self, required: bool) -> Self {
        self.config.required = required;
        self
    }

    /// Sets whether execution results should be encrypted.
    pub const fn encrypt_results(mut self, encrypt: bool) -> Self {
        self.config.encrypt_results = encrypt;
        self
    }

    /// Sets the minimum security level.
    pub const fn security_level(mut self, level: SecurityLevel) -> Self {
        self.config.min_security_level = level;
        self
    }

    /// Sets the key ID for encryption.
    pub fn key_id(mut self, key_id: impl Into<String>) -> Self {
        self.config.key_id = Some(key_id.into());
        self
    }

    /// Sets preferred encryption algorithms in order.
    pub fn algorithms(mut self, algorithms: Vec<String>) -> Self {
        self.config.preferred_algorithms = algorithms;
        self
    }

    /// Builds the encryption context.
    pub fn build(self) -> EncryptionContext {
        EncryptionContext::new(self.execution_id, self.config)
    }
}
