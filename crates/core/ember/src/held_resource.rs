// SPDX-License-Identifier: AGPL-3.0-only

//! A held hardware resource — handle + metadata + event channel.
//!
//! [`HeldResource`] is the core type that ember manages. It pairs an
//! exclusive [`ResourceHandle`] with persistent [`MetadataStore`] and
//! an optional event channel for kernel notifications (e.g. VFIO `REQ_IRQ`,
//! USB disconnect, HSM tamper alert).

use std::time::Instant;

use crate::metadata::MetadataStore;
use crate::resource_handle::ResourceHandle;

/// A held hardware resource with associated metadata and lifecycle tracking.
///
/// Generic over the handle type `H` so that GPU (VFIO fds), USB (claimed
/// interface), NPU (MMIO region), and any future hardware class can use
/// the same holder infrastructure.
#[derive(Debug)]
pub struct HeldResource<H: ResourceHandle> {
    handle: H,
    metadata: MetadataStore,
    held_since: Instant,
    release_count: u64,
}

impl<H: ResourceHandle> HeldResource<H> {
    /// Create a new held resource from an exclusive handle.
    #[must_use]
    pub fn new(handle: H) -> Self {
        Self {
            handle,
            metadata: MetadataStore::new(),
            held_since: Instant::now(),
            release_count: 0,
        }
    }

    /// Create a held resource with pre-existing metadata (e.g. restored from snapshot).
    #[must_use]
    pub fn with_metadata(handle: H, metadata: MetadataStore) -> Self {
        Self {
            handle,
            metadata,
            held_since: Instant::now(),
            release_count: 0,
        }
    }

    /// Reference to the underlying handle.
    #[must_use]
    pub const fn handle(&self) -> &H {
        &self.handle
    }

    /// Mutable reference to the underlying handle.
    pub fn handle_mut(&mut self) -> &mut H {
        &mut self.handle
    }

    /// Reference to the metadata store.
    #[must_use]
    pub const fn metadata(&self) -> &MetadataStore {
        &self.metadata
    }

    /// Mutable reference to the metadata store.
    pub fn metadata_mut(&mut self) -> &mut MetadataStore {
        &mut self.metadata
    }

    /// How long this resource has been held.
    #[must_use]
    pub fn held_duration(&self) -> std::time::Duration {
        self.held_since.elapsed()
    }

    /// How many times this resource has been released and reacquired.
    #[must_use]
    pub const fn release_count(&self) -> u64 {
        self.release_count
    }

    /// Whether the underlying handle is still alive.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.handle.is_alive()
    }

    /// Release the underlying handle.
    ///
    /// # Errors
    ///
    /// Returns the handle's error type if the kernel refuses.
    pub fn release(&mut self) -> Result<(), H::Error> {
        self.handle.release()?;
        self.release_count += 1;
        Ok(())
    }

    /// Attempt to reacquire after a release. Resets the held-since timer on success.
    ///
    /// # Errors
    ///
    /// Returns the handle's error type if reacquisition fails.
    pub fn reacquire(&mut self) -> Result<bool, H::Error> {
        let ok = self.handle.reacquire()?;
        if ok {
            self.held_since = Instant::now();
        }
        Ok(ok)
    }

    /// Consume the held resource, returning the handle and metadata separately.
    #[must_use]
    pub fn into_parts(self) -> (H, MetadataStore) {
        (self.handle, self.metadata)
    }
}
