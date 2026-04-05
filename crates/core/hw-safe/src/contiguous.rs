// SPDX-License-Identifier: AGPL-3.0-only
#![allow(unsafe_code)] // from_raw_parts in trait defaults — this is the single audit point

//! Shared trait for types that own a contiguous byte region.
//!
//! [`ContiguousBytes`] centralises all `std::slice::from_raw_parts` calls into
//! **exactly two** `unsafe` blocks (one for `&[u8]`, one for `&mut [u8]`).
//! Each implementing type proves the pointer/length invariant once via
//! `unsafe impl`; all slice access is then safe.

use std::ptr::NonNull;

/// Types that own a contiguous, valid byte region.
///
/// Default methods provide safe `as_bytes()`/`as_bytes_mut()` by calling
/// `from_raw_parts` exactly once each, centralising the safety proof.
///
/// # Safety
///
/// Implementors must guarantee:
/// - [`raw_ptr()`](Self::raw_ptr) returns a pointer valid for
///   [`raw_len()`](Self::raw_len) bytes.
/// - The region remains valid for the lifetime of `&self`.
/// - `&mut self` guarantees exclusive access to the entire region.
pub unsafe trait ContiguousBytes {
    /// Pointer to the start of the owned region.
    fn raw_ptr(&self) -> NonNull<u8>;

    /// Length of the owned region in bytes.
    fn raw_len(&self) -> usize;

    /// Safe immutable byte slice over the owned region.
    fn as_bytes(&self) -> &[u8] {
        // SAFETY: trait invariant guarantees ptr valid for len bytes;
        // &self ensures no concurrent mutation.
        unsafe { std::slice::from_raw_parts(self.raw_ptr().as_ptr(), self.raw_len()) }
    }

    /// Safe mutable byte slice over the owned region.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        // SAFETY: trait invariant guarantees ptr valid for len bytes;
        // &mut self ensures exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.raw_ptr().as_ptr(), self.raw_len()) }
    }
}
