// SPDX-License-Identifier: AGPL-3.0-or-later
//! Vendor-specific device lifecycle hooks for safe driver transitions.
//!
//! Absorbed from coralReef `coral-ember`. Different GPU/NPU vendors (and
//! chip families within a vendor) have wildly different behaviors when
//! VFIO-PCI unbinds, bus resets fire, or native drivers rebind. This
//! module encodes those differences as a trait so the core swap logic
//! stays generic.

mod amd;
mod brainchip;
mod detect;
mod generic;
mod intel;
mod nvidia;
mod types;

#[cfg(test)]
mod tests;

pub use amd::{AmdRdnaLifecycle, AmdVega20Lifecycle};
pub use brainchip::BrainChipLifecycle;
pub use detect::{detect_lifecycle, detect_lifecycle_for_target};
pub use generic::GenericLifecycle;
pub use intel::IntelXeLifecycle;
pub use nvidia::{
    NvidiaKeplerLifecycle, NvidiaLifecycle, NvidiaOpenLifecycle, NvidiaOracleLifecycle,
};
pub use types::{RebindStrategy, ResetMethod, VendorLifecycle};

#[cfg(test)]
pub(crate) use detect::{is_amd_vega20, is_nvidia_kepler, lifecycle_from_pci_ids};
