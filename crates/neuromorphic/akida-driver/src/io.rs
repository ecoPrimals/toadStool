//! Low-level I/O operations for Akida devices
//!
//! Handles direct read/write operations to device files with proper
//! error handling and tracing.
//!
//! # Evolution (Feb 27, 2026)
//!
//! Evolved from `unsafe BorrowedFd::borrow_raw` to safe `AsFd` trait.
//! Zero unsafe blocks — the borrow checker ensures fd validity.
//!
//! # Evolution (Feb 12, 2026)
//!
//! Evolved from `nix` to `rustix` for pure Rust syscall wrappers.

use crate::error::{AkidaError, Result};
use rustix::io::{read, write};
use std::os::unix::io::AsFd;

/// Read data from a file-descriptor-bearing object
///
/// # Errors
///
/// Returns error if read operation fails.
pub fn device_read(fd: &impl AsFd, buffer: &mut [u8]) -> Result<usize> {
    read(fd, buffer).map_err(|e| AkidaError::transfer_failed(format!("Read failed: {e}")))
}

/// Write data to a file-descriptor-bearing object
///
/// # Errors
///
/// Returns error if write operation fails.
pub fn device_write(fd: &impl AsFd, data: &[u8]) -> Result<usize> {
    write(fd, data).map_err(|e| AkidaError::transfer_failed(format!("Write failed: {e}")))
}
