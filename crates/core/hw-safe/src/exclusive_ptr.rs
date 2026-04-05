// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // manual Send/Sync — see SAFETY on impls below

//! Thread-safe exclusively-owned memory pointer.
//!
//! [`ExclusivePtr`] wraps `NonNull<u8>` with explicit `Send + Sync` so owning
//! types auto-derive thread safety. `std::ptr::NonNull<u8>` does **not**
//! implement `Send`/`Sync` (it is a raw owning pointer with no aliasing
//! guarantees to the type system), so we assert the intended invariant here.

use std::ptr::NonNull;

/// Exclusively-owned raw memory pointer with `Send + Sync`.
///
/// Types that store `ExclusivePtr` instead of raw `NonNull<u8>` auto-derive
/// `Send + Sync` (assuming all other fields are also `Send + Sync`),
/// eliminating the need for per-type `unsafe impl Send/Sync` on those types.
///
/// # Invariant
///
/// The owning type guarantees exclusive ownership of the pointed-to memory:
/// - No aliasing pointers exist
/// - The borrow checker enforces `&`/`&mut` exclusivity on the owning type
/// - The memory is not thread-local
#[repr(transparent)]
pub(crate) struct ExclusivePtr(NonNull<u8>);

// SAFETY: The owning type guarantees exclusive ownership of memory that is valid
// for use from any thread (heap via global alloc, or mmap). The pointer does
// not refer to thread-local storage. This matches the intended semantics of
// `Send` for owned byte buffers despite `NonNull<u8>` not being `Send`.
unsafe impl Send for ExclusivePtr {}

// SAFETY: Shared immutable access (`&ExclusivePtr`) is safe across threads when
// the underlying allocation is process-wide and the owning type serializes
// mutation via `&mut` (same reasoning as `Send`).
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
