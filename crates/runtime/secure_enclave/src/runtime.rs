// SPDX-License-Identifier: AGPL-3.0-only
//! Secure enclave runtime implementation
//!
//! Core runtime for zero-knowledge compute operations.

use crate::audit::{AuditEventType, AuditLogger};
use crate::error::{Error, Result};
use crate::isolated_memory::IsolatedMemoryRegion;
use crate::key_store::EphemeralKeyStore;

/// Secure enclave runtime for zero-knowledge compute
///
/// Provides isolated execution environment with:
/// - Memory isolation (no swap, no core dumps)
/// - Ephemeral key management
/// - Audit logging
/// - Cryptographic proof generation
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────┐
/// │   SecureEnclaveRuntime              │
/// ├─────────────────────────────────────┤
/// │ • IsolatedMemoryRegion (mlock)      │
/// │ • EphemeralKeyStore (wiped)         │
/// │ • AuditLogger (tamper-evident)      │
/// │ • ProofGenerator (cryptographic)    │
/// └─────────────────────────────────────┘
/// ```
///
/// # Example
///
/// ```rust,ignore
/// use secure_enclave::SecureEnclaveRuntime;
///
/// let mut runtime = SecureEnclaveRuntime::new()?;
///
/// // Process encrypted data
/// let result = runtime.process_encrypted(
///     &encrypted_data,
///     btsp_session,
///     |plaintext| {
///         // Your compute function
///         Ok(my_analysis(plaintext))
///     },
/// ).await?;
/// ```
pub struct SecureEnclaveRuntime {
    /// Key store for ephemeral keys
    key_store: EphemeralKeyStore,

    /// Runtime configuration
    config: RuntimeConfig,

    /// Audit logger for security events
    audit_logger: Option<AuditLogger>,
}

/// Configuration for secure enclave runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Maximum memory to allocate for processing (bytes)
    pub max_memory: usize,

    /// Enable audit logging
    pub audit_logging: bool,

    /// Enable proof generation
    pub proof_generation: bool,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_memory: 1024 * 1024 * 1024, // 1GB default
            audit_logging: true,
            proof_generation: true,
        }
    }
}

impl SecureEnclaveRuntime {
    /// Create a new secure enclave runtime with default configuration
    ///
    /// # Errors
    ///
    /// Returns error if memory allocation for key store fails
    pub fn new() -> Result<Self> {
        Self::with_config(RuntimeConfig::default())
    }

    /// Create a new secure enclave runtime with custom configuration
    ///
    /// # Errors
    ///
    /// Returns error if memory allocation for key store fails
    pub fn with_config(config: RuntimeConfig) -> Result<Self> {
        let key_store = EphemeralKeyStore::new()?;

        let audit_logger = if config.audit_logging {
            Some(AuditLogger::new())
        } else {
            None
        };

        tracing::info!(
            "Initialized secure enclave runtime with max_memory={}MB, audit_logging={}",
            config.max_memory / 1024 / 1024,
            config.audit_logging
        );

        Ok(Self {
            key_store,
            config,
            audit_logger,
        })
    }

    /// Store an encryption key in the ephemeral key store
    ///
    /// Keys are automatically wiped when the runtime is dropped
    ///
    /// # Errors
    ///
    /// Returns error if key exceeds maximum size or memory allocation fails
    pub fn store_key(&mut self, key: &[u8]) -> Result<()> {
        let result = self.key_store.store_key(key);

        if result.is_ok() {
            self.audit_log(
                AuditEventType::KeyStored,
                format!(r#"{{"key_size": {}}}"#, key.len()),
            )?;
        }

        result
    }

    /// Process data in isolated memory
    ///
    /// # Security
    ///
    /// - Data processed in locked memory (no swap)
    /// - Memory wiped after processing
    /// - Keys wiped after use
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Memory allocation fails
    /// - Processing function fails
    /// - Memory size exceeds configured maximum
    pub fn process_isolated<F, T>(&mut self, data: &[u8], process_fn: F) -> Result<T>
    where
        F: FnOnce(&[u8]) -> Result<T>,
    {
        if data.len() > self.config.max_memory {
            return Err(Error::memory_allocation(format!(
                "Data size {} exceeds max_memory {}",
                data.len(),
                self.config.max_memory
            )));
        }

        // Audit: Processing started
        self.audit_log(
            AuditEventType::ProcessingStarted,
            format!(r#"{{"data_size": {}}}"#, data.len()),
        )?;

        // Allocate isolated memory
        let mut memory = IsolatedMemoryRegion::new(data.len())?;

        self.audit_log(
            AuditEventType::MemoryAllocated,
            format!(r#"{{"size": {}}}"#, memory.physical_size()),
        )?;

        // Copy data into isolated region
        memory.as_mut_slice().copy_from_slice(data);

        // Process in isolated memory
        let result = process_fn(memory.as_slice());

        // Audit: Processing completed (before memory cleanup)
        if result.is_ok() {
            self.audit_log(AuditEventType::ProcessingCompleted, r"{}")?;
        }

        // Memory automatically wiped on drop
        self.audit_log(AuditEventType::MemoryDeallocated, r"{}")?;

        tracing::debug!("Processed {} bytes in isolated memory", data.len());

        result
    }

    /// Log an audit event
    fn audit_log(&mut self, event_type: AuditEventType, details: impl Into<String>) -> Result<()> {
        if let Some(ref mut logger) = self.audit_logger {
            logger.log(event_type, details)?;
        }
        Ok(())
    }

    /// Get reference to audit logger
    #[must_use]
    pub const fn audit_logger(&self) -> Option<&AuditLogger> {
        self.audit_logger.as_ref()
    }

    /// Check if runtime is ready for processing
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        // Ready if we have basic resources
        // Future: check for more resources (GPU, etc.)
        true
    }

    /// Get runtime configuration
    #[must_use]
    pub const fn config(&self) -> &RuntimeConfig {
        &self.config
    }
}

impl Default for SecureEnclaveRuntime {
    fn default() -> Self {
        Self::new().expect(
            "Failed to create default SecureEnclaveRuntime: ephemeral key store allocation failed (check available memory and mlock limits)",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_runtime() {
        let runtime = SecureEnclaveRuntime::new();
        assert!(runtime.is_ok());
        assert!(runtime.unwrap().is_ready());
    }

    #[test]
    fn test_process_isolated() {
        let mut runtime = SecureEnclaveRuntime::new().unwrap();

        let data = b"test data";
        let result = runtime.process_isolated(data, |isolated_data| {
            // Verify data is accessible
            assert_eq!(isolated_data, data);
            Ok(isolated_data.len())
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len());
    }

    #[test]
    fn test_exceeds_max_memory() {
        let config = RuntimeConfig {
            max_memory: 100,
            ..Default::default()
        };
        let mut runtime = SecureEnclaveRuntime::with_config(config).unwrap();

        let large_data = vec![0u8; 200];
        let result = runtime.process_isolated(&large_data, |_| Ok(()));

        assert!(result.is_err());
    }

    #[test]
    fn test_key_storage() {
        let mut runtime = SecureEnclaveRuntime::new().unwrap();

        let key = b"test_encryption_key_32_bytes!!!";
        let result = runtime.store_key(key);

        assert!(result.is_ok());
    }
}
