// SPDX-License-Identifier: AGPL-3.0-or-later
//! WarmKeepalive — unified lifecycle for GPU warm state preservation.
//!
//! Wraps `VfioAnchor` (fd holder) + clutch engagement + systemd fd store
//! into a single facade. The dispatch handler, SIGTERM handler, and startup
//! recovery all operate on `WarmKeepalive` instead of raw anchor/clutch/store
//! primitives.
//!
//! # Lifecycle
//!
//! ```text
//! startup:   WarmKeepalive::recover_from_restart()
//!                ↓  reconstructs anchors from systemd fd store
//! dispatch:  keepalive.engage(bdf)  →  ClutchHandle { bar0, dma }
//!                ↓                        │
//!                ↓                        └──  sovereign.init uses bar0+dma
//!                ↓                        └──  ClutchHandle drops → disengage
//! SIGTERM:   keepalive.store_for_restart()
//!                ↓  sends all fds to systemd via SCM_RIGHTS
//! ```

use std::collections::HashMap;
use std::os::fd::BorrowedFd;
use std::sync::Arc;

use crate::vfio_anchor::{AnchorBackendRef, VfioAnchor};

/// Unified warm keepalive lifecycle for a single GPU.
///
/// Each `WarmKeepalive` owns one `VfioAnchor` (the VFIO fds that keep
/// the GPU from getting bus-reset) and provides methods to engage the
/// clutch (BAR0 + DMA) and store/recover fds across restarts.
#[derive(Debug)]
pub struct WarmKeepalive {
    anchor: VfioAnchor,
}

impl WarmKeepalive {
    /// Wrap an existing `VfioAnchor` in the keepalive lifecycle.
    #[must_use]
    pub fn from_anchor(anchor: VfioAnchor) -> Self {
        Self { anchor }
    }

    /// Claim a new keepalive from raw VFIO fds (iommufd backend).
    #[must_use]
    pub fn claim_iommufd(
        bdf: String,
        device_fd: std::os::fd::OwnedFd,
        iommufd: std::sync::Arc<std::os::fd::OwnedFd>,
        ioas_id: u32,
    ) -> Self {
        Self {
            anchor: VfioAnchor::from_iommufd(bdf, device_fd, iommufd, ioas_id),
        }
    }

    /// Claim a new keepalive from raw VFIO fds (legacy group backend).
    #[must_use]
    pub fn claim_legacy(
        bdf: String,
        device_fd: std::os::fd::OwnedFd,
        container: std::sync::Arc<std::os::fd::OwnedFd>,
        group: std::os::fd::OwnedFd,
    ) -> Self {
        Self {
            anchor: VfioAnchor::from_legacy(bdf, device_fd, container, group),
        }
    }

    /// BDF address of the keepalive-held GPU.
    #[must_use]
    pub fn bdf(&self) -> &str {
        self.anchor.bdf()
    }

    /// Borrow the device fd for clutch engagement.
    #[must_use]
    pub fn device_fd(&self) -> BorrowedFd<'_> {
        self.anchor.device_fd()
    }

    /// Get the backend reference for constructing DMA backends.
    #[must_use]
    pub fn backend_ref(&self) -> AnchorBackendRef {
        self.anchor.backend_arc()
    }

    /// Get IOAS ID (iommufd backend only).
    #[must_use]
    pub fn ioas_id(&self) -> Option<u32> {
        self.anchor.ioas_id()
    }

    /// Borrow the underlying anchor (for fd store operations).
    #[must_use]
    pub fn anchor(&self) -> &VfioAnchor {
        &self.anchor
    }

    /// Consume this keepalive and return the inner anchor.
    #[must_use]
    pub fn into_anchor(self) -> VfioAnchor {
        self.anchor
    }

    /// Leak the anchor fds to prevent bus reset on process exit.
    /// Last-resort fallback when systemd fd store is unavailable.
    pub fn leak(self) {
        self.anchor.leak();
    }
}

/// Non-owning keepalive view — borrows a `VfioAnchor` for clutch operations.
///
/// Used by the dispatch handler to engage the clutch without taking
/// ownership of the anchor (which stays in the store).
pub struct WarmKeepaliveRef<'a> {
    anchor: &'a VfioAnchor,
}

impl WarmKeepalive {
    /// Create a non-owning reference view of an existing anchor.
    #[must_use]
    pub fn from_ref(anchor: &VfioAnchor) -> WarmKeepaliveRef<'_> {
        WarmKeepaliveRef { anchor }
    }
}

impl<'a> WarmKeepaliveRef<'a> {
    /// Borrow the device fd for clutch engagement.
    #[must_use]
    pub fn device_fd(&self) -> BorrowedFd<'_> {
        self.anchor.device_fd()
    }

    /// Get the backend reference for constructing DMA backends.
    #[must_use]
    pub fn backend_ref(&self) -> AnchorBackendRef {
        self.anchor.backend_arc()
    }

    /// Construct a DMA-ready backend from the anchor's backend reference.
    ///
    /// This is the one-liner that replaces the manual match on
    /// `AnchorBackendRef` in the dispatch handler.
    #[must_use]
    pub fn make_dma_backend(&self) -> DmaSpec {
        DmaSpec(self.anchor.backend_arc())
    }
}

/// DMA backend specification extracted from a keepalive.
///
/// The server crate calls `.into_cylinder_dma()` to convert to
/// the cylinder-specific `DmaBackend` type without ember depending
/// on cylinder.
#[derive(Clone)]
pub struct DmaSpec(AnchorBackendRef);

