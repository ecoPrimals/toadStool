// SPDX-License-Identifier: AGPL-3.0-or-later
//! Async CPU read and write paths for [`super::UnifiedBuffer`].

use super::UnifiedBuffer;
use crate::unified_memory::types::SyncState;
use bytes::Bytes;
use toadstool::error::{ToadStoolError, ToadStoolResult};

impl UnifiedBuffer {
    /// Write data from CPU (async, non-blocking)
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in bytes from buffer start
    /// * `data` - Data to write (accepts `&[u8]`, `Vec<u8>`, `Bytes`, or any `AsRef<[u8]>`)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Buffer has been freed
    /// - Write would overflow buffer
    /// - Pointer is invalid
    pub async fn write_async<D: AsRef<[u8]>>(
        &mut self,
        offset: usize,
        data: D,
    ) -> ToadStoolResult<()> {
        let data = data.as_ref();
        // Handle zero-length write
        if data.is_empty() {
            return Ok(());
        }

        // Validate buffer is still valid
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime("Buffer has been freed"));
        }

        // Validate size is not zero (defensive)
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        // Validate bounds with overflow protection
        let end_offset = offset
            .checked_add(data.len())
            .ok_or_else(|| ToadStoolError::runtime("Write offset + length would overflow"))?;

        if end_offset > self.size {
            return Err(ToadStoolError::runtime(format!(
                "Write would overflow buffer: offset={}, len={}, size={}",
                offset,
                data.len(),
                self.size
            )));
        }

        // Validate pointer value (defensive check - still useful for invalid addresses)
        let ptr_value = self.cpu_ptr.as_ptr() as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        let buffer_slice = self.as_cpu_slice_mut()?;
        buffer_slice[offset..offset + data.len()].copy_from_slice(data);

        // Update sync state
        *self
            .sync_state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = SyncState::CpuModified;

        // Update metadata
        if let Some(metadata) = self
            .allocations
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&self.id)
        {
            metadata.record_access();
        }

        tracing::trace!(
            "Wrote {} bytes to buffer {} at offset {}",
            data.len(),
            self.id,
            offset
        );

        Ok(())
    }

    /// Read data to CPU (async, non-blocking)
    ///
    /// Returns [`Bytes`] for zero-copy cloning when passing data across threads/tasks.
    /// Use `.to_vec()` if you need mutable access to the result.
    ///
    /// # Arguments
    ///
    /// * `offset` - Offset in bytes from buffer start
    /// * `len` - Number of bytes to read
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Buffer has been freed
    /// - Read would overflow buffer
    /// - Pointer is invalid
    pub async fn read_async(&self, offset: usize, len: usize) -> ToadStoolResult<Bytes> {
        // Handle zero-length read
        if len == 0 {
            return Ok(Bytes::new());
        }

        // Validate buffer is still valid
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime("Buffer has been freed"));
        }

        // Validate size is not zero (defensive)
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        // Validate bounds with overflow protection
        let end_offset = offset
            .checked_add(len)
            .ok_or_else(|| ToadStoolError::runtime("Read offset + length would overflow"))?;

        if end_offset > self.size {
            return Err(ToadStoolError::runtime(format!(
                "Read would overflow buffer: offset={}, len={}, size={}",
                offset, len, self.size
            )));
        }

        // Validate pointer value (defensive check - still useful for invalid addresses)
        let ptr_value = self.cpu_ptr.as_ptr() as usize;
        if ptr_value == 0 {
            return Err(ToadStoolError::runtime("CPU pointer is zero (invalid)"));
        }

        let buffer_slice = self.as_cpu_slice()?;
        let result = Bytes::copy_from_slice(&buffer_slice[offset..offset + len]);

        // Update metadata
        if let Some(metadata) = self
            .allocations
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(&self.id)
        {
            metadata.record_access();
        }

        tracing::trace!(
            "Read {} bytes from buffer {} at offset {}",
            len,
            self.id,
            offset
        );

        Ok(result)
    }
}
