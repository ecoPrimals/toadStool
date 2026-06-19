// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    dead_code,
    reason = "hardware register constants — comprehensive coverage for evolving absorption"
)]

//! Copy Engine (CE) instance register bases.

/// CE0 register block base in BAR0.
pub const CE0_BASE: u32 = 0x0010_4000;
/// Stride between CE instances.
pub const CE_STRIDE: u32 = 0x1000;

/// CE instance register base for a given CE index.
#[must_use]
pub const fn ce_base(ce: u32) -> u32 {
    CE0_BASE + ce * CE_STRIDE
}
