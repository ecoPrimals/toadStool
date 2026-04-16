// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // NonNull::slice_from_raw_parts in trait defaults — single audit point

//! Shared trait for types that own a contiguous byte region.
//!
//! [`ContiguousBytes`] centralises slice construction into **exactly two**
//! `unsafe` blocks (one for `&[u8]`, one for `&mut [u8]`), using
//! `NonNull::slice_from_raw_parts` plus `as_ref` / `as_mut` (fat-pointer form).
//! Each implementing type proves the pointer/length invariant once via
//! `unsafe impl`; all slice access is then safe.

use std::mem::align_of;
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
        let ptr = self.raw_ptr();
        // `NonNull::slice_from_raw_parts` + `as_ref` need a dereferenceable region for `len`
        // bytes (or an empty slice with a suitably non-null/dangling pointer per layout rules).
        // Non-null: guaranteed by `NonNull<u8>` (`raw_ptr()`).
        debug_assert!(
            ptr.as_ptr().align_offset(align_of::<u8>()) == 0,
            "ContiguousBytes: raw_ptr must meet alignment for u8"
        );
        // Slice extent: `assert!` above; duplicated here for debug-only redundancy.
        debug_assert!(
            isize::try_from(len).is_ok(),
            "ContiguousBytes: raw_len must fit in isize (slice metadata)"
        );
        // SAFETY (`NonNull::as_ref`):
        // - Implementor’s `unsafe trait ContiguousBytes` contract: `raw_ptr()` points to memory
        //   valid for reads for `raw_len()` bytes for the lifetime of `&self`.
        // - `len` equals `raw_len()` and is checked above to be `<= isize::MAX` (slice invariant).
        // - `raw_ptr()` is `NonNull<u8>` (non-null); debug assertion catches misaligned pointers.
        // - `&self` ensures no `&mut` to this region exists, so shared slice reads do not alias
        //   with exclusive mutation through the same implementing type for this lifetime.
        unsafe {
            let nn = NonNull::slice_from_raw_parts(ptr, len);
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
        let ptr = self.raw_ptr();
        debug_assert!(
            ptr.as_ptr().align_offset(align_of::<u8>()) == 0,
            "ContiguousBytes: raw_ptr must meet alignment for u8"
        );
        debug_assert!(
            isize::try_from(len).is_ok(),
            "ContiguousBytes: raw_len must fit in isize (slice metadata)"
        );
        // SAFETY (`NonNull::as_mut`):
        // - Same memory validity and `len` bounds as `as_bytes` (see that block).
        // - `&mut self` is the exclusive proof: no other `&`/`&mut` to this region may exist for
        //   the returned lifetime, matching the trait requirement for exclusive access to the
        //   whole region.
        unsafe {
            let mut nn = NonNull::slice_from_raw_parts(ptr, len);
            nn.as_mut()
        }
    }
}
