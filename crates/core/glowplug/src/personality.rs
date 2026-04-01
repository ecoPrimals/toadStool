// SPDX-License-Identifier: AGPL-3.0-only

//! Device personality — what driver/mode a device is operating in.
//!
//! A **personality** describes the current operational mode of a hardware
//! device. For GPUs this might be `vfio`, `nouveau`, or `nvidia`. For USB
//! it might be `host` or `gadget`. For CPUs it might be `performance`,
//! `powersave`, or `isolated`.
//!
//! The [`DevicePersonality`] trait is the abstraction; each hardware class
//! provides its own concrete personality enum.

use std::fmt;

/// A device's driver/mode personality.
///
/// Implementors describe a specific operational mode for a hardware class.
/// The trait is object-safe so that [`DeviceSlot`](super::device_slot::DeviceSlot)
/// can hold `Box<dyn DevicePersonality>`.
pub trait DevicePersonality: Send + Sync + fmt::Debug + fmt::Display {
    /// Short name for this personality (e.g. `"vfio"`, `"host"`, `"performance"`).
    fn name(&self) -> &str;

    /// Whether this personality provides direct/exclusive hardware access
    /// (e.g. VFIO passthrough, raw USB claim).
    fn provides_direct_access(&self) -> bool;

    /// Kernel driver module name, if applicable (e.g. `"vfio-pci"`, `"xhci_hcd"`).
    fn driver_module(&self) -> Option<&str>;

    /// Capability tags this personality exposes (e.g. `["compute", "dma"]`).
    fn capabilities(&self) -> &[&str];
}

/// Registry of known personalities for a hardware class.
///
/// Used by the swap orchestrator to validate target personalities and
/// instantiate the correct personality object from a name string.
pub trait PersonalityRegistry: Send + Sync {
    /// The concrete personality type this registry manages.
    type Personality: DevicePersonality;

    /// List all supported personality names.
    fn supported(&self) -> Vec<&str>;

    /// Create a personality by name. Returns `None` if the name is unknown.
    fn create(&self, name: &str) -> Option<Self::Personality>;

    /// Whether a personality name is supported.
    fn supports(&self, name: &str) -> bool {
        self.supported().contains(&name)
    }
}

/// The "unbound" personality — device has no driver and is not actively managed.
///
/// This is the universal fallback personality for any hardware class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unbound;

impl DevicePersonality for Unbound {
    fn name(&self) -> &'static str {
        "unbound"
    }

    fn provides_direct_access(&self) -> bool {
        false
    }

    fn driver_module(&self) -> Option<&str> {
        None
    }

    fn capabilities(&self) -> &'static [&'static str] {
        &[]
    }
}

impl fmt::Display for Unbound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unbound")
    }
}
