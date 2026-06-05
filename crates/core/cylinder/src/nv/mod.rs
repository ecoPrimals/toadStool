// SPDX-License-Identifier: AGPL-3.0-or-later
//! NVIDIA GPU hardware modules — Phase C absorption from coral-driver.
//!
//! This module contains the hardware lifecycle portions of the NVIDIA driver:
//! generation profiles, GPU identity probing, DRM ioctls, QMD encoding,
//! and pushbuf command stream construction.
//!
//! Modules that depend on GSP firmware (`probe`, full `vfio_compute`)
//! remain in coralReef until Phase D or trait boundary.

pub mod bar0;
pub mod registers;
pub mod compute_device;
pub mod driver_probe;
pub mod falcon_pio;
pub mod generation;
pub mod gr_init;
pub mod pmu_init;
pub mod pri;
pub mod rm_abi;
pub mod gsp_bridge;
pub mod hardware_guard;
pub mod identity;
pub mod ioctl;
pub mod iova;
pub mod nv_gsp_bridge;
#[expect(missing_docs, reason = "NV pushbuf — docs tracked as D-DOC")]
pub mod pushbuf;
pub mod qmd;

/// Start of the kernel-managed VA region passed to `VM_INIT`.
///
/// `VM_INIT` reserves `[kernel_managed_addr, kernel_managed_addr + size)` for
/// kernel use (page tables, internal objects). Userspace must allocate VA
/// addresses OUTSIDE this range.
pub const NV_KERNEL_MANAGED_ADDR: u64 = 0x80_0000_0000;

/// Userspace VA heap start — below the kernel-managed region.
///
/// Userspace maps GEM buffers here and grows upward. Must stay below
/// `NV_KERNEL_MANAGED_ADDR`. 4 GiB base avoids low-address collisions.
pub const NV_USER_VA_START: u64 = 0x1_0000_0000;
