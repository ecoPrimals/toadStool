// SPDX-License-Identifier: AGPL-3.0-or-later
//! CPU pointer validation and safe slice views for [`super::UnifiedBuffer`].

use std::mem::align_of;
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
    ///
    /// The `unsafe` on [`NonNull::as_mut`] is **not** about pointer arithmetic or slice length:
    /// [`NonNull::slice_from_raw_parts`] (safe) already pairs the data pointer with `size`.
    /// The remaining obligation is the usual slice aliasing contract: the memory must be
    /// valid for `size` bytes for reads/writes for the returned lifetime, and this `&mut`
    /// must be the only active reference to those bytes (see table).
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "&mut self required for soundness: exclusive borrow prevents aliased \
                  mutable access to the underlying GPU allocation through the returned slice"
    )]
    pub(in crate::unified_memory::buffer) fn as_cpu_slice_mut(
        &mut self,
    ) -> ToadStoolResult<&mut [u8]> {
        self.validate_cpu_ptr()?;

        // `cpu_ptr` is `NonNull<u8>` — non-null is a type invariant (no runtime null check).
        debug_assert!(self.size > 0, "validate_cpu_ptr ensures non-zero size");
        debug_assert!(
            isize::try_from(self.size).is_ok(),
            "validate_cpu_ptr ensures slice extent"
        );
        debug_assert!(
            self.cpu_ptr.as_ptr().align_offset(align_of::<u8>()) == 0,
            "cpu_ptr must be aligned for u8"
        );

        let mut slice_ptr = NonNull::slice_from_raw_parts(self.cpu_ptr, self.size);
        // SAFETY (`NonNull::as_mut`):
        // - `validate_cpu_ptr` + debug asserts: non-null `cpu_ptr`, `size` in `(0, isize::MAX]`,
        //   `ptr + size` does not wrap, and pointer is suitably aligned for `u8`.
        // - Memory validity for `size` bytes for reads/writes through this slice for the returned
        //   lifetime is guaranteed by the backend: allocation stays live until `free_unified`, and
        //   `size` matches the mapped extent (see module-level safety contract table).
        // - `&mut self` ensures no other reference to this buffer’s bytes aliases this `&mut [u8]`.
        // - Cross-device coherence (GPU vs CPU) remains the caller’s responsibility (`sync_to_cpu`
        //   before CPU write, etc.); this block only forms the Rust slice.
        Ok(unsafe { slice_ptr.as_mut() })
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
    ///
    /// As with [`Self::as_cpu_slice_mut`], [`NonNull::as_ref`] is `unsafe` only for the
    /// validity and aliasing contract on the underlying allocation, not for constructing
    /// the slice metadata (that is handled by [`NonNull::slice_from_raw_parts`]).
    pub(in crate::unified_memory::buffer) fn as_cpu_slice(&self) -> ToadStoolResult<&[u8]> {
        self.validate_cpu_ptr()?;

        // `cpu_ptr` is `NonNull<u8>` — non-null is a type invariant (no runtime null check).
        debug_assert!(self.size > 0, "validate_cpu_ptr ensures non-zero size");
        debug_assert!(
            isize::try_from(self.size).is_ok(),
            "validate_cpu_ptr ensures slice extent"
        );
        debug_assert!(
            self.cpu_ptr.as_ptr().align_offset(align_of::<u8>()) == 0,
            "cpu_ptr must be aligned for u8"
        );

        let slice_ptr = NonNull::slice_from_raw_parts(self.cpu_ptr, self.size);
        // SAFETY (`NonNull::as_ref`):
        // - Same preconditions as `as_cpu_slice_mut` (see that block): validated pointer, size,
        //   alignment, and backend lifetime/extent contract.
        // - `&self` only guarantees shared immutability in Rust; concurrent GPU writes are still
        //   unsound for CPU reads unless the caller has synchronized (`sync_to_cpu`). The slice is
        //   otherwise valid for immutable reads of `size` bytes for the returned lifetime.
        Ok(unsafe { slice_ptr.as_ref() })
    }
}