impl DmaSpec {
    /// Get the underlying backend reference.
    #[must_use]
    pub fn backend_ref(&self) -> &AnchorBackendRef {
        &self.0
    }

    /// Decompose into iommufd components, if applicable.
    #[must_use]
    pub fn as_iommufd(&self) -> Option<(Arc<std::os::fd::OwnedFd>, u32)> {
        match &self.0 {
            AnchorBackendRef::Iommufd { iommufd, ioas_id } => Some((Arc::clone(iommufd), *ioas_id)),
            AnchorBackendRef::LegacyGroup { .. } => None,
        }
    }

    /// Decompose into legacy container components, if applicable.
    #[must_use]
    pub fn as_legacy_container(&self) -> Option<Arc<std::os::fd::OwnedFd>> {
        match &self.0 {
            AnchorBackendRef::LegacyGroup { container } => Some(Arc::clone(container)),
            AnchorBackendRef::Iommufd { .. } => None,
        }
    }
}

/// Collection of warm keepalives, keyed by BDF.
///
/// This is the higher-level replacement for `AnchorStore`. The dispatch
/// handler and SIGTERM handler operate on this instead of raw HashMap.
#[derive(Debug, Default)]
pub struct KeepaliveStore {
    inner: HashMap<String, WarmKeepalive>,
}

impl KeepaliveStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a keepalive, keyed by its BDF.
    pub fn insert(&mut self, keepalive: WarmKeepalive) {
        let bdf = keepalive.bdf().to_string();
        self.inner.insert(bdf, keepalive);
    }

    /// Get a keepalive by BDF.
    #[must_use]
    pub fn get(&self, bdf: &str) -> Option<&WarmKeepalive> {
        self.inner.get(bdf)
    }

    /// Number of GPUs currently kept warm.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True if no GPUs are being kept warm.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Iterate over all keepalives.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &WarmKeepalive)> {
        self.inner.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Extract the inner anchor map for fd store operations.
    /// Returns anchors by reference for storing in systemd.
    #[must_use]
    pub fn anchor_map(&self) -> HashMap<String, &VfioAnchor> {
        self.inner
            .iter()
            .map(|(bdf, k)| (bdf.clone(), &k.anchor))
            .collect()
    }

    /// Drain all keepalives, consuming them.
    pub fn drain(&mut self) -> impl Iterator<Item = (String, WarmKeepalive)> + '_ {
        self.inner.drain()
    }

    /// Build a store from recovered anchors (startup path).
    #[must_use]
    pub fn from_anchors(anchors: HashMap<String, VfioAnchor>) -> Self {
        let inner = anchors
            .into_iter()
            .map(|(bdf, anchor)| (bdf, WarmKeepalive::from_anchor(anchor)))
            .collect();
        Self { inner }
    }

    /// Extract all anchors as an owned HashMap (for fd store).
    #[must_use]
    pub fn into_anchor_map(self) -> HashMap<String, VfioAnchor> {
        self.inner
            .into_iter()
            .map(|(bdf, k)| (bdf, k.anchor))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::OwnedFd;
    use std::sync::Arc;

    fn test_fd() -> OwnedFd {
        OwnedFd::from(std::fs::File::open("/dev/null").unwrap())
    }

    #[test]
    fn keepalive_from_anchor_preserves_bdf() {
        let anchor =
            VfioAnchor::from_iommufd("0000:02:00.0".into(), test_fd(), Arc::new(test_fd()), 42);
        let keepalive = WarmKeepalive::from_anchor(anchor);
        assert_eq!(keepalive.bdf(), "0000:02:00.0");
        assert_eq!(keepalive.ioas_id(), Some(42));
    }

    #[test]
    fn keepalive_claim_iommufd() {
        let keepalive =
            WarmKeepalive::claim_iommufd("0000:49:00.0".into(), test_fd(), Arc::new(test_fd()), 7);
        assert_eq!(keepalive.bdf(), "0000:49:00.0");
    }

    #[test]
    fn keepalive_store_insert_get() {
        let mut store = KeepaliveStore::new();
        assert!(store.is_empty());

        let k1 =
            WarmKeepalive::claim_iommufd("0000:02:00.0".into(), test_fd(), Arc::new(test_fd()), 1);
        let k2 =
            WarmKeepalive::claim_iommufd("0000:49:00.0".into(), test_fd(), Arc::new(test_fd()), 2);
        store.insert(k1);
        store.insert(k2);

        assert_eq!(store.len(), 2);
        assert!(store.get("0000:02:00.0").is_some());
        assert!(store.get("0000:49:00.0").is_some());
        assert!(store.get("0000:FF:00.0").is_none());
    }

    #[test]
    fn keepalive_store_from_anchors() {
        let mut anchors = HashMap::new();
        anchors.insert(
            "0000:02:00.0".into(),
            VfioAnchor::from_iommufd("0000:02:00.0".into(), test_fd(), Arc::new(test_fd()), 1),
        );
        let store = KeepaliveStore::from_anchors(anchors);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn keepalive_into_anchor() {
        let k =
            WarmKeepalive::claim_iommufd("0000:02:00.0".into(), test_fd(), Arc::new(test_fd()), 1);
        let anchor = k.into_anchor();
        assert_eq!(anchor.bdf(), "0000:02:00.0");
    }
}
