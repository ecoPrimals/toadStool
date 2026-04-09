// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU pointer validation and safe slice views for [`super::UnifiedBuffer`].

use std::ptr::NonNull;

use super::UnifiedBuffer;
use toadstool::error::{ToadStoolError, ToadStoolResult};

impl UnifiedBuffer {
    /// Validate CPU pointer before use.
    ///
    /// Checks that are **proven** at runtime:
    /// - `allocation.is_some()` — the RAII handle has not been consumed by `Drop`
    /// - NULL-page guard — address is above 4096 (kernel reserved on Linux)
    /// - Non-zero size
    /// - Slice extent — `size <= isize::MAX` and `ptr + size` does not overflow `usize`
    ///
    /// What is **assumed** (upheld by backend contract, not runtime-checked):
    /// - The backend keeps the allocation valid until `free_unified` is called
    /// - The pointer remains mapped for the lifetime of the `BackendAllocation`
    /// - `size` matches the actual allocation extent
    pub(in crate::unified_memory::buffer) fn validate_cpu_ptr(&self) -> ToadStoolResult<()> {
        if self.allocation.is_none() {
            return Err(ToadStoolError::runtime(
                "Buffer has been freed (allocation is None)",
            ));
        }

        let ptr_val = self.cpu_ptr.as_ptr() as usize;
        if ptr_val < 4096 {
            return Err(ToadStoolError::runtime(format!(
                "CPU pointer value {ptr_val} is in NULL page (invalid)"
            )));
        }

        if self.size == 0 {
            return Err(ToadStoolError::runtime("Buffer size is zero"));
        }

        // `slice::from_raw_parts` / `NonNull::slice_from_raw_parts` require `len <= isize::MAX`.
        if self.size > isize::MAX as usize {
            return Err(ToadStoolError::runtime(
                "Buffer size exceeds maximum slice extent (isize::MAX)",
            ));
        }

        ptr_val
            .checked_add(self.size)
            .ok_or_else(|| ToadStoolError::runtime("CPU pointer + size overflows address range"))?;

        Ok(())
    }

    /// Get mutable slice from CPU pointer (internal helper).
    ///
    /// This is the **sole** `NonNull::slice_from_raw_parts` / `as_mut` slice construction
    /// site in the buffer API.
    ///
    /// # Safety contract
    ///
    /// The `unsafe` block relies on invariants that are **partially runtime-checked**
    /// and **partially upheld by the backend allocation contract**:
    ///
    /// | Invariant | Enforcement |
    /// |-----------|-------------|
    /// | Non-null | `NonNull<u8>` (compile-time) |
    /// | Allocation alive | `allocation.is_some()` (runtime) |
    /// | Not in NULL page | `ptr_val >= 4096` (runtime) |
    /// | Slice extent | `size <= isize::MAX`, `ptr + size` does not overflow `usize` (runtime) |
    /// | Valid for `size` bytes | Backend contract: `allocate_unified(size)` maps `size` bytes |
    /// | Exclusively borrowed | `&mut self` (borrow checker) |
    /// | No concurrent GPU access | Caller must sync before CPU write (`sync_to_cpu`) |
    pub(in crate::unified_memory::buffer) fn as_cpu_slice_mut(
        &mut self,
    ) -> ToadStoolResult<&mut [u8]> {
        self.validate_cpu_ptr()?;

        // SAFETY: see safety contract table above. `NonNull::slice_from_raw_parts` is the
        // fat-pointer form of `from_raw_parts_mut`; invariants match those for a mutable slice.
        Ok(unsafe {
            let mut slice_nn = NonNull::slice_from_raw_parts(self.cpu_ptr, self.size);
            slice_nn.as_mut()
        })
    }

    /// Get immutable slice from CPU pointer (internal helper).
    ///
    /// This is the **sole** `NonNull::slice_from_raw_parts` / `as_ref` slice construction
    /// site in the buffer API.
    ///
    /// # Safety contract
    ///
    /// Same as [`Self::as_cpu_slice_mut`] except:
    /// - Shared access via `&self` — Rust borrow checker ensures no `&mut` alias
    /// - Concurrent *reads* are safe; concurrent *GPU writes* are not
    ///   (caller must `sync_to_cpu` first)
    pub(in crate::unified_memory::buffer) fn as_cpu_slice(&self) -> ToadStoolResult<&[u8]> {
        self.validate_cpu_ptr()?;

        // SAFETY: see `as_cpu_slice_mut` contract; `&self` ensures no concurrent `&mut`.
        Ok(unsafe {
            let slice_nn = NonNull::slice_from_raw_parts(self.cpu_ptr, self.size);
            slice_nn.as_ref()
        })
    }
}
