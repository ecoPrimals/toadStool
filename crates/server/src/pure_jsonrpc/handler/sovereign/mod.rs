// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign GPU initialization JSON-RPC handler.
//!
//! Exposes `sovereign.init` — the staged diesel-engine pipeline that brings a
//! VFIO-bound GPU from cold/warm state to compute-ready.

mod capture;
mod init;
mod snapshot;

pub use capture::*;
pub use init::*;
pub use snapshot::*;

const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;
