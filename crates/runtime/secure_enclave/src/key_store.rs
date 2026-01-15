//! Ephemeral key storage with explicit wiping
//!
//! Keys are stored in isolated memory and wiped explicitly on drop.

use crate::error::{Error, Result};
use crate::isolated_memory::IsolatedMemoryRegion;

/// Maximum key size (supports AES-256: 32 bytes + IV: 12 bytes)
const MAX_KEY_SIZE: usize = 64;

/// Ephemeral key store for cryptographic keys
///
/// Keys are:
/// - Stored in isolated memory (locked, no swap)
/// - Wiped explicitly before deallocation
/// - Never logged or serialized
///
/// # Security
///
/// This is a **deep solution** for key management:
/// - Keys never touch disk
/// - Memory locked to prevent swapping
/// - Explicit zeroing before deallocation
/// - Compiler fence prevents optimization
pub struct EphemeralKeyStore {
    /// Isolated memory for key storage
    memory: IsolatedMemoryRegion,

    /// Current key size (0 if no key stored)
    key_size: usize,
}

impl EphemeralKeyStore {
    /// Create a new empty key store
    ///
    /// # Errors
    /// Returns an error if isolated memory region allocation fails.
    pub fn new() -> Result<Self> {
        let memory = IsolatedMemoryRegion::new(MAX_KEY_SIZE)?;

        Ok(Self {
            memory,
            key_size: 0,
        })
    }

    /// Store a key in isolated memory
    ///
    /// # Errors
    ///
    /// Returns error if key is too large for storage
    pub fn store_key(&mut self, key: &[u8]) -> Result<()> {
        if key.len() > MAX_KEY_SIZE {
            return Err(Error::key_store(format!(
                "Key size {} exceeds maximum {}",
                key.len(),
                MAX_KEY_SIZE
            )));
        }

        let buffer = self.memory.as_mut_slice();
        buffer[..key.len()].copy_from_slice(key);
        self.key_size = key.len();

        tracing::trace!("Stored {}-byte key in isolated memory", key.len());
        Ok(())
    }

    /// Get reference to stored key
    ///
    /// # Errors
    ///
    /// Returns error if no key is stored
    pub fn key(&self) -> Result<&[u8]> {
        if self.key_size == 0 {
            return Err(Error::key_store("No key stored"));
        }

        Ok(&self.memory.as_slice()[..self.key_size])
    }

    /// Explicitly wipe the stored key
    ///
    /// This is also called automatically in Drop
    pub fn wipe(&mut self) {
        self.memory.wipe();
        self.key_size = 0;
        tracing::trace!("Wiped key from key store");
    }

    /// Check if a key is currently stored
    #[must_use]
    pub const fn has_key(&self) -> bool {
        self.key_size > 0
    }
}

impl Default for EphemeralKeyStore {
    fn default() -> Self {
        Self::new().expect("Failed to create default key store")
    }
}

impl Drop for EphemeralKeyStore {
    fn drop(&mut self) {
        // Explicit wipe before drop
        self.wipe();
        // IsolatedMemoryRegion::drop will also wipe
        tracing::trace!("Dropped ephemeral key store");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve_key() {
        let mut store = EphemeralKeyStore::new().unwrap();

        let key = b"test_key_32_bytes_long_padding!!";
        store.store_key(key).unwrap();

        assert!(store.has_key());
        assert_eq!(store.key().unwrap(), key);
    }

    #[test]
    fn test_empty_store() {
        let store = EphemeralKeyStore::new().unwrap();

        assert!(!store.has_key());
        assert!(store.key().is_err());
    }

    #[test]
    fn test_key_too_large() {
        let mut store = EphemeralKeyStore::new().unwrap();

        let large_key = vec![0u8; MAX_KEY_SIZE + 1];
        let result = store.store_key(&large_key);

        assert!(result.is_err());
    }

    #[test]
    fn test_explicit_wipe() {
        let mut store = EphemeralKeyStore::new().unwrap();

        let key = b"sensitive_key";
        store.store_key(key).unwrap();
        assert!(store.has_key());

        store.wipe();
        assert!(!store.has_key());
        assert!(store.key().is_err());
    }
}
