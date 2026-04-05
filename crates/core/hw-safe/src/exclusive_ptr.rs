// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // Send/Sync impls for exclusively-owned pointer

//! Thread-safe exclusively-owned memory pointer.
//!
//! [`ExclusivePtr`] wraps `NonNull<u8>` with `Send + Sync` so that types
//! storing it auto-derive thread safety — no per-type `unsafe impl` needed.

use std::ptr::NonNull;

/// Exclusively-owned raw memory pointer with `Send + Sync`.
///
/// Types that store `ExclusivePtr` instead of raw `NonNull<u8>` auto-derive
/// `Send + Sync` (assuming all other fields are also `Send + Sync`),
/// eliminating the need for per-type `unsafe impl Send/Sync`.
///
/// # Invariant
///
/// The owning type guarantees exclusive ownership of the pointed-to memory:
/// - No aliasing pointers exist
/// - The borrow checker enforces `&`/`&mut` exclusivity on the owning type
/// - The memory is not thread-local
#[repr(transparent)]
pub(crate) struct ExclusivePtr(NonNull<u8>);

// SAFETY: Exclusive ownership means no aliasing; moving between threads is safe
// because the memory is process-wide (heap/mmap), not thread-local.
unsafe impl Send for ExclusivePtr {}

// SAFETY: &self gives read-only access; &mut self requires exclusivity.
// The borrow checker on the owning type enforces this.
unsafe impl Sync for ExclusivePtr {}

impl ExclusivePtr {
    /// Wrap a `NonNull<u8>` with exclusive-ownership semantics.
    #[inline]
    pub(crate) fn new(ptr: NonNull<u8>) -> Self {
        Self(ptr)
    }

    /// Raw mutable pointer (for OS/kernel APIs that need `*mut u8`).
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut u8 {
        self.0.as_ptr()
    }

    /// Access as `NonNull<u8>` (for APIs that take `NonNull`).
    #[inline]
    pub(crate) fn as_non_null(&self) -> NonNull<u8> {
        self.0
    }
}

impl std::fmt::Debug for ExclusivePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.0.as_ptr())
    }
}

impl std::fmt::Pointer for ExclusivePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Pointer::fmt(&self.0.as_ptr(), f)
    }
}
