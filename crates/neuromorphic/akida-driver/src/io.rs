//! Low-level I/O operations for Akida devices
//!
//! Handles direct read/write operations to device files with proper
//! error handling and tracing.

use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use crate::error::{AkidaError, Result};

/// I/O operations handler
pub struct IoHandle {
    fd: RawFd,
}

impl IoHandle {
    /// Create new I/O handler for a file descriptor
    #[must_use]
    pub const fn new(fd: RawFd) -> Self {
        Self { fd }
    }

    /// Write data to device
    ///
    /// This performs a DMA transfer via the kernel driver.
    /// The actual transfer is handled by the akida_pcie driver.
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        tracing::trace!("Writing {} bytes to device", data.len());
        
        // SAFETY: We own the file descriptor and it's valid
        let mut file = unsafe { std::fs::File::from_raw_fd(self.fd) };
        let result = file.write(data);
        
        // Don't close the file descriptor when File is dropped
        let _ = file.into_raw_fd();
        
        let written = result.map_err(|e| {
            tracing::error!("Write failed: {e}");
            AkidaError::transfer_failed(format!("Write error: {e}"))
        })?;
        
        tracing::debug!("Wrote {} bytes successfully", written);
        
        Ok(written)
    }

    /// Read data from device
    ///
    /// This performs a DMA transfer via the kernel driver.
    /// The actual transfer is handled by the akida_pcie driver.
    pub fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        tracing::trace!("Reading up to {} bytes from device", buffer.len());
        
        // SAFETY: We own the file descriptor and it's valid
        let mut file = unsafe { std::fs::File::from_raw_fd(self.fd) };
        let result = file.read(buffer);
        
        // Don't close the file descriptor when File is dropped
        let _ = file.into_raw_fd();
        
        let read_bytes = result.map_err(|e| {
            tracing::error!("Read failed: {e}");
            AkidaError::transfer_failed(format!("Read error: {e}"))
        })?;
        
        tracing::debug!("Read {} bytes successfully", read_bytes);
        
        Ok(read_bytes)
    }
}

// Note: We deliberately don't implement Drop to avoid closing the FD
// The FD is owned by DeviceHandle which will close it properly
