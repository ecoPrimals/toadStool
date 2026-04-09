// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // NonNull::slice_from_raw_parts in trait defaults — single audit point

//! Shared trait for types that own a contiguous byte region.
//!
//! [`ContiguousBytes`] centralises slice construction into **exactly two**
//! `unsafe` blocks (one for `&[u8]`, one for `&mut [u8]`), using
//! `NonNull::slice_from_raw_parts` plus `as_ref` / `as_mut` (fat-pointer form).
//! Each implementing type proves the pointer/length invariant once via
//! `unsafe impl`; all slice access is then safe.

use std::ptr::NonNull;

/// Types that own a contiguous, valid byte region.
///
/// Default methods provide safe `as_bytes()`/`as_bytes_mut()` by building a
/// `NonNull<[u8]>` once each, centralising the safety proof.
///
/// # Safety
///
/// Implementors must guarantee:
/// - [`raw_ptr()`](Self::raw_ptr) returns a pointer valid for
///   [`raw_len()`](Self::raw_len) bytes.
/// - [`raw_len()`](Self::raw_len) is at most `isize::MAX` (required for slice types).
/// - The region remains valid for the lifetime of `&self`.
/// - `&mut self` guarantees exclusive access to the entire region.
pub unsafe trait ContiguousBytes {
    /// Pointer to the start of the owned region.
    fn raw_ptr(&self) -> NonNull<u8>;

    /// Length of the owned region in bytes.
    fn raw_len(&self) -> usize;

    /// Safe immutable byte slice over the owned region.
    fn as_bytes(&self) -> &[u8] {
        let len = self.raw_len();
        assert!(
            isize::try_from(len).is_ok(),
            "ContiguousBytes: raw_len {} exceeds isize::MAX (slice precondition)",
            len
        );
        // SAFETY: trait invariant guarantees `ptr` valid for `len` bytes; `len` is checked
        // above; `&self` ensures no concurrent mutation.
        unsafe {
            let nn = NonNull::slice_from_raw_parts(self.raw_ptr(), len);
            nn.as_ref()
        }
    }

    /// Safe mutable byte slice over the owned region.
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        let len = self.raw_len();
        assert!(
            isize::try_from(len).is_ok(),
            "ContiguousBytes: raw_len {} exceeds isize::MAX (slice precondition)",
            len
        );
        // SAFETY: trait invariant guarantees `ptr` valid for `len` bytes; `len` is checked
        // above; `&mut self` ensures exclusive access.
        unsafe {
            let mut nn = NonNull::slice_from_raw_parts(self.raw_ptr(), len);
            nn.as_mut()
        }
    }
}
