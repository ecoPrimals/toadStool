// SPDX-License-Identifier: AGPL-3.0-only

//! Managed device slot — a device under glowPlug's care.
//!
//! A [`DeviceSlot`] pairs a [`DeviceId`] with its current personality,
//! an ember [`HeldResource`] (if held), health status, and a lifecycle
//! journal.

use toadstool_ember::journal::SwapJournal;
use toadstool_ember::resource_handle::ResourceHandle;
use toadstool_ember::HeldResource;

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
