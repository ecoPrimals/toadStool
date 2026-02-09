//! Low-level I/O operations for Akida devices
//!
//! Handles direct read/write operations to device files with proper
//! error handling and tracing.
//!
//! Deep Debt: Minimal unsafe, well-documented, using nix for safe wrappers.

use crate::error::{AkidaError, Result};
use std::os::unix::io::RawFd;

/// I/O operations handler
///
/// Wraps a file descriptor for read/write operations.
/// Does not own the file descriptor - the caller retains ownership.
#[derive(Debug)]
pub struct IoHandle {
    fd: RawFd,
}

impl IoHandle {
    /// Create new I/O handler for a file descriptor
    #[must_use]
    pub const fn new(fd: RawFd) -> Self {
        Self { fd }
    }

    /// Read data from device
    ///
    /// # Errors
    ///
    /// Returns error if read operation fails.
    pub fn read(&self, buffer: &mut [u8]) -> Result<usize> {
        nix::unistd::read(self.fd, buffer)
            .map_err(|e| AkidaError::transfer_failed(format!("Read failed: {e}")))
    }

    /// Write data to device
    ///
    /// # Errors
    ///
    /// Returns error if write operation fails.
    pub fn write(&self, data: &[u8]) -> Result<usize> {
        nix::unistd::write(self.fd, data)
            .map_err(|e| AkidaError::transfer_failed(format!("Write failed: {e}")))
    }
}
