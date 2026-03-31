// SPDX-License-Identifier: AGPL-3.0-only
//! CPU pointer validation and safe slice views for [`super::UnifiedBuffer`].

use super::UnifiedBuffer;
use toadstool::error::{ToadStoolError, ToadStoolResult};

impl UnifiedBuffer {
    /// Validate CPU pointer before use.
    ///
    /// `NonNull` guarantees non-null at compile time; this checks the allocation
    /// is still live and the pointer is outside the NULL page.
    pub(in crate::unified_memory::buffer) fn validate_cpu_ptr(&self) -> ToadStoolResult<()> {
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime(
                "Buffer has been freed (allocation is None)",
            ));
        }

        // NonNull guarantees non-null, but we still guard against addresses
        // in the kernel NULL page (typically < 4096 on Linux).
        let ptr_val = self.cpu_ptr.as_ptr() as usize;
        if ptr_val < 4096 {
            return Err(ToadStoolError::runtime(format!(
                "CPU pointer value {ptr_val} is in NULL page (invalid)"
            )));
        }

        // Check pointer alignment (must be properly aligned)
        if !ptr_val.is_multiple_of(std::mem::align_of::<u8>()) {
            return Err(ToadStoolError::runtime(format!(
                "CPU pointer {ptr_val:#x} is not properly aligned"
            )));
        }

        // Check size is reasonable
        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        Ok(())
    }

    /// Get safe mutable slice from CPU pointer (internal helper)
    ///
    /// # Safety
    /// This is the ONLY place we convert raw pointer to slice.
    /// All unsafe pointer operations go through this method.
    ///
    /// # Guarantees
    /// - Pointer is validated (not null, properly aligned, allocation exists)
    /// - Size is valid (checked at creation and validate_cpu_ptr)
    /// - Bounds: slice covers [0..size], caller must bounds-check offset+len
    /// - Exclusive access via &mut self
    pub(in crate::unified_memory::buffer) fn as_cpu_slice_mut(
        &mut self,
    ) -> ToadStoolResult<&mut [u8]> {
        self.validate_cpu_ptr()?;

        debug_assert!(
            self.size > 0,
            "Buffer size must be > 0 (validated by validate_cpu_ptr)"
        );
        // SAFETY: Invariants: ptr valid for size; aligned; exclusive access; allocation exists.
        // Satisfied: validate_cpu_ptr checked; NonNull; &mut self; allocation.is_some().
        // Violation: invalid ptr/size → UB; aliasing → data race; use-after-free if freed.
        Ok(unsafe { std::slice::from_raw_parts_mut(self.cpu_ptr.as_ptr(), self.size) })
    }

    /// Get safe immutable slice from CPU pointer (internal helper)
    ///
    /// # Safety
    /// This is the ONLY place we convert raw pointer to slice for reads.
    /// All unsafe pointer operations go through this method.
    ///
    /// # Guarantees
    /// - Pointer is validated (not null, properly aligned, allocation exists)
    /// - Size is valid (checked at creation and validate_cpu_ptr)
    /// - Bounds: slice covers [0..size], caller must bounds-check offset+len
    /// - Shared access via &self (Rust ensures no concurrent writes)
    pub(in crate::unified_memory::buffer) fn as_cpu_slice(&self) -> ToadStoolResult<&[u8]> {
        self.validate_cpu_ptr()?;

        debug_assert!(
            self.size > 0,
            "Buffer size must be > 0 (validated by validate_cpu_ptr)"
        );
        // SAFETY: Invariants: ptr valid for size; aligned; allocation exists; no concurrent mutation.
        // Satisfied: validate_cpu_ptr checked; &self (Rust ensures no &mut); allocation.is_some().
        // Violation: invalid ptr/size → UB; concurrent mutation → data race.
        Ok(unsafe { std::slice::from_raw_parts(self.cpu_ptr.as_ptr(), self.size) })
    }
}
