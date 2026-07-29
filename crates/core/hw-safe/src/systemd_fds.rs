// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    unsafe_code,
    reason = "OwnedFd::from_raw_fd and remove_var require unsafe — containment zone"
)]

//! Safe wrappers for systemd fd-store adoption.
//!
//! Encapsulates the two unsafe operations needed for systemd `LISTEN_FDS`:
//! - Adopting raw fd numbers into `OwnedFd`
//! - Clearing env vars that carry fd metadata (unsafe since Rust 1.83)

use std::os::fd::{FromRawFd, OwnedFd};

/// Adopt a raw fd number (from systemd `LISTEN_FDS`) into an `OwnedFd`.
///
/// # Safety guarantee
///
/// Caller must verify `LISTEN_PID` matches the current process and that
/// `fd_num` is within the range `[3, 3 + LISTEN_FDS)`.
pub fn adopt_raw_fd(fd_num: i32) -> OwnedFd {
    // SAFETY: systemd guarantees these fd numbers are valid and owned by us
    // when LISTEN_PID matches our PID. Caller is responsible for the PID check.
    unsafe { OwnedFd::from_raw_fd(fd_num) }
}

/// Clear systemd fd-passing env vars to prevent child inheritance.
///
/// Must be called single-threaded at startup, before spawning worker threads.
pub fn clear_systemd_fd_env() {
    // SAFETY: Rust 1.83+ marks remove_var as unsafe because concurrent access
    // is UB on some platforms. We document the single-threaded startup requirement.
    unsafe {
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_PID");
        std::env::remove_var("LISTEN_FDNAMES");
    }
}
