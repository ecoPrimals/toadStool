//! Error types for secure enclave runtime
//!
//! Follows ToadStool's 3-tier error system with proper error handling
//! and context preservation.

use thiserror::Error;

/// Result type for secure enclave operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in secure enclave operations
#[derive(Error, Debug)]
pub enum Error {
    /// Memory allocation failed
    #[error("Failed to allocate isolated memory: {reason}")]
    MemoryAllocation {
        /// Reason for allocation failure
        reason: String,
    },

    /// Memory locking failed (mlock)
    #[error("Failed to lock memory pages: {reason}")]
    MemoryLock {
        /// Reason for lock failure
        reason: String,
    },

    /// Memory protection failed (madvise)
    #[error("Failed to set memory protection: {reason}")]
    MemoryProtection {
        /// Reason for protection failure
        reason: String,
    },

    /// Invalid memory layout
    #[error("Invalid memory layout: size={size}, alignment={alignment}")]
    InvalidLayout {
        /// Requested size in bytes
        size: usize,
        /// Required alignment
        alignment: usize,
    },

    /// Decompression failed
    #[error("Decompression failed: {reason}")]
    Decompression {
        /// Reason for decompression failure
        reason: String,
    },

    /// Encryption/decryption failed
    #[error("Cryptographic operation failed: {operation}")]
    Cryptography {
        /// Operation that failed
        operation: String,
    },

    /// Key management error
    #[error("Key store error: {reason}")]
    KeyStore {
        /// Reason for key store error
        reason: String,
    },

    /// BTSP communication error
    #[error("BTSP protocol error: {reason}")]
    Btsp {
        /// Reason for BTSP error
        reason: String,
    },

    /// Audit log error
    #[error("Audit logging failed: {reason}")]
    AuditLog {
        /// Reason for audit log failure
        reason: String,
    },

    /// Security violation detected
    #[error("Security violation: {violation}")]
    SecurityViolation {
        /// Description of violation
        violation: String,
    },

    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Integration with ToadStool common errors
    #[error("ToadStool error: {0}")]
    ToadStool(#[from] toadstool_common::error::ToadStoolError),
}

impl Error {
    /// Create a memory allocation error
    pub fn memory_allocation(reason: impl Into<String>) -> Self {
        Self::MemoryAllocation {
            reason: reason.into(),
        }
    }

    /// Create a memory lock error
    pub fn memory_lock(reason: impl Into<String>) -> Self {
        Self::MemoryLock {
            reason: reason.into(),
        }
    }

    /// Create a memory protection error
    pub fn memory_protection(reason: impl Into<String>) -> Self {
        Self::MemoryProtection {
            reason: reason.into(),
        }
    }

    /// Create an invalid layout error
    #[must_use]
    pub const fn invalid_layout(size: usize, alignment: usize) -> Self {
        Self::InvalidLayout { size, alignment }
    }

    /// Create a decompression error
    pub fn decompression(reason: impl Into<String>) -> Self {
        Self::Decompression {
            reason: reason.into(),
        }
    }

    /// Create a cryptography error
    pub fn cryptography(operation: impl Into<String>) -> Self {
        Self::Cryptography {
            operation: operation.into(),
        }
    }

    /// Create a key store error
    pub fn key_store(reason: impl Into<String>) -> Self {
        Self::KeyStore {
            reason: reason.into(),
        }
    }

    /// Create a BTSP error
    pub fn btsp(reason: impl Into<String>) -> Self {
        Self::Btsp {
            reason: reason.into(),
        }
    }

    /// Create an audit log error
    pub fn audit_log(reason: impl Into<String>) -> Self {
        Self::AuditLog {
            reason: reason.into(),
        }
    }

    /// Create a security violation error
    pub fn security_violation(violation: impl Into<String>) -> Self {
        Self::SecurityViolation {
            violation: violation.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::memory_allocation("out of memory");
        assert!(err.to_string().contains("out of memory"));

        let err = Error::invalid_layout(1024, 64);
        assert!(err.to_string().contains("1024"));
        assert!(err.to_string().contains("64"));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
