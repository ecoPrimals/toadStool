// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! # ember — Hardware-Agnostic Device Holder
//!
//! ember is the subsystem inside glowPlug that **holds** exclusive hardware
//! resources across process restarts. Like the glow plug in a diesel engine,
//! ember keeps things warm — it warms, sparks, resurrects, and immortalizes
//! drivers and hardware as needed.
//!
//! ## Responsibilities
//!
//! - **Hold** exclusive resource handles (VFIO fds, USB claims, DRM sessions,
//!   HSM contexts) so the kernel does not tear down hardware when the broker
//!   restarts.
//! - **Persist** opaque per-device metadata (ring state, mailbox snapshots,
//!   firmware versions) so state survives across personality swaps.
//! - **Lend / Reclaim** — hand off an exclusive handle to a consumer, then
//!   take it back when they are done.
//! - **Journal** every lifecycle event (hold, release, swap, lend, reclaim)
//!   as an immutable log for diagnostics and provenance.
//!
//! ## Design
//!
//! ember is hardware-agnostic. GPU, NPU, USB, HSM, CPU, Bluetooth — every
//! hardware class implements [`ResourceHandle`] for its exclusive access
//! pattern. ember does not know what bus it is holding; it knows only that
//! it owns a handle, the handle has an identity, and it must keep it alive.
//!
//! ## Absorbed from coralReef (Wave 8 Phase A)
//!
//! The vendor lifecycle module, observation types, ring metadata, sysfs helpers,
//! and error types were absorbed from coralReef's `coral-ember` crate as part
//! of the Compute Trio Wave 8 sprint. These encode hardware-specific knowledge
//! about safe driver transitions across NVIDIA, AMD, Intel, and BrainChip devices.

pub mod error;
pub mod held_resource;
pub mod journal;
pub mod lend_reclaim;
pub mod metadata;
pub mod observation;
pub mod plx_keepalive;
pub mod portable_handle;
pub mod pri_ring_anchor;
pub mod resource_handle;
pub mod ring_meta;
pub mod sysfs;
pub mod vendor_lifecycle;

#[cfg(target_os = "linux")]
pub mod vfio_anchor;
#[cfg(target_os = "linux")]
pub mod vfio_handle;
#[cfg(target_os = "linux")]
pub mod warm_keepalive;

pub use error::{SwapError, SysfsError};
pub use held_resource::HeldResource;
pub use journal::{JournalEntry, SwapJournal};
pub use lend_reclaim::{LendReceipt, LendState};
pub use metadata::MetadataStore;
pub use observation::{HealthResult, ResetObservation, SwapObservation, SwapTiming, epoch_ms};
pub use plx_keepalive::{
    ActivityTracker, KeepaliveHandle, PLX_VENDOR_ID, PcieBridgeKeepalive, PlxKeepalive,
    detect_pcie_bridges, detect_plx_bridge, is_pci_bdf,
};
pub use portable_handle::{GpuBackend, PortableHandleError, PortableResourceHandle};
pub use pri_ring_anchor::{
    BootServiceEvidence as EmberBootServiceEvidence, PriRingAnchor, PriRingHealth,
};
pub use resource_handle::ResourceHandle;
pub use ring_meta::{MailboxMeta, RingMeta, RingMetaEntry};
pub use vendor_lifecycle::{
    RebindStrategy, ResetMethod, VendorLifecycle, detect_lifecycle, detect_lifecycle_for_target,
};
#[cfg(target_os = "linux")]
pub use vfio_anchor::{AnchorBackendRef, VfioAnchor};
#[cfg(target_os = "linux")]
pub use vfio_handle::{VfioHandleError, VfioResourceHandle};
#[cfg(target_os = "linux")]
pub use warm_keepalive::{DmaSpec, KeepaliveStore, WarmKeepalive, WarmKeepaliveRef};
