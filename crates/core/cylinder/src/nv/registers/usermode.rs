// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(dead_code, reason = "hardware register constants — comprehensive coverage for evolving absorption")]

//! NV_USERMODE — userspace doorbell and notification registers.

/// Channel pending notification status.
pub const NOTIFY_CHANNEL_PENDING: u32 = 0x0081_0090;

/// GK104 doorbell register offset for a given channel ID.
#[must_use]
pub const fn gk104_doorbell(channel_id: u32) -> u32 {
    0x3000 + channel_id * 8
}
