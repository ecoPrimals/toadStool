// SPDX-License-Identifier: AGPL-3.0-or-later

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Debug)]
    struct MockHandle {
        alive: AtomicBool,
        reacquire_succeeds: bool,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("mock handle error")]
    struct MockError;

    impl MockHandle {
        fn new() -> Self {
            Self {
                alive: AtomicBool::new(true),
                reacquire_succeeds: true,
            }
        }

        fn always_fail_reacquire() -> Self {
            Self {
                alive: AtomicBool::new(true),
                reacquire_succeeds: false,
            }
        }
    }

    impl ResourceHandle for MockHandle {
        type Error = MockError;

        fn handle_type(&self) -> &'static str {
            "mock"
        }

        fn is_alive(&self) -> bool {
            self.alive.load(Ordering::Relaxed)
        }

        fn release(&mut self) -> Result<(), Self::Error> {
            self.alive.store(false, Ordering::Relaxed);
            Ok(())
        }

        fn reacquire(&mut self) -> Result<bool, Self::Error> {
            if self.reacquire_succeeds {
                self.alive.store(true, Ordering::Relaxed);
                Ok(true)
            } else {
                Ok(false)
            }
        }
    }

    #[test]
    fn new_resource_starts_alive_with_zero_release_count() {
        let held = HeldResource::new(MockHandle::new());
        assert!(held.is_alive());
        assert_eq!(held.release_count(), 0);
        assert!(held.metadata().is_empty());
    }

    #[test]
    fn with_metadata_preserves_store() {
        let mut meta = MetadataStore::new();
        meta.set("key", serde_json::json!("val"));
        let held = HeldResource::with_metadata(MockHandle::new(), meta);
        assert_eq!(held.metadata().len(), 1);
        assert_eq!(held.metadata().get("key").unwrap(), "val");
    }

    #[test]
    fn release_increments_count_and_marks_dead() {
        let mut held = HeldResource::new(MockHandle::new());
        held.release().expect("release");
        assert!(!held.is_alive());
        assert_eq!(held.release_count(), 1);

        held.release().expect("idempotent release");
        assert_eq!(held.release_count(), 2);
    }

    #[test]
    fn reacquire_resets_held_since_on_success() {
        let mut held = HeldResource::new(MockHandle::new());
        held.release().expect("release");
        let ok = held.reacquire().expect("reacquire");
        assert!(ok);
        assert!(held.is_alive());
        // Timer was just reset — duration must be near-zero.
        assert!(held.held_duration() < std::time::Duration::from_millis(100));
    }

    #[test]
    fn reacquire_returns_false_without_resetting_timer() {
        let mut held = HeldResource::new(MockHandle::always_fail_reacquire());
        let ok = held.reacquire().expect("reacquire call");
        assert!(!ok);
    }

    #[test]
    fn handle_ref_and_mut() {
        let mut held = HeldResource::new(MockHandle::new());
        assert_eq!(held.handle().handle_type(), "mock");
        assert!(held.handle_mut().is_alive());
    }

    #[test]
    fn metadata_mut_allows_writes() {
        let mut held = HeldResource::new(MockHandle::new());
        held.metadata_mut().set("x", serde_json::json!(42));
        assert_eq!(held.metadata().len(), 1);
    }

    #[test]
    fn into_parts_decomposes() {
        let mut held = HeldResource::new(MockHandle::new());
        held.metadata_mut().set("a", serde_json::json!(1));
        let (handle, meta) = held.into_parts();
        assert!(handle.is_alive());
        assert_eq!(meta.len(), 1);
    }

    #[test]
    fn held_duration_is_non_negative() {
        let held = HeldResource::new(MockHandle::new());
        assert!(held.held_duration() < std::time::Duration::from_secs(5));
    }
}
