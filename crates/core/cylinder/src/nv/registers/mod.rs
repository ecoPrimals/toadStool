// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU-wide BAR0 register offsets for NVIDIA GPUs.
//!
//! This module consolidates hardware register constants that are scattered
//! across driver bring-up, sovereign handoff, and diagnostic binaries.
//! It covers BAR0 domains that are **not** channel-specific — PFIFO, PBDMA,
//! RAMIN, and USERD offsets remain in [`crate::vfio::channel::registers`].
//!
//! Falcon engine bases and per-falcon offsets live in [`falcon`]; PMU-specific
//! extensions (DMATRF layout) are in [`pmu`].

pub mod ce;
pub mod falcon;
pub mod gpc;
pub mod pbus;
pub mod pfb;
pub mod pgraph;
pub mod pmc;
pub mod pmu;
pub mod pramin;
pub mod pri;
pub mod ptimer;
pub mod usermode;
