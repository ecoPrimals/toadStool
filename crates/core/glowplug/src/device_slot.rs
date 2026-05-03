// SPDX-License-Identifier: AGPL-3.0-or-later

//! Managed device slot — a device under glowPlug's care.
//!
//! A [`DeviceSlot`] pairs a [`DeviceId`] with its current personality,
//! an ember [`HeldResource`] (if held), health status, and a lifecycle
//! journal.

use toadstool_ember::HeldResource;
use toadstool_ember::journal::SwapJournal;
use toadstool_ember::resource_handle::ResourceHandle;

use crate::device_id::DeviceId;
use crate::health::HealthStatus;

/// A managed device slot — glowPlug's view of one piece of hardware.
///
/// Generic over:
/// - `H`: the [`ResourceHandle`] type (VFIO fds, USB claim, etc.)
/// - `P`: the concrete personality type
#[derive(Debug)]
pub struct DeviceSlot<H: ResourceHandle, P> {
    id: DeviceId,
    personality: P,
    held: Option<HeldResource<H>>,
    health: HealthStatus,
    journal: SwapJournal,
}

impl<H: ResourceHandle, P> DeviceSlot<H, P> {
    /// Create a new device slot with initial personality and no held resource.
    #[must_use]
    pub fn new(id: DeviceId, personality: P) -> Self {
        Self {
            id,
            personality,
            held: None,
            health: HealthStatus::Unknown,
            journal: SwapJournal::new(),
        }
    }

    /// Create a device slot that already holds a resource.
    #[must_use]
    pub fn with_held(id: DeviceId, personality: P, held: HeldResource<H>) -> Self {
        Self {
            id,
            personality,
            held: Some(held),
            health: HealthStatus::Unknown,
            journal: SwapJournal::new(),
        }
    }

    /// The device identity.
    #[must_use]
    pub const fn id(&self) -> &DeviceId {
        &self.id
    }

    /// The current personality.
    #[must_use]
    pub const fn personality(&self) -> &P {
        &self.personality
    }

    /// Set the personality (used during swap).
    pub fn set_personality(&mut self, personality: P) {
        self.personality = personality;
    }

    /// The held resource, if any.
    #[must_use]
    pub const fn held(&self) -> Option<&HeldResource<H>> {
        self.held.as_ref()
    }

    /// Mutable access to the held resource.
    pub fn held_mut(&mut self) -> Option<&mut HeldResource<H>> {
        self.held.as_mut()
    }

    /// Set the held resource.
    pub fn set_held(&mut self, held: HeldResource<H>) {
        self.held = Some(held);
    }

    /// Take the held resource out of the slot (e.g. for lend).
    pub fn take_held(&mut self) -> Option<HeldResource<H>> {
        self.held.take()
    }

    /// Current health status.
    #[must_use]
    pub const fn health(&self) -> &HealthStatus {
        &self.health
    }

    /// Update health status.
    pub fn set_health(&mut self, status: HealthStatus) {
        self.health = status;
    }

    /// The lifecycle journal.
    #[must_use]
    pub const fn journal(&self) -> &SwapJournal {
        &self.journal
    }

    /// Mutable access to the journal.
    pub fn journal_mut(&mut self) -> &mut SwapJournal {
        &mut self.journal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthStatus;
    use crate::personality::Unbound;
    use std::sync::atomic::{AtomicBool, Ordering};
    use toadstool_ember::journal::JournalEvent;

    #[derive(Debug)]
    struct StubHandle(AtomicBool);

    #[derive(Debug, thiserror::Error)]
    #[error("stub")]
    struct StubErr;

    impl toadstool_ember::ResourceHandle for StubHandle {
        type Error = StubErr;
        fn handle_type(&self) -> &'static str {
            "stub"
        }
        fn is_alive(&self) -> bool {
            self.0.load(Ordering::Relaxed)
        }
        fn release(&mut self) -> Result<(), Self::Error> {
            self.0.store(false, Ordering::Relaxed);
            Ok(())
        }
        fn reacquire(&mut self) -> Result<bool, Self::Error> {
            self.0.store(true, Ordering::Relaxed);
            Ok(true)
        }
    }

    fn stub_held() -> HeldResource<StubHandle> {
        HeldResource::new(StubHandle(AtomicBool::new(true)))
    }

    #[test]
    fn new_slot_has_no_held_resource() {
        let slot: DeviceSlot<StubHandle, Unbound> =
            DeviceSlot::new(DeviceId::Platform("test".into()), Unbound);
        assert!(slot.held().is_none());
        assert_eq!(slot.health(), &HealthStatus::Unknown);
        assert!(slot.journal().is_empty());
    }

    #[test]
    fn with_held_stores_resource() {
        let slot = DeviceSlot::with_held(
            DeviceId::PciBdf("0000:01:00.0".into()),
            Unbound,
            stub_held(),
        );
        assert!(slot.held().is_some());
        assert!(slot.held().unwrap().is_alive());
    }

    #[test]
    fn set_and_take_held() {
        let mut slot: DeviceSlot<StubHandle, Unbound> =
            DeviceSlot::new(DeviceId::Serial("SN1".into()), Unbound);
        assert!(slot.held().is_none());

        slot.set_held(stub_held());
        assert!(slot.held().is_some());

        let taken = slot.take_held();
        assert!(taken.is_some());
        assert!(slot.held().is_none());
    }

    #[test]
    fn set_personality() {
        let mut slot: DeviceSlot<StubHandle, String> =
            DeviceSlot::new(DeviceId::UsbPath("1-2".into()), "host".to_string());
        assert_eq!(slot.personality(), "host");

        slot.set_personality("gadget".to_string());
        assert_eq!(slot.personality(), "gadget");
    }

    #[test]
    fn set_health() {
        let mut slot: DeviceSlot<StubHandle, Unbound> =
            DeviceSlot::new(DeviceId::Platform("x".into()), Unbound);
        slot.set_health(HealthStatus::Healthy);
        assert_eq!(slot.health(), &HealthStatus::Healthy);
        assert!(slot.health().is_usable());
    }

    #[test]
    fn journal_records_events() {
        let mut slot: DeviceSlot<StubHandle, Unbound> =
            DeviceSlot::new(DeviceId::Platform("j".into()), Unbound);
        slot.journal_mut()
            .record(JournalEvent::Acquired, Some("test".into()));
        assert_eq!(slot.journal().len(), 1);
    }

    #[test]
    fn id_accessor() {
        let id = DeviceId::PciBdf("0000:02:00.0".into());
        let slot: DeviceSlot<StubHandle, Unbound> = DeviceSlot::new(id.clone(), Unbound);
        assert_eq!(slot.id(), &id);
    }

    #[test]
    fn held_mut_allows_release() {
        let mut slot = DeviceSlot::with_held(DeviceId::Platform("m".into()), Unbound, stub_held());
        slot.held_mut().unwrap().release().expect("release");
        assert!(!slot.held().unwrap().is_alive());
    }
}
