// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(unsafe_code)] // This module itself is safe; allow needed for crate-level deny

//! GPU implementation of the glowPlug/ember hardware lifecycle traits.
//!
//! This is the **first concrete implementation** of toadStool's hardware-
//! agnostic glowPlug subsystem, modeled after the visualization service's
//! `coral-glowplug` and `coral-ember` (GPU/VFIO-specific). When the
//! visualization service fully cracks the GPU (FECS, compute dispatch),
//! toadStool absorbs their implementation here and the visualization
//! service leans on toadStool for hardware.
//!
//! ## What's GPU-specific
//!
//! - [`GpuPersonality`] — VFIO, nouveau, nvidia, amdgpu, xe, i915, unbound
//! - [`GpuPersonalityRegistry`] — Linux GPU driver registry
//! - [`GpuFirmwareAccess`] — direct FECS/GPCCS/PMU BAR0 register reads
//! - PCI BDF addressing, BAR0 MMIO (via hw-safe), DRM card paths
//!
//! ## What's inherited from glowPlug (hardware-agnostic)
//!
//! - Device slot lifecycle
//! - Swap orchestration (quiesce → persist → swap → restore → health)
//! - ember hold / lend / reclaim
//! - Metadata persistence and journal

pub mod discovery;
pub mod firmware;
pub mod personality;

pub use discovery::GpuDiscovery;
pub use firmware::GpuFirmwareAccess;
pub use personality::{GpuPersonality, GpuPersonalityRegistry};
