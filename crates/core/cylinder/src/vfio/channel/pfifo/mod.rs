// SPDX-License-Identifier: AGPL-3.0-or-later
//! PFIFO engine initialization and diagnostic readback for Volta+ GPUs.
//!
//! Implements the engine bring-up sequence from nouveau's `gk104_fifo_init()`,
//! `gk104_fifo_init_pbdmas()`, `gf100_runq_init()`, and `gk208_runq_init()`.

mod channel;
mod diag;
mod discover;
mod init;
mod kepler;
mod runlist;
mod volta;

pub use discover::{discover_ce_runlist, find_pbdma_for_runlist};
pub use init::PfifoInitConfig;
pub use kepler::init_pfifo_engine_kepler;
pub use volta::init_pfifo_engine_with;
