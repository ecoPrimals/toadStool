// SPDX-License-Identifier: AGPL-3.0-or-later

//! # Platform Substrate Abstraction (G68)
//!
//! Cross-platform primitives for filesystem operations that historically used
//! raw `std::os::unix` APIs behind `#[cfg(unix)]` blocks.
//!
//! ## Layers
//!
//! - **L1 Links** — [`platform_link`]: symlink on unix, symlink/junction on Windows
//! - **L2 Access** — [`PlatformAccess`] + [`set_access`] / [`check_access`]:
//!   semantic permission intent (mode bits on unix, best-effort on Windows)
//!
//! ## Design
//!
//! The G68 test: "Does this primal do *less* on Windows, or the *same thing differently*?"
//! These abstractions ensure the primal does the **same thing differently** on each platform.

pub mod access;
pub mod links;

pub use access::{PlatformAccess, check_access, set_access};
pub use links::platform_link;
