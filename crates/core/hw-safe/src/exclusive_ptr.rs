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

// SAFETY (`Send`):
// - `Send` requires that moving the value to another thread cannot violate memory safety.
// - `ExclusivePtr` is `repr(transparent)` over `NonNull<u8>` and is only constructed via
//   `ExclusivePtr::new` in this crate; callers must pass a pointer to memory that is safe to
//   own and access from any thread the owning type may move to (e.g. heap via the global
//   allocator, `mmap`, or other process-wide storage).
// - The pointer must not reference thread-local storage or stack memory tied to another thread.
// - Aliasing: the “exclusive” contract means the owning abstraction must not expose aliasing
//   `&mut` or raw pointers that break `Send` when the wrapper crosses threads; `ExclusivePtr`
//   itself does not synchronize—parent types must enforce that.
unsafe impl Send for ExclusivePtr {}

// SAFETY (`Sync`):
// - `Sync` requires that sharing `&ExclusivePtr` across threads is sound if the only operations
//   are those allowed on shared references (here: copying the pointer value, `ExclusivePtr::as_ptr`,
//   `ExclusivePtr::as_non_null`, formatting).
// - Those operations only read the address; they do not dereference through `&ExclusivePtr`
//   in a way that races with mutation, as long as the owning type uses interior mutability or
//   `&mut` consistently with Rust's aliasing rules for the underlying allocation.
// - Same storage and “no TLS / no stack from another frame” requirements as `Send`; concurrent
//   mutation of the pointed-to bytes must still be serialized by the owner (e.g. mutex or
//   single-writer discipline), since `ExclusivePtr` does not add locking.
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
